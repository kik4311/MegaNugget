use device_manager::{checked, valid_udid, Failure, Runner};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashSet,
    fs::File,
    io::{Read, Seek, SeekFrom},
    path::Path,
};
use tempfile::NamedTempFile;
use zip::ZipArchive;

const MAX_IPA: u64 = 256 * 1024 * 1024;
const MAX_EXPANDED: u64 = 1024 * 1024 * 1024;

#[derive(Debug, Serialize)]
pub struct IpaInfo {
    pub bundle_id: String,
    pub version: String,
    pub minimum_ios: String,
    pub signing: &'static str,
}

pub struct ValidatedIpa {
    pub info: IpaInfo,
    snapshot: NamedTempFile,
}

impl ValidatedIpa {
    /// Explicit user-selected export; includes a private provisioning profile.
    pub fn save(&self, destination: &Path) -> Result<(), InstallError> {
        std::fs::copy(self.snapshot.path(), destination).map_err(|_| InstallError::Io)?;
        Ok(())
    }
}

/// An unsigned candidate cannot be passed to install(). Only the signing path accepts it.
pub struct SigningInput {
    pub info: IpaInfo,
    snapshot: NamedTempFile,
}

impl SigningInput {
    /// Extract only into a fresh, private directory owned by the caller.
    pub fn extract(&self, destination: &Path) -> Result<std::path::PathBuf, InstallError> {
        let mut zip =
            ZipArchive::new(self.snapshot.as_file()).map_err(|_| InstallError::InvalidIpa)?;
        let mut app_root = None;
        let mut actual_total = 0u64;
        for i in 0..zip.len() {
            let mut entry = zip.by_index(i).map_err(|_| InstallError::InvalidIpa)?;
            let enclosed = entry.enclosed_name().ok_or(InstallError::UnsafeArchive)?;
            if !enclosed.starts_with("Payload") {
                continue;
            }
            let parts: Vec<_> = enclosed.components().collect();
            if parts.len() >= 2 && parts[1].as_os_str().to_string_lossy().ends_with(".app") {
                app_root = Some(destination.join("Payload").join(parts[1].as_os_str()));
            }
            let path = destination.join(enclosed);
            if entry.is_dir() {
                std::fs::create_dir_all(&path).map_err(|_| InstallError::Io)?;
                continue;
            }
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent).map_err(|_| InstallError::Io)?;
            }
            let mut output = std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&path)
                .map_err(|_| InstallError::Io)?;
            let declared = entry.size();
            let size = std::io::copy(&mut entry.by_ref().take(declared + 1), &mut output)
                .map_err(|_| InstallError::InvalidIpa)?;
            actual_total = actual_total
                .checked_add(size)
                .ok_or(InstallError::SizeLimit)?;
            if size != declared || actual_total > MAX_EXPANDED {
                return Err(InstallError::SizeLimit);
            }
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                output
                    .set_permissions(std::fs::Permissions::from_mode(0o700))
                    .map_err(|_| InstallError::Io)?;
            }
        }
        app_root.ok_or(InstallError::InvalidIpa)
    }
}

pub fn inspect_for_signing(path: &Path) -> Result<SigningInput, InstallError> {
    let snapshot = snapshot_ipa(path)?;
    let info = validate(snapshot.as_file(), false)?;
    // Initial signing scope: our own simple app only, never special-app injection paths.
    if info.bundle_id != "org.meganugget.ios" {
        return Err(InstallError::UnsupportedSigningBundle);
    }
    let zip = ZipArchive::new(snapshot.as_file()).map_err(|_| InstallError::InvalidIpa)?;
    if zip
        .file_names()
        .any(|n| n.contains("/Frameworks/") || n.contains("/PlugIns/") || n.ends_with(".dylib"))
    {
        return Err(InstallError::SigningContentUnsupported);
    }
    Ok(SigningInput { info, snapshot })
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum InstallError {
    InvalidIpa,
    UnsafeArchive,
    SizeLimit,
    MissingSigningArtifacts,
    UnsupportedSigningBundle,
    SigningContentUnsupported,
    Io,
    InvalidDevice,
    UnsupportedTool,
    Tool(Failure),
    VerificationFailed,
}

impl InstallError {
    /// Stable snake_case codes for the UI; the frontend maps them to localized hints.
    pub const fn code(&self) -> &'static str {
        match self {
            Self::InvalidIpa => "invalid_ipa",
            Self::UnsafeArchive => "unsafe_archive",
            Self::SizeLimit => "size_limit",
            Self::MissingSigningArtifacts => "missing_signing_artifacts",
            Self::UnsupportedSigningBundle => "unsupported_signing_bundle",
            Self::SigningContentUnsupported => "signing_content_unsupported",
            Self::Io => "io",
            Self::InvalidDevice => "invalid_device",
            Self::UnsupportedTool => "unsupported_tool",
            Self::Tool(_) => "tool_failure",
            Self::VerificationFailed => "verification_failed",
        }
    }
}

#[derive(Deserialize)]
struct AppPlist {
    #[serde(rename = "CFBundleIdentifier")]
    bundle_id: String,
    #[serde(rename = "CFBundleVersion")]
    version: String,
    #[serde(rename = "CFBundleExecutable")]
    executable: String,
    #[serde(rename = "CFBundlePackageType")]
    package_type: String,
    #[serde(rename = "MinimumOSVersion")]
    minimum_ios: String,
    #[serde(rename = "CFBundleSupportedPlatforms")]
    platforms: Vec<String>,
}

/// Snapshot first: the source path cannot be swapped between validation and install.
/// No archive members are extracted. Signing artifacts are not signature validation.
pub fn inspect(path: &Path) -> Result<ValidatedIpa, InstallError> {
    let snapshot = snapshot_ipa(path)?;
    let info = validate_archive(snapshot.as_file())?;
    Ok(ValidatedIpa { info, snapshot })
}

fn snapshot_ipa(path: &Path) -> Result<NamedTempFile, InstallError> {
    if !path
        .extension()
        .is_some_and(|x| x.eq_ignore_ascii_case("ipa"))
    {
        return Err(InstallError::InvalidIpa);
    }
    let input = File::open(path).map_err(|_| InstallError::Io)?;
    if !input.metadata().map_err(|_| InstallError::Io)?.is_file() {
        return Err(InstallError::InvalidIpa);
    }
    let mut snapshot = tempfile::Builder::new()
        .suffix(".ipa")
        .tempfile()
        .map_err(|_| InstallError::Io)?;
    let size =
        std::io::copy(&mut input.take(MAX_IPA + 1), &mut snapshot).map_err(|_| InstallError::Io)?;
    if size > MAX_IPA {
        return Err(InstallError::SizeLimit);
    }
    snapshot
        .seek(SeekFrom::Start(0))
        .map_err(|_| InstallError::Io)?;
    Ok(snapshot)
}

fn validate_archive(file: &File) -> Result<IpaInfo, InstallError> {
    validate(file, true)
}

fn validate(file: &File, require_signing: bool) -> Result<IpaInfo, InstallError> {
    let mut zip = ZipArchive::new(file).map_err(|_| InstallError::InvalidIpa)?;
    if zip.len() > 20_000 {
        return Err(InstallError::SizeLimit);
    }
    let mut names = HashSet::new();
    let mut roots = HashSet::new();
    let mut expanded = 0u64;
    for i in 0..zip.len() {
        let entry = zip.by_index(i).map_err(|_| InstallError::InvalidIpa)?;
        let name = entry.name();
        if entry.enclosed_name().is_none()
            || name.contains('\\')
            || name.contains(':')
            || name.split('/').any(|c| c == ".." || c == ".")
            || entry.unix_mode().is_some_and(|m| m & 0o170000 == 0o120000)
            || !names.insert(name.to_owned())
        {
            return Err(InstallError::UnsafeArchive);
        }
        expanded = expanded
            .checked_add(entry.size())
            .ok_or(InstallError::SizeLimit)?;
        if expanded > MAX_EXPANDED {
            return Err(InstallError::SizeLimit);
        }
        let parts: Vec<_> = name.split('/').collect();
        if parts.len() >= 3 && parts[0] == "Payload" && parts[1].ends_with(".app") {
            roots.insert(format!("Payload/{}/", parts[1]));
        }
    }
    if roots.len() != 1 {
        return Err(InstallError::InvalidIpa);
    }
    let root = roots.into_iter().next().ok_or(InstallError::InvalidIpa)?;
    let plist_bytes = {
        let mut entry = zip
            .by_name(&format!("{root}Info.plist"))
            .map_err(|_| InstallError::InvalidIpa)?;
        if entry.size() > 1024 * 1024 {
            return Err(InstallError::SizeLimit);
        }
        let mut bytes = vec![];
        entry
            .by_ref()
            .take(1024 * 1024 + 1)
            .read_to_end(&mut bytes)
            .map_err(|_| InstallError::InvalidIpa)?;
        if bytes.len() > 1024 * 1024 {
            return Err(InstallError::SizeLimit);
        }
        bytes
    };
    let app: AppPlist = plist::from_bytes(&plist_bytes).map_err(|_| InstallError::InvalidIpa)?;
    if app.bundle_id.len() > 255
        || !app.bundle_id.contains('.')
        || !app
            .bundle_id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'-'))
        || app.executable.is_empty()
        || app.executable.contains(['/', '\\'])
        || app.executable == "."
        || app.executable == ".."
        || app.package_type != "APPL"
        || !app.platforms.iter().any(|p| p == "iPhoneOS")
        || app.version.is_empty()
        || app.version.len() > 100
        || app.minimum_ios.len() > 32
    {
        return Err(InstallError::InvalidIpa);
    }
    let signing_artifacts = ["_CodeSignature/CodeResources", "embedded.mobileprovision"]
        .iter()
        .all(|name| {
            zip.by_name(&format!("{root}{name}"))
                .is_ok_and(|e| e.size() > 0)
        });
    if !zip
        .by_name(&format!("{root}{}", app.executable))
        .is_ok_and(|e| e.size() > 0)
    {
        return Err(InstallError::MissingSigningArtifacts);
    }
    if require_signing && !signing_artifacts {
        return Err(InstallError::MissingSigningArtifacts);
    }
    Ok(IpaInfo {
        bundle_id: app.bundle_id,
        version: app.version,
        minimum_ios: app.minimum_ios,
        signing: if signing_artifacts {
            "artifacts_present_not_verified"
        } else {
            "requires_signing"
        },
    })
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum InstallerCli {
    Legacy,
    Modern,
}

pub fn installer_cli(runner: &impl Runner) -> Result<InstallerCli, InstallError> {
    let help = checked(runner, "ideviceinstaller", &["--help"], 10).map_err(InstallError::Tool)?;
    let help = String::from_utf8_lossy(&help);
    if help.contains("--install") && help.contains("--list-apps") {
        Ok(InstallerCli::Legacy)
    } else if help.contains("install PATH") && help.contains("--xml") {
        Ok(InstallerCli::Modern)
    } else {
        Err(InstallError::UnsupportedTool)
    }
}

#[derive(Deserialize)]
struct InstalledApp {
    #[serde(rename = "CFBundleIdentifier")]
    id: String,
    #[serde(rename = "CFBundleVersion", default)]
    version: String,
}

pub fn install(runner: &impl Runner, ipa: &ValidatedIpa, id: &str) -> Result<(), InstallError> {
    if !valid_udid(id) {
        return Err(InstallError::InvalidDevice);
    }
    device_manager::device_info(runner, id).map_err(InstallError::Tool)?;
    let cli = installer_cli(runner)?;
    let path = ipa.snapshot.path().to_str().ok_or(InstallError::Io)?;
    let args = match cli {
        InstallerCli::Legacy => vec!["-u", id, "-i", path],
        InstallerCli::Modern => vec!["-u", id, "install", path],
    };
    checked(runner, "ideviceinstaller", &args, 180).map_err(InstallError::Tool)?;
    let args = match cli {
        InstallerCli::Legacy => vec!["-u", id, "-l", "-o", "xml"],
        InstallerCli::Modern => vec!["-u", id, "list", "--xml"],
    };
    let bytes = checked(runner, "ideviceinstaller", &args, 30)
        .map_err(|_| InstallError::VerificationFailed)?;
    let apps: Vec<InstalledApp> =
        plist::from_bytes(&bytes).map_err(|_| InstallError::VerificationFailed)?;
    if apps
        .iter()
        .any(|a| a.id == ipa.info.bundle_id && a.version == ipa.info.version)
    {
        Ok(())
    } else {
        Err(InstallError::VerificationFailed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    fn fixture(extra: Option<&str>, signed: bool) -> NamedTempFile {
        let f = tempfile::Builder::new().suffix(".ipa").tempfile().unwrap();
        let mut zip = zip::ZipWriter::new(f.reopen().unwrap());
        let options = zip::write::SimpleFileOptions::default();
        zip.start_file("Payload/MegaNugget.app/Info.plist", options)
            .unwrap();
        zip.write_all(br#"<?xml version="1.0"?><plist version="1.0"><dict><key>CFBundleIdentifier</key><string>org.meganugget.ios</string><key>CFBundleVersion</key><string>1</string><key>CFBundleExecutable</key><string>MegaNugget</string><key>MinimumOSVersion</key><string>15.0</string><key>CFBundlePackageType</key><string>APPL</string><key>CFBundleSupportedPlatforms</key><array><string>iPhoneOS</string></array></dict></plist>"#).unwrap();
        let mut paths = vec!["Payload/MegaNugget.app/MegaNugget"];
        if signed {
            paths.extend([
                "Payload/MegaNugget.app/_CodeSignature/CodeResources",
                "Payload/MegaNugget.app/embedded.mobileprovision",
            ]);
        }
        if let Some(p) = extra {
            paths.push(p);
        }
        for p in paths {
            zip.start_file(p, options).unwrap();
            zip.write_all(b"test-not-real-signature").unwrap();
        }
        zip.finish().unwrap();
        f
    }
    #[test]
    fn artifacts_are_not_trusted_signatures() {
        let f = fixture(None, true);
        let ipa = inspect(f.path()).unwrap();
        assert_eq!(ipa.info.signing, "artifacts_present_not_verified");
    }
    #[test]
    fn unsigned_rejected() {
        assert!(matches!(
            inspect(fixture(None, false).path()),
            Err(InstallError::MissingSigningArtifacts)
        ));
    }
    #[test]
    fn unsigned_can_only_enter_signing_path() {
        let file = fixture(None, false);
        assert!(inspect(file.path()).is_err());
        let input = inspect_for_signing(file.path()).unwrap();
        assert_eq!(input.info.signing, "requires_signing");
        let destination = tempfile::tempdir().unwrap();
        let app = input.extract(destination.path()).unwrap();
        assert!(app.join("MegaNugget").is_file());
        assert!(app.join("Info.plist").is_file());
        assert!(!app.join("embedded.mobileprovision").exists());
    }

    #[test]
    fn signing_rejects_extensions_and_unsafe_paths() {
        for extra in [
            "Payload/MegaNugget.app/PlugIns/Extension.appex/file",
            "Payload/MegaNugget.app/Frameworks/Other.framework/file",
            "Payload/MegaNugget.app/Injected.dylib",
            "../bad",
        ] {
            let file = fixture(Some(extra), false);
            assert!(inspect_for_signing(file.path()).is_err());
        }
        assert!(matches!(
            inspect_for_signing(
                fixture(
                    Some("Payload/MegaNugget.app/Frameworks/X.framework/b"),
                    false
                )
                .path()
            ),
            Err(InstallError::SigningContentUnsupported)
        ));
    }

    #[test]
    fn error_codes_are_stable_snake_case() {
        assert_eq!(
            InstallError::UnsupportedSigningBundle.code(),
            "unsupported_signing_bundle"
        );
        assert_eq!(
            InstallError::SigningContentUnsupported.code(),
            "signing_content_unsupported"
        );
        assert_eq!(
            InstallError::MissingSigningArtifacts.code(),
            "missing_signing_artifacts"
        );
        assert_eq!(
            InstallError::Tool(Failure::CommandFailed).code(),
            "tool_failure"
        );
    }

    #[test]
    fn traversal_rejected() {
        assert!(matches!(
            inspect(fixture(Some("../bad"), true).path()),
            Err(InstallError::UnsafeArchive)
        ));
    }
    #[test]
    fn multiple_apps_rejected() {
        assert!(matches!(
            inspect(fixture(Some("Payload/Other.app/Info.plist"), true).path()),
            Err(InstallError::InvalidIpa)
        ));
    }
    #[test]
    fn snapshot_survives_source_change() {
        let f = fixture(None, true);
        let ipa = inspect(f.path()).unwrap();
        std::fs::write(f.path(), b"replaced").unwrap();
        assert!(validate_archive(ipa.snapshot.as_file()).is_ok());
    }
    struct Fake {
        modern: bool,
        verify: bool,
    }
    impl Runner for Fake {
        fn run(
            &self,
            tool: &str,
            args: &[&str],
            _: std::time::Duration,
        ) -> Result<device_manager::Output, Failure> {
            let data = if tool == "ideviceinfo" {
                "<plist version=\"1.0\"><dict><key>ProductType</key><string>iPhone9,1</string><key>ProductVersion</key><string>15.8.8</string><key>BuildVersion</key><string>test</string></dict></plist>"
            } else if args == ["--help"] {
                if self.modern {
                    "install PATH --xml"
                } else {
                    "--install --list-apps"
                }
            } else if args.contains(&"list") || args.contains(&"-l") {
                if self.verify {
                    "<plist version=\"1.0\"><array><dict><key>CFBundleIdentifier</key><string>org.meganugget.ios</string><key>CFBundleVersion</key><string>1</string></dict></array></plist>"
                } else {
                    "<plist version=\"1.0\"><array/></plist>"
                }
            } else {
                ""
            };
            Ok(device_manager::Output {
                success: true,
                stdout: data.as_bytes().to_vec(),
            })
        }
    }
    #[test]
    fn both_cli_dialects_verify() {
        let ipa = inspect(fixture(None, true).path()).unwrap();
        for modern in [true, false] {
            assert!(install(
                &Fake {
                    modern,
                    verify: true
                },
                &ipa,
                "0123456789012345678901234567890123456789"
            )
            .is_ok());
            assert!(matches!(
                install(
                    &Fake {
                        modern,
                        verify: false
                    },
                    &ipa,
                    "0123456789012345678901234567890123456789"
                ),
                Err(InstallError::VerificationFailed)
            ));
        }
    }
}

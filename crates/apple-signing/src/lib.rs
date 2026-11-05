use app_installer::{SigningInput, ValidatedIpa};
use isideload::{
    anisette::remote_v3::{DEFAULT_ANISETTE_V3_URL, RemoteV3AnisetteProvider},
    auth::apple_account::{AppleAccount, TwoFactorCallbackResponse},
    dev::{developer_session::DeveloperSession, devices::DevicesApi, teams::TeamsApi},
    sideload::{SideloaderBuilder, builder::MaxCertsBehavior},
    util::{
        callbacks::{ExtensionsBehaviorCallbackBox, MaxCertsCallbackBox},
        keyring_storage::KeyringStorage,
    },
};
use serde::Serialize;
use std::{
    path::Path,
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::sync::oneshot;
use zeroize::Zeroizing;

pub const ANISETTE_URL: &str = DEFAULT_ANISETTE_V3_URL;

#[derive(Clone, Serialize, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Phase {
    Idle,
    Authenticating,
    TwoFactor,
    SelectTeam,
    Provisioning,
    Signing,
    Signed,
    Failed,
    Cancelled,
}

#[derive(Clone, Serialize)]
pub struct Team {
    pub id: String,
    pub name: String,
}

#[derive(Clone, Serialize)]
pub struct Status {
    pub phase: Phase,
    pub teams: Vec<Team>,
    pub error: Option<&'static str>,
}

pub struct Session {
    status: Status,
    response: Option<oneshot::Sender<String>>,
}
impl Default for Session {
    fn default() -> Self {
        Self {
            status: Status {
                phase: Phase::Idle,
                teams: vec![],
                error: None,
            },
            response: None,
        }
    }
}
pub type SharedSession = Arc<Mutex<Session>>;
impl Session {
    pub fn status(&self) -> Status {
        self.status.clone()
    }
    pub fn set_phase(&mut self, phase: Phase) {
        self.status.phase = phase;
        self.status.teams.clear();
        self.response = None;
    }
    pub fn fail(&mut self, error: &'static str) {
        self.set_phase(Phase::Failed);
        self.status.error = Some(error);
    }
    pub fn respond(&mut self, value: String) -> Result<(), &'static str> {
        if self.status.phase == Phase::TwoFactor
            && (value.len() != 6 || !value.bytes().all(|b| b.is_ascii_digit()))
        {
            return Err("invalid_2fa_code");
        }
        if self.status.phase == Phase::SelectTeam
            && !self.status.teams.iter().any(|t| t.id == value)
        {
            return Err("invalid_team");
        }
        let sender = self.response.take().ok_or("no_pending_challenge")?;
        sender.send(value).map_err(|_| "challenge_expired")
    }
}

async fn challenge(
    session: &SharedSession,
    phase: Phase,
    teams: Vec<Team>,
) -> Result<String, &'static str> {
    let (tx, rx) = oneshot::channel();
    {
        let mut state = session.lock().map_err(|_| "state_unavailable")?;
        state.set_phase(phase);
        state.status.teams = teams;
        state.response = Some(tx);
    }
    let response = tokio::time::timeout(Duration::from_secs(180), rx)
        .await
        .map_err(|_| "challenge_timeout")?
        .map_err(|_| "challenge_cancelled")?;
    session
        .lock()
        .map_err(|_| "state_unavailable")?
        .set_phase(Phase::Authenticating);
    Ok(response)
}

pub fn version_allowed(minimum: &str, installed: &str) -> bool {
    fn parse(s: &str) -> Option<[u32; 3]> {
        let parts: Vec<_> = s.split('.').collect();
        if parts.is_empty() || parts.len() > 3 {
            return None;
        }
        let mut result = [0; 3];
        for (i, part) in parts.iter().enumerate() {
            if part.is_empty() || !part.bytes().all(|c| c.is_ascii_digit()) {
                return None;
            }
            result[i] = part.parse().ok()?;
        }
        Some(result)
    }
    match (parse(minimum), parse(installed)) {
        (Some(a), Some(b)) => a <= b,
        _ => false,
    }
}

/// Real Apple authentication/provisioning/signing. No automatic certificate revocation.
/// Call only after consent to account mutations and the disclosed anisette service.
pub async fn sign(
    input: SigningInput,
    udid: String,
    ios: String,
    email: String,
    password: Zeroizing<String>,
    session: SharedSession,
) -> Result<ValidatedIpa, &'static str> {
    if !device_manager::valid_udid(&udid) {
        return Err("invalid_device");
    }
    if !version_allowed(&input.info.minimum_ios, &ios) {
        return Err("minimum_ios_not_met");
    }
    // Private extraction directory; never let upstream extract an untrusted ZIP itself.
    let workspace = tempfile::Builder::new()
        .prefix("meganugget-sign-")
        .tempdir()
        .map_err(|_| "temporary_storage_failed")?;
    let app_path = input
        .extract(workspace.path())
        .map_err(|_| "ipa_extraction_failed")?;
    let _ = rustls::crypto::ring::default_provider().install_default();
    static INIT: std::sync::OnceLock<Result<(), ()>> = std::sync::OnceLock::new();
    INIT.get_or_init(|| isideload::init().map_err(|_| ()))
        .as_ref()
        .map_err(|_| "signer_initialization_failed")?;
    let anisette = RemoteV3AnisetteProvider::new(
        ANISETTE_URL,
        Box::new(KeyringStorage::new(
            "org.meganugget.signing.anisette".into(),
        )),
        "0".into(),
    )
    .map_err(|_| "anisette_initialization_failed")?;
    session
        .lock()
        .map_err(|_| "state_unavailable")?
        .set_phase(Phase::Authenticating);
    let auth_session = session.clone();
    let mut account = AppleAccount::builder(&email)
        .anisette_provider(anisette)
        .login(password.as_str(), move |_params| {
            let state = auth_session.clone();
            async move {
                let code = challenge(&state, Phase::TwoFactor, vec![])
                    .await
                    .map_err(|_| rootcause::report!("authentication challenge cancelled"))?;
                Ok(TwoFactorCallbackResponse::SubmitCode(code))
            }
        })
        .await
        .map_err(|_| "apple_authentication_failed")?;
    drop(password);
    let mut dev = DeveloperSession::from_account(&mut account)
        .await
        .map_err(|_| "developer_session_failed")?;
    let teams = dev.list_teams().await.map_err(|_| "team_list_failed")?;
    let team = if teams.len() == 1 {
        teams[0].clone()
    } else {
        if teams.is_empty() {
            return Err("no_developer_teams");
        }
        let selected = challenge(
            &session,
            Phase::SelectTeam,
            teams
                .iter()
                .map(|t| Team {
                    id: t.team_id.clone(),
                    name: t.name.clone().unwrap_or_else(|| "Apple Team".into()),
                })
                .collect(),
        )
        .await?;
        teams
            .into_iter()
            .find(|t| t.team_id == selected)
            .ok_or("invalid_team")?
    };
    session
        .lock()
        .map_err(|_| "state_unavailable")?
        .set_phase(Phase::Provisioning);
    dev.ensure_device_registered(&team, "MegaNugget test iPhone", &udid, None)
        .await
        .map_err(|_| "device_registration_failed")?;
    let mut signer =
        SideloaderBuilder::<MaxCertsCallbackBox, ExtensionsBehaviorCallbackBox>::new(dev, email)
            .max_certs_behavior(MaxCertsBehavior::Error)
            .storage(Box::new(KeyringStorage::new(
                "org.meganugget.signing.certificates".into(),
            )))
            .machine_name("MegaNugget Desktop".into())
            .build();
    session
        .lock()
        .map_err(|_| "state_unavailable")?
        .set_phase(Phase::Signing);
    let (signed_app, _) = signer
        .sign_app(
            app_path,
            Some(team),
            false,
            None::<fn(f32) -> std::future::Ready<()>>,
            None,
            None,
        )
        .await
        .map_err(|_| "apple_signing_failed_no_automatic_revocation")?;
    let output = workspace.path().join("signed.ipa");
    archive_app(&signed_app, &output)?;
    app_installer::inspect(&output).map_err(|_| "signed_ipa_validation_failed")
}

fn archive_app(app: &Path, output: &Path) -> Result<(), &'static str> {
    let file = std::fs::File::create(output).map_err(|_| "signed_ipa_write_failed")?;
    let mut zip = zip::ZipWriter::new(file);
    let root = Path::new("Payload").join(app.file_name().ok_or("invalid_signed_app")?);
    let mut pending = vec![app.to_path_buf()];
    while let Some(dir) = pending.pop() {
        for entry in std::fs::read_dir(dir).map_err(|_| "signed_ipa_write_failed")? {
            let entry = entry.map_err(|_| "signed_ipa_write_failed")?;
            let path = entry.path();
            let kind = entry.file_type().map_err(|_| "signed_ipa_write_failed")?;
            if kind.is_symlink() {
                return Err("signed_app_symlink_rejected");
            }
            if kind.is_dir() {
                pending.push(path);
                continue;
            }
            let member = root
                .join(path.strip_prefix(app).map_err(|_| "invalid_signed_app")?)
                .to_string_lossy()
                .replace('\\', "/");
            zip.start_file(
                member,
                zip::write::SimpleFileOptions::default().unix_permissions(0o755),
            )
            .map_err(|_| "signed_ipa_write_failed")?;
            std::io::copy(
                &mut std::fs::File::open(path).map_err(|_| "signed_ipa_write_failed")?,
                &mut zip,
            )
            .map_err(|_| "signed_ipa_write_failed")?;
        }
    }
    zip.finish().map_err(|_| "signed_ipa_write_failed")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn version_gate() {
        assert!(version_allowed("15.0", "15.8.8"));
        for min in ["26.0", "15.x", "", "15.8.8.0"] {
            assert!(!version_allowed(min, "15.8.8"));
        }
    }
    #[test]
    fn no_secret_fields_in_status() {
        let session = Session::default();
        let json = serde_json::to_value(session.status()).unwrap();
        assert_eq!(json.as_object().unwrap().len(), 3);
    }
    #[test]
    fn no_unexpected_response() {
        assert!(Session::default().respond("123456".into()).is_err());
    }
    #[test]
    fn two_factor_validation_and_delivery() {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_time()
            .build()
            .unwrap();
        runtime.block_on(async {
            let state = SharedSession::default();
            let other = state.clone();
            let task =
                tokio::spawn(async move { challenge(&other, Phase::TwoFactor, vec![]).await });
            tokio::task::yield_now().await;
            assert!(state.lock().unwrap().respond("123".into()).is_err());
            assert!(state.lock().unwrap().respond("abcdef".into()).is_err());
            state.lock().unwrap().respond("123456".into()).unwrap();
            assert_eq!(task.await.unwrap().unwrap(), "123456");
            assert_eq!(state.lock().unwrap().status().phase, Phase::Authenticating);
        });
    }
    #[test]
    fn cancellation_drops_challenge() {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_time()
            .build()
            .unwrap();
        runtime.block_on(async {
            let state = SharedSession::default();
            let other = state.clone();
            let task =
                tokio::spawn(async move { challenge(&other, Phase::TwoFactor, vec![]).await });
            tokio::task::yield_now().await;
            state.lock().unwrap().set_phase(Phase::Cancelled);
            assert!(task.await.unwrap().is_err());
            assert_eq!(state.lock().unwrap().status().phase, Phase::Cancelled);
        });
    }
}

//! Locating and verifying official upstream release artifacts.
//! Downloads are bounded, checksummed and content-addressed; nothing is
//! modified, and versions are reported as-is (prereleases stay prereleases).

use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::process::Command;
use std::time::Duration;

pub const REPO: &str = "opa334/Dopamine";
/// GitHub JSON for a release list is small; artifacts are tens of MB.
pub const DEFAULT_MAX_JSON: usize = 8 * 1024 * 1024;
pub const DEFAULT_MAX_ASSET: usize = 192 * 1024 * 1024;

#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct Asset {
    pub name: String,
    pub url: String,
    pub size: u64,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct Release {
    pub tag: String,
    pub prerelease: bool,
    pub assets: Vec<Asset>,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum FetchError {
    ToolMissing,
    Timeout,
    OutputLimit,
    Transfer,
    InvalidResponse,
}

impl std::fmt::Display for FetchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::ToolMissing => "fetch_tool_missing",
            Self::Timeout => "fetch_timeout",
            Self::OutputLimit => "fetch_output_limit",
            Self::Transfer => "fetch_transfer_failed",
            Self::InvalidResponse => "fetch_invalid_response",
        })
    }
}

/// Minimal bounded HTTP surface; the system runner sources bytes via `curl`
/// and never trusts response size above the configured cap.
pub trait Fetcher: Send + Sync {
    fn get(&self, url: &str, max_bytes: usize, timeout: Duration) -> Result<Vec<u8>, FetchError>;
}

pub struct SystemFetcher;

impl Fetcher for SystemFetcher {
    fn get(&self, url: &str, max_bytes: usize, timeout: Duration) -> Result<Vec<u8>, FetchError> {
        let program = if cfg!(windows) { "curl.exe" } else { "curl" };
        let max_time = timeout.as_secs().clamp(1, u64::from(u32::MAX)).to_string();
        let output = Command::new(program)
            .args([
                "-fsSL",
                "--max-time",
                &max_time,
                "--max-filesize",
                &max_bytes.to_string(),
                url,
            ])
            .output()
            .map_err(|e| {
                if e.kind() == std::io::ErrorKind::NotFound {
                    FetchError::ToolMissing
                } else {
                    FetchError::Transfer
                }
            })?;
        // curl: 28 = operation timed out; other non-zero codes are plain failures.
        if output.status.code() == Some(28) {
            return Err(FetchError::Timeout);
        }
        if !output.status.success() {
            return Err(FetchError::Transfer);
        }
        if output.stdout.len() > max_bytes {
            return Err(FetchError::OutputLimit);
        }
        Ok(output.stdout)
    }
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    bytes
        .iter()
        .fold(Sha256::new(), |mut h, chunk| {
            h.update([*chunk]);
            h
        })
        .finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

fn parse_releases(json: &Value) -> Result<Vec<Release>, FetchError> {
    let Some(list) = json.as_array() else {
        return Err(FetchError::InvalidResponse);
    };
    let mut releases = Vec::new();
    for item in list {
        let Some(tag) = item.get("tag_name").and_then(Value::as_str) else {
            return Err(FetchError::InvalidResponse);
        };
        let Some(assets) = item.get("assets").and_then(Value::as_array) else {
            return Err(FetchError::InvalidResponse);
        };
        let mut parsed = Vec::new();
        for asset in assets {
            let (Some(name), Some(url)) = (
                asset.get("name").and_then(Value::as_str),
                asset.get("browser_download_url").and_then(Value::as_str),
            ) else {
                return Err(FetchError::InvalidResponse);
            };
            parsed.push(Asset {
                name: name.to_string(),
                url: url.to_string(),
                size: asset
                    .get("size")
                    .and_then(Value::as_u64)
                    .ok_or(FetchError::InvalidResponse)?,
            });
        }
        releases.push(Release {
            tag: tag.to_string(),
            prerelease: item
                .get("prerelease")
                .and_then(Value::as_bool)
                .ok_or(FetchError::InvalidResponse)?,
            assets: parsed,
        });
    }
    Ok(releases)
}

/// Client tied to one repository; keeps fetch details out of callers.
pub struct ReleaseClient<T: Fetcher> {
    pub fetcher: T,
}

impl<T: Fetcher> ReleaseClient<T> {
    pub fn releases(&self, max_bytes: usize) -> Result<Vec<Release>, FetchError> {
        let json = self.fetcher.get(
            &format!("https://api.github.com/repos/{REPO}/releases"),
            max_bytes,
            Duration::from_secs(60),
        )?;
        let value: Value =
            serde_json::from_slice(&json).map_err(|_| FetchError::InvalidResponse)?;
        parse_releases(&value)
    }
}

/// Exact-tag lookup; tags are authoritative, no fuzzy matching.
pub fn find_by_tag<'a>(releases: &'a [Release], tag: &str) -> Result<&'a Release, FetchError> {
    releases
        .iter()
        .find(|r| r.tag == tag)
        .ok_or(FetchError::InvalidResponse)
}

/// The app artifact for on-device install; `.tipa` is the TrollStore twin.
pub fn app_artifact(release: &Release) -> Result<&Asset, FetchError> {
    release
        .assets
        .iter()
        .find(|a| a.name.ends_with(".ipa"))
        .ok_or(FetchError::InvalidResponse)
}

/// Verified download: bytes plus their digest, for the caller to pin.
pub fn download_verified<T: Fetcher>(
    client: &ReleaseClient<T>,
    asset: &Asset,
    max_bytes: usize,
) -> Result<(Vec<u8>, String), FetchError> {
    if asset.size > u64::try_from(max_bytes).unwrap_or(u64::MAX) {
        return Err(FetchError::OutputLimit);
    }
    let bytes = client
        .fetcher
        .get(&asset.url, max_bytes, Duration::from_secs(600))?;
    if bytes.len() != usize::try_from(asset.size).unwrap_or(usize::MAX) {
        return Err(FetchError::Transfer);
    }
    let digest = sha256_hex(&bytes);
    Ok((bytes, digest))
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Stub {
        body: Vec<u8>,
        fail: Option<FetchError>,
    }
    impl Stub {
        fn new(body: &[u8]) -> Self {
            Self {
                body: body.to_vec(),
                fail: None,
            }
        }
    }
    impl Fetcher for Stub {
        fn get(
            &self,
            _url: &str,
            max_bytes: usize,
            _timeout: Duration,
        ) -> Result<Vec<u8>, FetchError> {
            if let Some(e) = self.fail {
                return Err(e);
            }
            if self.body.len() > max_bytes {
                return Err(FetchError::OutputLimit);
            }
            Ok(self.body.clone())
        }
    }

    const SAMPLE: &str = r#"[{"tag_name":"3.0.13","prerelease":false,"assets":[{"name":"Dopamine.ipa","browser_download_url":"https://example/dopamine3.ipa","size":7},{"name":"Dopamine.tipa","browser_download_url":"https://example/dopamine3.tipa","size":7}]},{"tag_name":"2.5b4","prerelease":true,"assets":[{"name":"Dopamine.ipa","browser_download_url":"https://example/dopamine25.ipa","size":11}]}]"#;

    #[test]
    fn releases_json_is_parsed() {
        let client = ReleaseClient {
            fetcher: Stub::new(SAMPLE.as_bytes()),
        };
        let releases = client.releases(DEFAULT_MAX_JSON).unwrap();
        assert_eq!(releases.len(), 2);
        assert_eq!(releases[0].tag, "3.0.13");
        assert!(!releases[0].prerelease);
        assert!(releases[1].prerelease);
    }

    #[test]
    fn invalid_json_is_rejected() {
        let client = ReleaseClient {
            fetcher: Stub::new(b"not json"),
        };
        assert_eq!(
            client.releases(DEFAULT_MAX_JSON),
            Err(FetchError::InvalidResponse)
        );
    }

    #[test]
    fn fetch_errors_marshalled() {
        let client = ReleaseClient {
            fetcher: Stub {
                fail: Some(FetchError::Timeout),
                body: vec![],
            },
        };
        assert_eq!(client.releases(DEFAULT_MAX_JSON), Err(FetchError::Timeout));
    }

    #[test]
    fn tag_must_match_exactly() {
        let client = ReleaseClient {
            fetcher: Stub::new(SAMPLE.as_bytes()),
        };
        let releases = client.releases(DEFAULT_MAX_JSON).unwrap();
        let beta = find_by_tag(&releases, "2.5b4").unwrap();
        assert!(beta.prerelease);
        assert!(find_by_tag(&releases, "2.5").is_err());
    }

    #[test]
    fn only_ipa_is_picked_as_app_artifact() {
        let client = ReleaseClient {
            fetcher: Stub::new(SAMPLE.as_bytes()),
        };
        let releases = client.releases(DEFAULT_MAX_JSON).unwrap();
        let stable = find_by_tag(&releases, "3.0.13").unwrap();
        let artifact = app_artifact(stable).unwrap();
        assert_eq!(artifact.name, "Dopamine.ipa");
        assert_eq!(artifact.url, "https://example/dopamine3.ipa");
    }

    #[test]
    fn download_verifies_size_and_digest() {
        struct Fixed;
        impl Fetcher for Fixed {
            fn get(&self, _: &str, _: usize, _: Duration) -> Result<Vec<u8>, FetchError> {
                Ok(b"dopamine-ipa".to_vec())
            }
        }
        let client = ReleaseClient { fetcher: Fixed };
        let asset = Asset {
            name: "Dopamine.ipa".into(),
            url: "u".into(),
            size: b"dopamine-ipa".len() as u64,
        };
        let (bytes, digest) = download_verified(&client, &asset, DEFAULT_MAX_JSON).unwrap();
        assert_eq!(bytes, b"dopamine-ipa");
        assert_eq!(digest, sha256_hex(b"dopamine-ipa"));
    }

    #[test]
    fn oversize_assets_fail_before_request() {
        let client = ReleaseClient {
            fetcher: Stub::new(SAMPLE.as_bytes()),
        };
        let releases = client.releases(DEFAULT_MAX_JSON).unwrap();
        let asset = &releases[0].assets[0];
        let big = Asset {
            name: asset.name.clone(),
            url: asset.url.clone(),
            size: DEFAULT_MAX_JSON as u64 + 1,
        };
        assert_eq!(
            download_verified(&client, &big, DEFAULT_MAX_JSON),
            Err(FetchError::OutputLimit)
        );
    }

    #[test]
    fn size_mismatch_is_a_transfer_error() {
        struct Wrong;
        impl Fetcher for Wrong {
            fn get(&self, _: &str, _: usize, _: Duration) -> Result<Vec<u8>, FetchError> {
                Ok(b"12".to_vec())
            }
        }
        let client = ReleaseClient { fetcher: Wrong };
        let asset = Asset {
            name: "a.ipa".into(),
            url: "u".into(),
            size: 11,
        };
        assert_eq!(
            download_verified(&client, &asset, DEFAULT_MAX_JSON),
            Err(FetchError::Transfer)
        );
    }

    #[test]
    fn sha256_hex_matches_known_vectors() {
        assert_eq!(
            sha256_hex(b"hello"),
            "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824"
        );
        assert_eq!(
            sha256_hex(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }
}

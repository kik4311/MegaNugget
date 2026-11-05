use serde::{Deserialize, Serialize};
use std::{
    io::Read,
    process::{Command, Stdio},
    time::Duration,
};
use wait_timeout::ChildExt;

pub const MAX_OUTPUT: u64 = 4 * 1024 * 1024;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Failure {
    ToolMissing,
    PermissionDenied,
    Timeout,
    OutputLimit,
    CommandFailed,
    InvalidOutput,
}

pub struct Output {
    pub success: bool,
    pub stdout: Vec<u8>,
}

/// No shell, no arbitrary command names accepted from IPC, no raw stderr in reports.
pub trait Runner {
    fn run(&self, tool: &str, args: &[&str], timeout: Duration) -> Result<Output, Failure>;
}
pub struct SystemRunner;

fn drain(mut reader: impl Read) -> std::io::Result<Vec<u8>> {
    let mut bytes = Vec::new();
    let mut chunk = [0u8; 8192];
    let mut exceeded = false;
    loop {
        let n = reader.read(&mut chunk)?;
        if n == 0 {
            break;
        }
        if bytes.len() + n <= MAX_OUTPUT as usize {
            bytes.extend_from_slice(&chunk[..n]);
        } else {
            exceeded = true;
        }
    }
    if exceeded {
        Err(std::io::Error::other("output limit"))
    } else {
        Ok(bytes)
    }
}

impl Runner for SystemRunner {
    fn run(&self, tool: &str, args: &[&str], timeout: Duration) -> Result<Output, Failure> {
        let mut command = Command::new(tool);
        command
            .args(args)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            command.creation_flags(0x08000000);
        }
        let mut child = command.spawn().map_err(|e| match e.kind() {
            std::io::ErrorKind::NotFound => Failure::ToolMissing,
            std::io::ErrorKind::PermissionDenied => Failure::PermissionDenied,
            _ => Failure::CommandFailed,
        })?;
        let stdout = child.stdout.take().ok_or(Failure::CommandFailed)?;
        let stderr = child.stderr.take().ok_or(Failure::CommandFailed)?;
        let out = std::thread::spawn(move || drain(stdout));
        let err = std::thread::spawn(move || drain(stderr));
        let waited = child.wait_timeout(timeout);
        let status = match waited {
            Ok(Some(status)) => Ok(status),
            result => {
                let _ = child.kill();
                let _ = child.wait();
                Err(if result.is_ok() {
                    Failure::Timeout
                } else {
                    Failure::CommandFailed
                })
            }
        };
        let stdout = out.join().map_err(|_| Failure::CommandFailed)?;
        let stderr = err.join().map_err(|_| Failure::CommandFailed)?;
        let status = status?;
        let stdout = stdout.map_err(|_| Failure::OutputLimit)?;
        stderr.map_err(|_| Failure::OutputLimit)?;
        Ok(Output {
            success: status.success(),
            stdout,
        })
    }
}

pub fn checked(
    runner: &impl Runner,
    tool: &str,
    args: &[&str],
    seconds: u64,
) -> Result<Vec<u8>, Failure> {
    let out = runner.run(tool, args, Duration::from_secs(seconds))?;
    if !out.success {
        return Err(Failure::CommandFailed);
    }
    Ok(out.stdout)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceInfo {
    #[serde(rename = "ProductType")]
    pub product: String,
    #[serde(rename = "ProductVersion")]
    pub ios: String,
    #[serde(rename = "BuildVersion")]
    pub build: String,
    #[serde(rename = "CPUArchitecture", default)]
    pub architecture: String,
}

#[derive(Debug, Serialize)]
pub struct Device {
    pub id: String,
    pub info: Option<DeviceInfo>,
    pub trust: &'static str,
    pub failure: Option<Failure>,
}

#[derive(Debug, Serialize)]
pub struct Scan {
    pub transport: &'static str,
    pub devices: Vec<Device>,
    pub failure: Option<Failure>,
}

pub fn valid_udid(id: &str) -> bool {
    (id.len() == 40 && id.bytes().all(|b| b.is_ascii_hexdigit()))
        || (id.len() == 25
            && id.as_bytes()[8] == b'-'
            && id
                .bytes()
                .enumerate()
                .all(|(i, b)| i == 8 || b.is_ascii_hexdigit()))
}

pub fn device_info(runner: &impl Runner, id: &str) -> Result<DeviceInfo, Failure> {
    if !valid_udid(id) {
        return Err(Failure::InvalidOutput);
    }
    let bytes = checked(runner, "ideviceinfo", &["-u", id, "-x"], 15)?;
    plist::from_bytes(&bytes).map_err(|_| Failure::InvalidOutput)
}

/// idevice_id -l enumerates USB only; no pairing is initiated by this application.
pub fn scan(runner: &impl Runner) -> Scan {
    let result = (|| {
        let bytes = checked(runner, "idevice_id", &["-l"], 10)?;
        let text = std::str::from_utf8(&bytes).map_err(|_| Failure::InvalidOutput)?;
        let mut devices = Vec::new();
        for id in text.lines().map(str::trim).filter(|id| !id.is_empty()) {
            if !valid_udid(id) || devices.len() >= 32 {
                return Err(Failure::InvalidOutput);
            }
            let info = device_info(runner, id);
            devices.push(match info {
                Ok(info) => Device {
                    id: id.into(),
                    info: Some(info),
                    trust: "lockdown_readable",
                    failure: None,
                },
                Err(failure) => Device {
                    id: id.into(),
                    info: None,
                    trust: "unknown",
                    failure: Some(failure),
                },
            });
        }
        Ok(devices)
    })();
    match result {
        Ok(devices) => Scan {
            transport: "usbmuxd_usb",
            devices,
            failure: None,
        },
        Err(failure) => Scan {
            transport: "usbmuxd_usb",
            devices: vec![],
            failure: Some(failure),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Fake {
        list: &'static str,
        readable: bool,
    }
    impl Runner for Fake {
        fn run(&self, tool: &str, args: &[&str], _: Duration) -> Result<Output, Failure> {
            if tool == "idevice_id" {
                assert_eq!(args, ["-l"]);
                return Ok(Output {
                    success: true,
                    stdout: self.list.into(),
                });
            }
            Ok(Output { success: self.readable, stdout: br#"<?xml version="1.0"?><plist version="1.0"><dict><key>ProductType</key><string>iPhone9,1</string><key>ProductVersion</key><string>15.8.8</string><key>BuildVersion</key><string>test-build</string></dict></plist>"#.to_vec() })
        }
    }
    #[test]
    fn disconnected_is_not_failure() {
        let s = scan(&Fake {
            list: "",
            readable: true,
        });
        assert!(s.devices.is_empty() && s.failure.is_none());
    }
    #[test]
    fn usb_and_info() {
        let s = scan(&Fake {
            list: "0123456789012345678901234567890123456789\n",
            readable: true,
        });
        assert_eq!(s.devices[0].info.as_ref().unwrap().product, "iPhone9,1");
        assert_eq!(s.devices[0].trust, "lockdown_readable");
    }
    #[test]
    fn failure_does_not_claim_unpaired() {
        let s = scan(&Fake {
            list: "0123456789012345678901234567890123456789",
            readable: false,
        });
        assert_eq!(s.devices[0].trust, "unknown");
        assert!(s.devices[0].failure.is_some());
    }
    #[test]
    fn reject_untrusted_id() {
        assert!(scan(&Fake {
            list: "--network",
            readable: true
        })
        .failure
        .is_some());
        assert!(valid_udid("00008020-0123456789ABCDEF"));
        assert!(!valid_udid("../../file"));
    }

    #[test]
    fn output_is_bounded() {
        assert!(drain(std::io::Cursor::new(vec![b'x'; MAX_OUTPUT as usize])).is_ok());
        assert!(drain(std::io::Cursor::new(vec![b'x'; MAX_OUTPUT as usize + 1])).is_err());
    }

    #[test]
    fn missing_tool_is_explicit() {
        assert!(matches!(
            SystemRunner.run(
                "meganugget-nonexistent-tool-for-test",
                &[],
                Duration::from_secs(1)
            ),
            Err(Failure::ToolMissing)
        ));
    }

    #[test]
    fn timeout_propagates_without_device_claims() {
        struct TimedOut;
        impl Runner for TimedOut {
            fn run(&self, _: &str, _: &[&str], _: Duration) -> Result<Output, Failure> {
                Err(Failure::Timeout)
            }
        }
        let result = scan(&TimedOut);
        assert!(matches!(result.failure, Some(Failure::Timeout)));
        assert!(result.devices.is_empty());
    }
}

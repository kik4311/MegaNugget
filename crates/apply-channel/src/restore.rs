//! Stage 3, step 2: put the merged copy onto the device with
//! `idevicebackup2 restore`. This is the only command in the channel that
//! writes to the device, and it is gated by:
//!
//! 1. an explicit consent flag — missing consent refuses before anything is
//!    spawned at all (`restore_requires_consent`);
//! 2. the merged copy passing the exact same verify gates as a real backup;
//! 3. exactly one connected device matching the exact target.
//!
//! The tool runs whitelisted full-mode arguments only (`-u <udid> restore
//! <dir>` — no selective/system/settings/reboot flags). Its output is
//! drained, counted, and never printed: progress lines can contain device
//! paths with the raw UDID.
//!
//! A restore has no operational timeout — a 3.9 GB restore legitimately runs
//! for a long time. The deadline below exists purely as a runaway kill-guard,
//! and the output cap protects memory, not the operation: past the cap the
//! bytes are discarded while the tool keeps running.

use std::io::Read;
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use device_manager::Runner;
use serde::Serialize;

use crate::manifest::debug;
use crate::{
    device_directory, verify_backup, Error, RESTORE_OUTPUT_CAP_BYTES, RESTORE_TIMEOUT_SECS,
    RESTORE_TOOL,
};

#[derive(Debug, Serialize)]
pub struct RestoreReport {
    pub udid_masked: String,
    pub output_truncated: bool,
}

/// Apply `applied_dir` (a merged, self-verified copy) to the one connected
/// target device. `consent` is the explicit user decision for this exact
/// step; with `consent == false` nothing is scanned or spawned.
pub fn restore(
    runner: &impl Runner,
    applied_dir: &Path,
    consent: bool,
) -> Result<RestoreReport, Error> {
    if !consent {
        return Err(Error::RestoreRequiresConsent);
    }
    verify_backup(applied_dir)?;
    let (_, udid) = device_directory(applied_dir)?;
    crate::single_target_device(runner)?;
    let args = restore_args(&udid, applied_dir);
    let args_ref: Vec<&str> = args.iter().map(String::as_str).collect();
    let outcome = LongRunner::new(RESTORE_TOOL, &args_ref)
        .deadline(Duration::from_secs(RESTORE_TIMEOUT_SECS))
        .cap(RESTORE_OUTPUT_CAP_BYTES)
        .run()?;
    if !outcome.success {
        debug("restore failed", &format!("killed={}", outcome.killed));
        return Err(Error::RestoreFailed);
    }
    Ok(RestoreReport {
        udid_masked: crate::mask_udid(&udid),
        output_truncated: outcome.truncated,
    })
}

/// The fixed command line handed to `idevicebackup2` — deliberately a strict
/// whitelist so no caller can smuggle selective or destructive options in.
pub fn restore_args(udid: &str, applied_dir: &Path) -> Vec<String> {
    vec![
        "-u".into(),
        udid.into(),
        "restore".into(),
        applied_dir.display().to_string(),
    ]
}

pub(crate) struct LongOutcome {
    pub success: bool,
    pub killed: bool,
    pub truncated: bool,
    /// Accounted bytes (bounded by the cap); asserted in tests only.
    #[cfg_attr(not(test), allow(dead_code))]
    pub bytes: u64,
}

pub(crate) struct LongRunner<'a> {
    tool: &'a str,
    args: &'a [&'a str],
    deadline: Duration,
    poll: Duration,
    cap: u64,
}

impl<'a> LongRunner<'a> {
    pub(crate) fn new(tool: &'a str, args: &'a [&'a str]) -> Self {
        Self {
            tool,
            args,
            deadline: Duration::from_secs(RESTORE_TIMEOUT_SECS),
            poll: Duration::from_millis(250),
            cap: RESTORE_OUTPUT_CAP_BYTES,
        }
    }

    /// Runaway kill-guard. Real restore steps must not end because of it:
    /// the default (24h) is far beyond any real restore of this device.
    pub(crate) fn deadline(mut self, deadline: Duration) -> Self {
        self.deadline = deadline;
        self
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn poll(mut self, poll: Duration) -> Self {
        self.poll = poll;
        self
    }

    /// Bound on accounted output; bytes past the cap are discarded while the
    /// tool keeps running (marked truncated in the outcome).
    pub(crate) fn cap(mut self, cap: u64) -> Self {
        self.cap = cap;
        self
    }

    /// Spawn the tool, drain stdout/stderr through capped counters (content
    /// is never kept in memory, never printed), enforce the deadline
    /// kill-guard, and rely on the child's own exit status.
    pub(crate) fn run(&self) -> Result<LongOutcome, Error> {
        let mut child = Command::new(self.tool)
            .args(self.args)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .inspect_err(|error| debug("restore spawn", &error.to_string()))
            .map_err(|error| match error.kind() {
                std::io::ErrorKind::NotFound => Error::RestoreToolMissing,
                _ => Error::RestoreFailed,
            })?;
        let counter = Arc::new(AtomicU64::new(0));
        let truncated = Arc::new(AtomicBool::new(false));
        let stdout = child.stdout.take().ok_or(Error::RestoreFailed)?;
        let stderr = child.stderr.take().ok_or(Error::RestoreFailed)?;
        let pipes: Vec<Box<dyn Read + Send>> = vec![Box::new(stdout), Box::new(stderr)];
        let drains = pipes
            .into_iter()
            .map(|pipe| {
                let counter = counter.clone();
                let truncated = truncated.clone();
                let cap = self.cap;
                std::thread::spawn(move || drain_counted(pipe, &counter, &truncated, cap))
            })
            .collect::<Vec<_>>();
        let deadline = Instant::now() + self.deadline;
        let mut killed = false;
        let status = loop {
            match child.try_wait() {
                Ok(Some(status)) => break status,
                Ok(None) => {
                    if Instant::now() >= deadline {
                        let _ = child.kill();
                        let status = child.wait().map_err(|_| {
                            debug("restore wait", "post-kill wait failed");
                            Error::RestoreFailed
                        })?;
                        killed = true;
                        break status;
                    }
                    std::thread::sleep(self.poll);
                }
                Err(error) => {
                    debug("restore wait", &error.to_string());
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(Error::RestoreFailed);
                }
            }
        };
        for drain in drains {
            let _ = drain.join();
        }
        Ok(LongOutcome {
            success: !killed && status.success(),
            killed,
            truncated: truncated.load(Ordering::Relaxed),
            bytes: counter.load(Ordering::Relaxed),
        })
    }
}

fn drain_counted(mut pipe: impl Read, counter: &AtomicU64, truncated: &AtomicBool, cap: u64) {
    let mut chunk = [0u8; 16 * 1024];
    loop {
        match pipe.read(&mut chunk) {
            Ok(0) => return,
            Ok(n) => {
                let seen = counter.load(Ordering::Relaxed);
                if seen >= cap {
                    truncated.store(true, Ordering::Relaxed);
                    continue;
                }
                let retained = (cap - seen).min(n as u64);
                counter.fetch_add(retained, Ordering::Relaxed);
                if retained < n as u64 {
                    truncated.store(true, Ordering::Relaxed);
                }
                // Deliberately discarded: restore output must never
                // accumulate in memory past the cap, and it must never be
                // echoed (it can carry the raw UDID in device paths).
            }
            Err(_) => return, // pipe broken because the child exited
        }
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use device_manager::{Failure, Output};
    use std::sync::{Arc, Mutex};

    struct Probe {
        calls: Arc<Mutex<Vec<String>>>,
    }

    impl Runner for Probe {
        fn run(&self, tool: &str, args: &[&str], _: Duration) -> Result<Output, Failure> {
            self.calls
                .lock()
                .expect("calls")
                .push(format!("{tool} {args:?}"));
            if tool == "idevice_id" {
                return Ok(Output {
                    success: true,
                    stdout: "0123456789012345678901234567890123456789\n".into(),
                });
            }
            Err(Failure::ToolMissing)
        }
    }

    #[test]
    fn restore_args_whitelist() {
        assert_eq!(
            restore_args("UDIDPLACEHOLDER", Path::new("/tmp/applied")),
            vec![
                "-u".to_string(),
                "UDIDPLACEHOLDER".to_string(),
                "restore".to_string(),
                "/tmp/applied".to_string(),
            ]
        );
    }

    #[test]
    fn restore_refuses_without_consent_before_any_spawn() {
        let probe = Probe {
            calls: Arc::new(Mutex::new(Vec::new())),
        };
        let error = restore(&probe, Path::new("/no/entry"), false).unwrap_err();
        assert_eq!(error.code(), "restore_requires_consent");
        assert!(
            probe.calls.lock().expect("calls").is_empty(),
            "consent gate runs before any tool call"
        );
    }

    #[test]
    fn restore_refuses_unverified_copy_before_any_spawn() {
        let probe = Probe {
            calls: Arc::new(Mutex::new(Vec::new())),
        };
        let empty = tempfile::tempdir().expect("tempdir");
        let error = restore(&probe, empty.path(), true).unwrap_err();
        assert_eq!(error.code(), "backup_layout_unsupported");
        assert!(
            probe
                .calls
                .lock()
                .expect("calls")
                .iter()
                .all(|call| !call.starts_with("idevicebackup2")),
            "restore must not run for an unverified copy"
        );
    }

    #[test]
    fn long_runner_reports_missing_tool() {
        let outcome = LongRunner::new("/nonexistent/apply-channel-missing-tool", &[]).run();
        assert!(matches!(outcome, Err(Error::RestoreToolMissing)));
    }

    #[test]
    fn long_runner_small_success() {
        let outcome = LongRunner::new("/bin/sh", &["-c", "echo hello"])
            .run()
            .unwrap();
        assert!(outcome.success);
        assert!(!outcome.truncated);
        assert!(!outcome.killed);
        assert!(outcome.bytes >= 6);
    }

    #[test]
    fn long_runner_caps_output_without_aborting() {
        let outcome = LongRunner::new("/bin/sh", &["-c", "head -c 70000000 /dev/zero"])
            .cap(1024 * 1024)
            .run()
            .expect("run");
        assert!(
            outcome.success,
            "the cap protects memory, not the operation"
        );
        assert!(outcome.truncated);
        assert!(
            outcome.bytes <= 2 * 1024 * 1024,
            "counting stops at the cap"
        );
    }

    #[test]
    fn long_runner_kill_guard_fires() {
        let outcome = LongRunner::new("/bin/sh", &["-c", "sleep 30"])
            .deadline(Duration::from_millis(300))
            .poll(Duration::from_millis(50))
            .run()
            .expect("run");
        assert!(outcome.killed);
        assert!(!outcome.success);
    }

    #[test]
    fn long_runner_failing_exit_is_reported() {
        let outcome = LongRunner::new("/bin/sh", &["-c", "exit 3"]).run().unwrap();
        assert!(!outcome.success);
        assert!(!outcome.killed);
    }
}

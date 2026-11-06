//! CLI prototype of the apply channel — stage 1, read-only.
//!
//! ```text
//! apply-channel detect
//! apply-channel inspect <dir>
//! apply-channel verify <dir>
//! apply-channel backup <dir> --consent-backup
//! ```
//!
//! Errors are one-line stable snake_case codes on stderr (`error: <code>`,
//! exit 1). Output is single-line JSON on stdout, whitelisted fields only.

use std::path::PathBuf;
use std::process::ExitCode;

use apply_channel::{detection, inspect_backup, verify_backup, Error};
use device_manager::SystemRunner;

const USAGE: &str =
    "usage: apply-channel detect | inspect <dir> | verify <dir> | backup <dir> --consent-backup";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let args_ref: Vec<&str> = args.iter().map(String::as_str).collect();
    let runner = SystemRunner;
    match args_ref.as_slice() {
        [flag] if flag == &"-h" || flag == &"--help" => {
            println!("{USAGE}");
            ExitCode::SUCCESS
        }
        ["detect"] => run(detection(&runner)),
        ["inspect", dir] => run(inspect_backup(&PathBuf::from(dir))),
        ["verify", dir] => run(verify_backup(&PathBuf::from(dir))),
        // No consent flag: refuse before touching anything (fail closed).
        ["backup", _] => run::<apply_channel::BackupSnapshot>(Err(Error::BackupRequiresConsent)),
        ["backup", dir, "--consent-backup"] => {
            run(apply_channel::backup(&runner, &PathBuf::from(dir), true))
        }
        _ => {
            eprintln!("{USAGE}");
            ExitCode::from(2)
        }
    }
}

fn run<T: serde::Serialize>(result: Result<T, Error>) -> ExitCode {
    match result {
        Ok(report) => {
            println!("{}", serde_json::to_string(&report).expect("serializable"));
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("error: {}", error.code());
            ExitCode::FAILURE
        }
    }
}

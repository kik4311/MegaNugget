#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use diagnostics::{Event, Journal, Level};
use serde::Serialize;
use std::sync::{Arc, Mutex};
use tauri::State;

#[derive(Default)]
struct Inner {
    journal: Journal,
    ipa: Option<app_installer::ValidatedIpa>,
}
type Shared = Arc<Mutex<Inner>>;

#[derive(Serialize)]
struct DeviceView {
    device: device_manager::Device,
    compatibility: Option<compatibility::Compatibility>,
}
#[derive(Serialize)]
struct ScanView {
    devices: Vec<DeviceView>,
    failure: Option<device_manager::Failure>,
    installer: Result<app_installer::InstallerCli, app_installer::InstallError>,
}

#[tauri::command]
async fn scan_devices(state: State<'_, Shared>) -> Result<ScanView, String> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let runner = device_manager::SystemRunner;
        let scan = device_manager::scan(&runner);
        state
            .lock()
            .map_err(|_| "state_unavailable")?
            .journal
            .record(
                if scan.failure.is_some() {
                    Level::Error
                } else {
                    Level::Info
                },
                if scan.failure.is_some() {
                    Event::ScanFailed
                } else {
                    Event::ScanCompleted
                },
            );
        Ok(ScanView {
            devices: scan
                .devices
                .into_iter()
                .map(|device| {
                    let compatibility = device
                        .info
                        .as_ref()
                        .map(|info| compatibility::evaluate(&info.product, &info.ios));
                    DeviceView {
                        device,
                        compatibility,
                    }
                })
                .collect(),
            failure: scan.failure,
            installer: app_installer::installer_cli(&runner),
        })
    })
    .await
    .map_err(|_| "worker_failed".to_string())?
}

#[tauri::command]
async fn choose_ipa(state: State<'_, Shared>) -> Result<Option<serde_json::Value>, String> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let mut inner = state.lock().map_err(|_| "state_unavailable")?;
        inner.ipa = None;
        let Some(path) = rfd::FileDialog::new()
            .add_filter("iOS application", &["ipa"])
            .pick_file()
        else {
            return Ok(None);
        };
        match app_installer::inspect(&path) {
            Ok(ipa) => {
                let info = serde_json::to_value(&ipa.info).map_err(|_| "serialization_failed")?;
                inner.ipa = Some(ipa);
                inner.journal.record(Level::Info, Event::IpaValidated);
                Ok(Some(info))
            }
            Err(error) => {
                inner.journal.record(Level::Warning, Event::IpaRejected);
                Err(format!("{error:?}"))
            }
        }
    })
    .await
    .map_err(|_| "worker_failed".to_string())?
}

#[tauri::command]
async fn install_ipa(id: String, state: State<'_, Shared>) -> Result<String, String> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        if !device_manager::valid_udid(&id) { return Err("invalid_device".into()); }
        let mut inner = state.lock().map_err(|_| "state_unavailable")?;
        let ipa = inner.ipa.as_ref().ok_or("select_ipa_first")?;
        let confirmed = rfd::MessageDialog::new().set_title("MegaNugget — confirm installation")
            .set_description(format!("Install {} (build {}) on device {}?\nThis can replace an existing app with the same bundle ID.\nApple signing and provisioning are required. No bypass is performed.\n\nУстановить приложение на выбранное устройство? Это может заменить существующее приложение. Подпись Apple обязательна.", ipa.info.bundle_id, ipa.info.version, id))
            .set_level(rfd::MessageLevel::Warning).set_buttons(rfd::MessageButtons::YesNo).show();
        if confirmed != rfd::MessageDialogResult::Yes { return Ok("cancelled".into()); }
        inner.journal.record(Level::Info, Event::InstallStarted);
        let ipa = inner.ipa.as_ref().ok_or("select_ipa_first")?;
        match app_installer::install(&device_manager::SystemRunner, ipa, &id) {
            Ok(()) => { inner.journal.record(Level::Info, Event::InstallVerified); Ok("installed_verified".into()) }
            Err(app_installer::InstallError::VerificationFailed) => {
                inner.journal.record(Level::Warning, Event::InstallUnverified);
                Err("verification_failed_state_unknown".into())
            }
            Err(error) => {
                inner.journal.record(Level::Error, Event::InstallFailed);
                Err(format!("{error:?}; device_state_unknown"))
            }
        }
    }).await.map_err(|_| "worker_failed".to_string())?
}

#[tauri::command]
fn get_logs(state: State<'_, Shared>) -> Result<Vec<diagnostics::Entry>, String> {
    Ok(state
        .lock()
        .map_err(|_| "state_unavailable")?
        .journal
        .entries())
}

#[tauri::command]
async fn export_report(state: State<'_, Shared>) -> Result<bool, String> {
    let report = state
        .lock()
        .map_err(|_| "state_unavailable")?
        .journal
        .export();
    tauri::async_runtime::spawn_blocking(move || {
        let Some(path) = rfd::FileDialog::new()
            .set_file_name("meganugget-report.json")
            .save_file()
        else {
            return Ok(false);
        };
        std::fs::write(path, report).map_err(|_| "report_write_failed".to_string())?;
        Ok(true)
    })
    .await
    .map_err(|_| "worker_failed".to_string())?
}

fn main() {
    tauri::Builder::default()
        .manage(Shared::default())
        .invoke_handler(tauri::generate_handler![
            scan_devices,
            choose_ipa,
            install_ipa,
            get_logs,
            export_report
        ])
        .run(tauri::generate_context!())
        .expect("failed to start MegaNugget");
}

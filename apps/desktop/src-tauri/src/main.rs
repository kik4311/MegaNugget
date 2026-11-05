#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use diagnostics::{Event, Journal, Level};
use futures_util::FutureExt;
use serde::Serialize;
use std::sync::{Arc, Mutex};
use tauri::State;

#[derive(Default)]
struct Inner {
    journal: Journal,
    ipa: Option<app_installer::ValidatedIpa>,
    signing_input: Option<app_installer::SigningInput>,
    signing: apple_signing::SharedSession,
    signing_job: Option<tauri::async_runtime::JoinHandle<()>>,
}
type Shared = Arc<Mutex<Inner>>;

fn signing_active(inner: &Inner) -> Result<bool, String> {
    use apple_signing::Phase;
    let phase = inner
        .signing
        .lock()
        .map_err(|_| "state_unavailable")?
        .status()
        .phase;
    Ok(!matches!(
        phase,
        Phase::Idle | Phase::Signed | Phase::Failed | Phase::Cancelled
    ))
}

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
async fn choose_ipa(
    for_signing: Option<bool>,
    state: State<'_, Shared>,
) -> Result<Option<serde_json::Value>, String> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let mut inner = state.lock().map_err(|_| "state_unavailable")?;
        if signing_active(&inner)? {
            return Err("signing_in_progress".into());
        }
        inner.ipa = None;
        inner.signing_input = None;
        inner.signing = Default::default();
        let Some(path) = rfd::FileDialog::new()
            .add_filter("iOS application", &["ipa"])
            .pick_file()
        else {
            return Ok(None);
        };
        if for_signing.unwrap_or(false) {
            return match app_installer::inspect_for_signing(&path) {
                Ok(input) => {
                    let info =
                        serde_json::to_value(&input.info).map_err(|_| "serialization_failed")?;
                    inner.signing_input = Some(input);
                    inner.journal.record(Level::Info, Event::IpaValidated);
                    Ok(Some(info))
                }
                Err(error) => {
                    inner.journal.record(Level::Warning, Event::IpaRejected);
                    Err(format!("{error:?}"))
                }
            };
        }
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
        if signing_active(&inner)? { return Err("signing_in_progress".into()); }
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

#[derive(Serialize)]
struct SigningView {
    status: apple_signing::Status,
    ipa: Option<serde_json::Value>,
}

#[tauri::command]
fn signing_status(state: State<'_, Shared>) -> Result<SigningView, String> {
    let inner = state.lock().map_err(|_| "state_unavailable")?;
    let status = inner
        .signing
        .lock()
        .map_err(|_| "state_unavailable")?
        .status();
    let ipa = inner
        .ipa
        .as_ref()
        .map(|ipa| serde_json::to_value(&ipa.info))
        .transpose()
        .map_err(|_| "serialization_failed")?;
    Ok(SigningView { status, ipa })
}

#[tauri::command]
async fn begin_signing(
    id: String,
    email: String,
    password: String,
    state: State<'_, Shared>,
) -> Result<bool, String> {
    let password = zeroize::Zeroizing::new(password);
    if !device_manager::valid_udid(&id)
        || email.len() > 254
        || !email.contains('@')
        || password.is_empty()
        || password.len() > 1024
    {
        return Err("invalid_signing_input".into());
    }
    let shared = state.inner().clone();
    let state_for_confirmation = shared.clone();
    let device_id = id.clone();
    let prepared = tauri::async_runtime::spawn_blocking(move || -> Result<_, String> {
        let mut inner = state_for_confirmation.lock().map_err(|_| "state_unavailable")?;
        if signing_active(&inner)? { return Err("signing_in_progress".into()); }
        let input = inner.signing_input.as_ref().ok_or("choose_unsigned_meganugget_first")?;
        let device = device_manager::device_info(&device_manager::SystemRunner, &device_id).map_err(|_| "device_not_readable")?;
        if !apple_signing::version_allowed(&input.info.minimum_ios, &device.ios) { return Err("minimum_ios_not_met".into()); }
        let confirmed = rfd::MessageDialog::new().set_title("MegaNugget — Apple Account signing / Подпись Apple")
            .set_description(format!("Подписать MegaNugget для устройства {device_id}?\n\nБудут обращения к Apple и стороннему anisette-сервису {}. Сервис получает provisioning-данные anisette. Используются неофициальные Apple API; работоспособность не гарантируется.\n\nВ аккаунте могут быть созданы сертификат, App ID, группа и профиль; UDID регистрируется у Apple. Bundle ID получит суффикс Team ID. Автоматического отзыва сертификатов нет.\n\nСертификат/закрытый ключ и anisette-state сохраняются в системном keyring. Пароль и 2FA не сохраняются MegaNugget. Бесплатная подпись обычно действует 7 дней. Отмена не откатывает изменения аккаунта.\n\nThis signs only; installation requires a separate confirmation. Apple and third-party anisette networking, account changes and OS keyring storage are required. No automatic certificate revocation or protection bypass.", apple_signing::ANISETTE_URL))
            .set_level(rfd::MessageLevel::Warning).set_buttons(rfd::MessageButtons::YesNo).show();
        if confirmed != rfd::MessageDialogResult::Yes { return Ok(None); }
        let input = inner.signing_input.take().ok_or("choose_unsigned_meganugget_first")?;
        inner.ipa = None;
        inner.signing = Default::default();
        inner.signing.lock().map_err(|_| "state_unavailable")?.set_phase(apple_signing::Phase::Authenticating);
        inner.journal.record(Level::Info, Event::SigningStarted);
        Ok(Some((input, device.ios, inner.signing.clone())))
    }).await.map_err(|_| "worker_failed")??;
    let Some((input, ios, session)) = prepared else {
        return Ok(false);
    };
    let finished_state = shared.clone();
    let mut inner = shared.lock().map_err(|_| "state_unavailable")?;
    if !Arc::ptr_eq(&inner.signing, &session) || !signing_active(&inner)? {
        return Err("signing_cancelled".into());
    }
    let job = tauri::async_runtime::spawn(async move {
        let operation = tokio::time::timeout(
            std::time::Duration::from_secs(600),
            apple_signing::sign(input, id, ios, email, password, session.clone()),
        );
        let result = match std::panic::AssertUnwindSafe(operation).catch_unwind().await {
            Ok(Ok(result)) => result,
            Ok(Err(_)) => Err("signing_timeout_account_state_unknown"),
            Err(_) => Err("signing_worker_panicked_account_state_unknown"),
        };
        if let Ok(mut inner) = finished_state.lock() {
            if !Arc::ptr_eq(&inner.signing, &session) {
                return;
            }
            match result {
                Ok(ipa) => {
                    inner.ipa = Some(ipa);
                    if let Ok(mut session) = session.lock() {
                        session.set_phase(apple_signing::Phase::Signed);
                    }
                    inner.journal.record(Level::Info, Event::SigningCompleted);
                }
                Err(error) => {
                    if let Ok(mut session) = session.lock() {
                        session.fail(error);
                    }
                    inner.journal.record(Level::Error, Event::SigningFailed);
                }
            }
        }
    });
    inner.signing_job = Some(job);
    Ok(true)
}

#[tauri::command]
fn signing_respond(value: String, state: State<'_, Shared>) -> Result<(), String> {
    let inner = state.lock().map_err(|_| "state_unavailable")?;
    let mut session = inner.signing.lock().map_err(|_| "state_unavailable")?;
    session.respond(value).map_err(str::to_string)
}

#[tauri::command]
fn cancel_signing(state: State<'_, Shared>) -> Result<(), String> {
    let mut inner = state.lock().map_err(|_| "state_unavailable")?;
    if !signing_active(&inner)? {
        return Err("no_active_signing".into());
    }
    if let Some(job) = inner.signing_job.take() {
        job.abort();
    }
    inner
        .signing
        .lock()
        .map_err(|_| "state_unavailable")?
        .set_phase(apple_signing::Phase::Cancelled);
    inner
        .journal
        .record(Level::Warning, Event::SigningCancelled);
    Ok(())
}

#[tauri::command]
async fn export_signed_ipa(state: State<'_, Shared>) -> Result<bool, String> {
    let shared = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let mut inner = shared.lock().map_err(|_| "state_unavailable")?;
        if signing_active(&inner)? {
            return Err("signing_in_progress".into());
        }
        let ipa = inner.ipa.as_ref().ok_or("no_signed_ipa")?;
        let Some(path) = rfd::FileDialog::new()
            .set_title("Private IPA: contains device provisioning / Не публикуйте профиль")
            .set_file_name("MegaNugget-signed.ipa")
            .save_file()
        else {
            return Ok(false);
        };
        ipa.save(&path).map_err(|_| "signed_ipa_export_failed")?;
        inner.journal.record(Level::Info, Event::SignedIpaExported);
        Ok(true)
    })
    .await
    .map_err(|_| "worker_failed".to_string())?
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
            export_report,
            signing_status,
            begin_signing,
            signing_respond,
            cancel_signing,
            export_signed_ipa
        ])
        .run(tauri::generate_context!())
        .expect("failed to start MegaNugget");
}

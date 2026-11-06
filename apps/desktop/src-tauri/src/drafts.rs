//! IPC adapters for the offline tweak drafts: thin command wrappers around
//! `crates/customization`. Errors surface as the crate's stable snake_case
//! codes; there is no transport here and the bundled catalog never changes.

use diagnostics::{Event, Level};
use serde::Serialize;
use std::collections::BTreeMap;
use tauri::State;

/// One headline card for the Tweaks page: counts mirror the row list exactly.
#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
pub struct TweaksCatalogView {
    pub repository: &'static str,
    pub revision: &'static str,
    pub registry_count: usize,
    pub counts: customization::Counts,
    pub entries: Vec<customization::Summary>,
}

fn target(model: &str, system_version: &str, architecture: &str) -> customization::Device {
    // Kept verbatim: gate evaluation already fails closed for malformed input,
    // trimming inputs here would only hide real data problems.
    customization::Device::new(model, system_version, architecture)
}

#[tauri::command]
pub fn tweaks_catalog(
    model: String,
    system_version: String,
    architecture: String,
) -> Result<TweaksCatalogView, String> {
    let device = target(&model, &system_version, &architecture);
    let catalog = customization::catalog();
    Ok(TweaksCatalogView {
        repository: &catalog.repository,
        revision: &catalog.revision,
        registry_count: customization::REGISTRY_COUNT,
        counts: customization::counts(&device),
        entries: customization::summarize(&device),
    })
}

#[tauri::command]
pub fn tweaks_preview(
    model: String,
    system_version: String,
    architecture: String,
    draft: BTreeMap<String, customization::TweakValue>,
    state: State<'_, crate::Shared>,
) -> Result<customization::DraftPreview, String> {
    let device = target(&model, &system_version, &architecture);
    let result = customization::compiler::preview(&draft, &device);
    match result {
        Ok(preview) => {
            state
                .lock()
                .map_err(|_| "state_unavailable")?
                .journal
                .record(Level::Info, Event::DraftCompiled);
            Ok(preview)
        }
        Err(error) => Err(error.code().to_string()),
    }
}

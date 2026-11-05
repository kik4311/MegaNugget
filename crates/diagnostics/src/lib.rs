use serde::Serialize;
use std::collections::VecDeque;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Level {
    Info,
    Warning,
    Error,
}

/// Closed vocabulary: never accept device output, identifiers, paths or user input here.
#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Event {
    ScanCompleted,
    ScanFailed,
    IpaValidated,
    IpaRejected,
    InstallStarted,
    InstallVerified,
    InstallFailed,
    InstallUnverified,
}

#[derive(Debug, Clone, Serialize)]
pub struct Entry {
    pub timestamp: u64,
    pub level: Level,
    pub event: Event,
}

#[derive(Default)]
pub struct Journal {
    entries: VecDeque<Entry>,
}
impl Journal {
    pub fn record(&mut self, level: Level, event: Event) {
        if self.entries.len() == 500 {
            self.entries.pop_front();
        }
        self.entries.push_back(Entry {
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            level,
            event,
        });
    }
    pub fn entries(&self) -> Vec<Entry> {
        self.entries.iter().cloned().collect()
    }
    pub fn export(&self) -> String {
        serde_json::to_string_pretty(&serde_json::json!({
            "schema": 1, "version": env!("CARGO_PKG_VERSION"), "privacy": "allowlisted-events-only",
            "events": self.entries()
        }))
        .expect("serializable journal")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bounded_and_private() {
        let mut j = Journal::default();
        for _ in 0..510 {
            j.record(Level::Info, Event::ScanCompleted);
        }
        assert_eq!(j.entries().len(), 500);
        let report: serde_json::Value = serde_json::from_str(&j.export()).unwrap();
        assert_eq!(report["privacy"], "allowlisted-events-only");
        let event = report["events"][0].as_object().unwrap();
        assert_eq!(event.len(), 3);
        assert!(
            event.contains_key("event")
                && event.contains_key("level")
                && event.contains_key("timestamp")
        );
    }
}

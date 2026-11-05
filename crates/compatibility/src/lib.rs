use serde::Serialize;

#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    ResearchTarget,
    Unverified,
}

#[derive(Debug, Serialize)]
pub struct Compatibility {
    pub status: Status,
    pub jailbreak_available: bool,
    pub reason: &'static str,
}

/// A research target is NOT a supported jailbreak configuration.
pub fn evaluate(product: &str, ios: &str) -> Compatibility {
    let target = matches!(product, "iPhone9,1" | "iPhone9,3") && ios == "15.8.8";
    Compatibility {
        status: if target {
            Status::ResearchTarget
        } else {
            Status::Unverified
        },
        jailbreak_available: false,
        reason: if target {
            "Target hardware/version; no verified jailbreak method."
        } else {
            "This hardware/version combination has not been verified."
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exact_targets_only() {
        for model in ["iPhone9,1", "iPhone9,3"] {
            assert_eq!(evaluate(model, "15.8.8").status, Status::ResearchTarget);
        }
        for (model, version) in [
            ("iPhone9,2", "15.8.8"),
            ("iPhone9,1", "15.8.7"),
            ("iPhone9,1", "16.0"),
            ("iPhone15,2", "17.0"),
            ("", ""),
        ] {
            assert_eq!(evaluate(model, version).status, Status::Unverified);
        }
    }
    #[test]
    fn no_jailbreak_claims() {
        assert!(!evaluate("iPhone9,1", "15.8.8").jailbreak_available);
    }
}

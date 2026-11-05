//! Which public jailbreak component can realistically handle an exact
//! device/OS combination. Rows are documentary data from public upstream
//! project pages and community reports (checked 2026-11); provenance and
//! caveats live in `docs/JAILBREAK_ORCHESTRATION.md`. The matrix intentionally
//! refuses to promise anything for combinations without public evidence.

use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Stability {
    /// Public reports of this exact combination working.
    ReportedWorking,
    /// Public reports of this exact combination misbehaving (bootloops, random reboots).
    ReportedConcerns,
    /// No reliable public evidence found.
    Unverified,
}

/// A documentary plan: what upstream component, which version and which
/// on-device install route are publicly reported for one device class.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct Plan {
    pub product: &'static str,
    pub component: &'static str,
    pub version: &'static str,
    pub exploit: &'static str,
    pub install_route: &'static str,
    pub stability: Stability,
    pub notes: &'static str,
}

const A9: [&str; 3] = ["iPhone8,1", "iPhone8,2", "iPhone8,4"];
const A10: [&str; 4] = ["iPhone9,1", "iPhone9,2", "iPhone9,3", "iPhone9,4"];
const A11: [&str; 6] = [
    "iPhone10,1",
    "iPhone10,2",
    "iPhone10,3",
    "iPhone10,4",
    "iPhone10,5",
    "iPhone10,6",
];

#[derive(Debug, PartialEq, Eq)]
pub enum PlanError {
    /// The catalog deliberately has no plan for this device.
    NoPlan,
    /// A device exists in the catalog but there is no verified public route
    /// for this exact OS; the matrix must not fabricate one.
    UnverifiedRoute,
}

impl std::fmt::Display for PlanError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoPlan => write!(f, "no_plan"),
            Self::UnverifiedRoute => write!(f, "unverified_route"),
        }
    }
}

/// `false` for versions the parser refuses to trust (we never approximate).
fn ios_tuple(ios: &str) -> Option<(u32, u32, u32)> {
    let mut parts = ios.split('.');
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next().unwrap_or("0").parse().ok()?;
    let patch = parts.next().unwrap_or("0").parse().ok()?;
    if parts.next().is_some() || major == 0 {
        return None;
    }
    Some((major, minor, patch))
}

const DARKSWORD_NOTE: &str = "DarkSword does not support A8(X), may not work on A9X and on early iOS 16.x; failures can close the app (upstream 2.5b4 notes)";

fn darksword_plan(product: &'static str, stability: Stability, note: &'static str) -> Plan {
    Plan {
        product,
        component: "Dopamine",
        version: ">= 2.5b4",
        exploit: "DarkSword",
        install_route: "trollstore",
        stability,
        notes: note,
    }
}

const LANDA_NOTE: &str = "landa (kernel exploit of the stable 2.x line) was patched by Apple starting 15.8.7; only valid below that";

fn landa_plan(product: &'static str) -> Plan {
    Plan {
        product,
        component: "Dopamine",
        version: "2.x stable",
        exploit: "landa",
        install_route: "trollstore",
        stability: Stability::ReportedWorking,
        notes: LANDA_NOTE,
    }
}

/// Resolve the publicly reported plan for an exact device/OS combination.
pub fn required_plan(product: &str, ios: &str) -> Result<Plan, PlanError> {
    // An unreadable version cannot be matched honestly at all.
    let Some(version) = ios_tuple(ios) else {
        return Err(PlanError::NoPlan);
    };
    let family = match product {
        p if A9.contains(&p) => "A9 (arm64)",
        p if A10.contains(&p) => "A10 (arm64)",
        p if A11.contains(&p) => "A11 (arm64)",
        _ => return Err(PlanError::NoPlan),
    };

    let ios16 = version.0 == 16;
    let is_15_8_8 = version == (15, 8, 8);

    if ios16 {
        // landa is only publicly evidenced below 16.7 on arm64.
        if version < (16, 7, 0) {
            return Ok(landa_plan(family));
        }
        return Err(PlanError::UnverifiedRoute);
    }
    if version >= (15, 8, 7) {
        if family == "A11 (arm64)" {
            // Public evidence is incomplete for A11 on DarkSword-era firmwares.
            return Err(PlanError::UnverifiedRoute);
        }
        let stability = if is_15_8_8 {
            Stability::ReportedConcerns
        } else {
            Stability::ReportedWorking
        };
        let note = if is_15_8_8 {
            "Upstream 2.5b4 officially covers 15.8.8; community reports bootloops on iPhone 7/15.8.8 even without tweaks, fix described is DFU restore. Treat as risky"
        } else {
            DARKSWORD_NOTE
        };
        return Ok(darksword_plan(family, stability, note));
    }
    Ok(landa_plan(family))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn iphone7_1588_maps_to_darksword_beta_with_bootloop_warning() {
        for product in ["iPhone9,1", "iPhone9,2", "iPhone9,3", "iPhone9,4"] {
            let plan = required_plan(product, "15.8.8").unwrap();
            assert_eq!(plan.exploit, "DarkSword");
            assert_eq!(plan.version, ">= 2.5b4");
            assert_eq!(plan.install_route, "trollstore");
            assert_eq!(plan.stability, Stability::ReportedConcerns);
            assert!(plan.notes.contains("bootloops"));
        }
    }

    #[test]
    fn iphone7_1587_maps_to_darksword_without_bootloop_note() {
        let plan = required_plan("iPhone9,1", "15.8.7").unwrap();
        assert_eq!(plan.exploit, "DarkSword");
        assert_eq!(plan.stability, Stability::ReportedWorking);
    }

    #[test]
    fn below_1587_uses_landa() {
        for (product, ios) in [
            ("iPhone9,1", "15.8.6"),
            ("iPhone8,4", "15.8.1"),
            ("iPhone10,4", "15.0"),
        ] {
            let plan = required_plan(product, ios).unwrap();
            assert_eq!(plan.exploit, "landa");
            assert!(plan.notes.contains("patched by Apple"));
        }
    }

    #[test]
    fn ios16_arm64_uses_landa_up_to_16_6_1_still() {
        assert_eq!(
            required_plan("iPhone9,1", "16.6.1").unwrap().exploit,
            "landa"
        );
        // 16.7+ public arm64 evidence is not encoded; refuse honestly.
        assert_eq!(
            required_plan("iPhone9,1", "16.7.10"),
            Err(PlanError::UnverifiedRoute)
        );
    }

    #[test]
    fn a11_darksword_era_is_refused() {
        assert_eq!(
            required_plan("iPhone10,4", "15.8.8"),
            Err(PlanError::UnverifiedRoute)
        );
    }

    #[test]
    fn unknown_devices_have_no_plan() {
        for product in ["iPhone11,2", "iPhone12,8", "iPad7,1", "iPhone9,99"] {
            assert_eq!(required_plan(product, "15.8.8"), Err(PlanError::NoPlan));
        }
    }

    #[test]
    fn malformed_devices_and_versions_refused() {
        assert_eq!(required_plan("mac-mini", "15.8.8"), Err(PlanError::NoPlan));
        assert_eq!(
            required_plan("iPhone9,1", "15.8.8.1"),
            Err(PlanError::NoPlan)
        );
        assert_eq!(required_plan("", "15.8.8"), Err(PlanError::NoPlan));
    }
}

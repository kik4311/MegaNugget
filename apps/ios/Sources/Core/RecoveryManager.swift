public enum RecoveryManager {
    /// Guidance only: never reboot, restore or modify a device automatically.
    public static let guidance = [
        "No jailbreak method is implemented. MegaNugget has not modified system partitions.",
        "If installation is not verified, inspect the Home Screen before retrying. Do not assume rollback.",
        "If the app does not launch, check the signing profile, expiry and device provisioning in Xcode.",
        "Back up your device before future research. Backups may not restore all data.",
        "For iPhone 7, Apple's force restart uses Side + Volume Down until the Apple logo appears.",
        "Recovery-mode restore can erase data and may only install firmware currently signed by Apple. Consult docs/RECOVERY.md first."
    ]
}

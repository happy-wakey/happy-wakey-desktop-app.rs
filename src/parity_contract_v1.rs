//! Shared feature-parity contract with `happy-wakey/happy-wakey-flutter` and
//! the canonical e2e manifest. Native Qt/Flutter, BLE, notification, and
//! lifecycle behavior belongs only in [`AppPlatformAdapter`].
pub const CROSS_PLATFORM_PARITY_CONTRACT_VERSION: u32 = 1;
pub const FLUTTER_COUNTERPART: &str = "happy-wakey/happy-wakey-flutter";
pub const E2E_PARITY_CONTRACT: &str =
    "happy-wakey/happy-wakey-e2e/contracts/desktop-parity.json";
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum AppSurface { Mobile, FlutterDesktop, RustDesktop }
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum AppCapability {
    Authentication, Home, Calendar, Weather, Markets, News, Planner, Focus,
    Devices, SafeBrowserNavigation, Settings, Reminders, BlePreviewCommands,
    DeepLinks, SecureStorage, OfflineCache, BackgroundSync, Notifications,
    Telemetry, Accessibility, ApplicationUpdates,
}
pub const REQUIRED_PARITY_CAPABILITIES: &[AppCapability] = &[
    AppCapability::Authentication, AppCapability::Home, AppCapability::Calendar,
    AppCapability::Weather, AppCapability::Markets, AppCapability::News,
    AppCapability::Planner, AppCapability::Focus, AppCapability::Devices,
    AppCapability::SafeBrowserNavigation, AppCapability::Settings,
    AppCapability::Reminders, AppCapability::BlePreviewCommands,
    AppCapability::DeepLinks, AppCapability::SecureStorage,
    AppCapability::OfflineCache, AppCapability::BackgroundSync,
    AppCapability::Notifications, AppCapability::Telemetry,
    AppCapability::Accessibility, AppCapability::ApplicationUpdates,
];
pub trait AppPlatformAdapter {
    fn surface(&self) -> AppSurface;
    fn supports(&self, capability: AppCapability) -> bool;
}
pub fn verify_required_parity_capabilities(
    adapter: &impl AppPlatformAdapter,
) -> Result<(), Vec<AppCapability>> {
    let missing = REQUIRED_PARITY_CAPABILITIES.iter().copied()
        .filter(|capability| !adapter.supports(*capability)).collect::<Vec<_>>();
    if missing.is_empty() { Ok(()) } else { Err(missing) }
}

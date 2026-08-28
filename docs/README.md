# Happy Wakey Documentation

This directory documents the product and the implementation in:

`/Users/maca5/codes/happy-wakey/happy-wakey-desktop-app.rs`

The documents describe the code as it exists on July 16, 2026. Planned features are labeled explicitly so they are not confused with shipped behavior.

## Documents

| Document | Purpose |
| --- | --- |
| [Progress](./PROGRESS.md) | What works, what is partial, what was tested, and what remains |
| [Product design](./PRODUCT_DESIGN.md) | UX goals, information architecture, onboarding, theme, and interaction rules |
| [Architecture and tech stack](./ARCHITECTURE_AND_TECH_STACK.md) | How the Rust and Qt desktop process works and why it is cross-platform |
| [Cross-platform distribution](./CROSS_PLATFORM_AND_DISTRIBUTION.md) | Platform builds, packaging, signing, installers, and CI |
| [Calendar notifications and reminders](./CALENDAR_NOTIFICATIONS_AND_REMINDERS.md) | Daily agenda, provider sync, Gmail/Calendly, scheduling, and server decision |
| [Data, security, and services](./DATA_SECURITY_AND_SERVICES.md) | Local JSON, Supabase, RLS, OAuth, external APIs, and secret handling |
| [Development, testing, and roadmap](./DEVELOPMENT_TESTING_AND_ROADMAP.md) | Local setup, test commands, acceptance checks, and implementation priorities |

## Short Answer

Happy Wakey is a native desktop application, not a browser shell. Rust owns
application state, configuration, authentication, network calls, native BLE,
and service integration. Qt 6 Quick/QML renders the interface and `cxx-qt`
generates the typed bridge between Rust and Qt. There is no embedded browser or
webview; safe HTTP/HTTPS links open in the system browser.

The architecture is cross-platform across macOS, Windows, and Linux because
Rust, Qt 6, QML, CXX-Qt, Reqwest, and btleplug support those platforms. The
application must still be compiled and packaged separately on each target
operating system because it links to that platform's Qt runtime, Bluetooth
stack, and native windowing stack.

The current repository builds and runs on macOS. Production installers, signing, notarization, Windows/Linux release validation, native notification support, and automatic Git backup are still roadmap items.

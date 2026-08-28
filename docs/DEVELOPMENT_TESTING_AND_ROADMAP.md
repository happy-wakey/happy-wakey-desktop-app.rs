# Development, Testing, and Roadmap

## Repository

```text
/Users/maca5/codes/happy-wakey/happy-wakey-desktop-app.rs
```

## Prerequisites

- Rust toolchain compatible with the dependency lockfile;
- Qt 6 with Quick and Controls;
- CoreBluetooth, WinRT Bluetooth, or BlueZ/DBus for native BLE acceptance;
- a C++ compiler supported by the installed Qt build;
- platform deployment/signing tools for release packages.

On macOS, Homebrew Qt is suitable for development. Ensure the matching Qt bin and CMake prefix are discoverable before building.

## Configuration

Copy `.env.example` to `.env` for local development and configure only the providers being tested.

Important variables:

| Variable | Purpose |
| --- | --- |
| `SUPABASE_URL` | Supabase project URL |
| `SUPABASE_ANON_KEY` | Public anon/publishable key for auth and REST |
| `OPEN_METEO_BASE_URL` | Free or paid Open-Meteo forecast endpoint |
| `OPEN_METEO_API_KEY` | Paid Open-Meteo customer key |
| `OPENWEATHER_API_KEY` | Optional weather fallback |
| `FINNHUB_API_KEY` | Market quotes |
| `NEWSAPI_KEY` | Headlines |
| `CONFIG_DIR` | Isolated config location for tests |
| `HAPPY_WAKEY_OAUTH_PORT` | OAuth loopback port; default 47217 |
| `HAPPY_WAKEY_BUNDLE_ID` | Stable registered application ID used by macOS notifications |
| `HAPPY_WAKEY_PLATFORM_URL` | Optional HTTPS hostname for shared auth and the gateway; no default |
| `HAPPY_WAKEY_SHARED_AUTH_URL` | Loopback/development override for shared auth |
| `HAPPY_WAKEY_GATEWAY_URL` | Loopback/development override for the product gateway |

Precedence is CLI flag, system environment, `.env`, then built-in default.

## Core Commands

```bash
cargo fmt --check
cargo test --locked
cargo build --locked
cargo run --locked
```

Run the real Open-Meteo smoke test explicitly:

```bash
cargo test open_meteo_live_smoke -- --ignored --nocapture
```

Run QML linting when the Qt toolchain provides it:

```bash
qmllint qml/*.qml
```

The custom QML module is generated during the CXX-Qt build. A standalone lint invocation may warn that `com.happywakey` is not on its import path even when the application build and runtime load correctly.

## Test Strategy

### Unit Tests

Current unit coverage includes:

- config collection limits and sanitization;
- URL and symbol validation;
- secret redaction and merge preservation;
- onboarding normalization and timestamp merge behavior;
- CLI flag parsing;
- Finnhub nullable/missing response fields;
- Open-Meteo parsing and WMO mapping;
- OAuth provider aliases, PKCE, callback/state parsing;
- external URL safety;
- transient HTTP retry against a local test server.
- Google/Microsoft calendar normalization, local week boundaries, all-day semantics, agenda conflicts, and deduplication;
- reminder offset reconciliation, cancellation filtering, ledger retention, and failed-delivery retry state;
- deterministic future-only cloud jobs and HTTPS/loopback service URL enforcement.

### Live Provider Tests

Live tests should be opt-in and use non-production test accounts/keys. They should verify:

- Open-Meteo response shape and forecast completeness;
- Google week fetch with timezone/all-day events;
- Microsoft week fetch and token scopes;
- Finnhub valid, invalid, ETF, and commodity-like symbols;
- NewsAPI keyword enforcement and empty result behavior;
- Supabase RLS with two distinct users.

Never print keys or provider tokens in test output.

### Native UI Acceptance

Test the built desktop executable, not only QML source:

1. Launch with an isolated `CONFIG_DIR`.
2. Complete onboarding using pointer and keyboard.
3. Restart and verify the dashboard opens directly.
4. Verify Home at 900x600 and 1280x860.
5. Open every panel from the sidebar.
6. Trigger refresh twice quickly and verify only one request sweep runs.
7. Verify empty, loading, success, partial failure, and total failure states.
8. Scan for a fixture peripheral, connect, send a preview alarm, disconnect,
   and confirm no credential appears in the GATT payload.
9. Restart and verify configuration persistence.
10. Send a test reminder, change reminder offsets, restart, and verify both native delivery and persisted settings.
11. After sign-in, enable cloud email reminders, refresh Calendar, verify the pending count, and use Test cloud email.
12. Test screen reader names, tab order, high DPI, and reduced motion where applicable.

## Immediate Roadmap

### P0: Production Safety

- Move OAuth/provider tokens to OS credential vaults.
- Add provider token refresh and logout/revocation behavior.
- Add real two-user Supabase RLS integration tests.
- Connect remote config hydration with explicit field merge rules.
- Add structured logging that redacts secrets.
- Add crash-safe service result caching and timestamps.

### P0: Core Promised Features

- Extend the implemented normalized event model, daily agenda, native reminders, validated join/open links, and notification ledger with snooze and notification actions.
- Add Google Calendar incremental sync, Microsoft Graph delta sync, Apple EventKit on macOS, Calendly native OAuth/polling, and optional Gmail invitation discovery.
- Implement real Git backup with redacted config, locking, semantic merge, commit, and push.
- Replace the calendar list with a weekly time grid.
- Decide Apple calendar strategy: platform EventKit on Apple devices, CalDAV with app-specific credentials, or clearly identity-only Apple support.

### P1: Cross-Platform Release

- Add GitHub Actions native build matrix.
- Add macOS app bundle, signing, notarization, and DMG scripts.
- Add Windows deployment and signed MSI/installer.
- Add Linux Flatpak manifest and AppImage fallback.
- Add installed-artifact Bluetooth permission and fixture-device smoke tests.

### P1: Daily-Use Quality

- Persist last successful data with freshness labels.
- Add automatic refresh intervals with per-provider rate budgets.
- Add calendar pagination and multi-provider aggregation.
- Add stock ordering, asset metadata, market state, and sparklines.
- Add news exclusions and source controls.
- Add browser back/forward/reload, session restore, and download policy.
- Add accessible roles/names for custom QML controls.
- Remove `qmllint` unqualified-reference warnings using bound components.

### P2: Operations

- Add privacy policy and provider attribution review.
- Add opt-in crash reporting with data minimization.
- Add signed update channels and rollback.
- Add configuration schema version migrations.
- Add localization and timezone test matrix.

## Definition of Production Ready

The application is production ready when:

- all promised core features work without developer tooling;
- secrets are not stored in plain JSON;
- RLS and OAuth are verified with real test users;
- calendar reminders survive restart and sleep/wake;
- Git backup handles concurrent edits without data loss;
- installed packages pass acceptance tests on supported macOS, Windows, and Linux versions;
- packages and updates are signed;
- provider terms, attribution, and privacy disclosures are complete;
- failures are visible and recoverable without reading logs.

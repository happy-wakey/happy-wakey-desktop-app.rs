# Progress

Status date: August 25, 2026

## Current Product State

Happy Wakey is a working native Rust and Qt desktop prototype with a cohesive
dashboard, onboarding, local configuration, Supabase-backed authentication/sync
foundations, external data panels, and native Bluetooth alarm-device support.
It has no embedded browser or webview. It is beyond a static mockup, but it is
not yet a production-ready signed application.

## Capability Matrix

| Area | Status | Current behavior |
| --- | --- | --- |
| Native desktop shell | Working | Qt 6 application window with QML navigation and panels |
| Home dashboard | Working | Preview cards for calendar, weather, stocks, news, Bluetooth devices, and setup |
| Light daytime theme | Working | Light theme, with a softer low-brightness palette from 5:00 AM to 8:00 AM |
| Onboarding | Working | Five-step flow with local persistence and Supabase sync after login |
| Onboarding controls | Live-tested | Continue and Open Dashboard complete the flow and persist completion |
| Google OAuth | Implemented, configuration required | Supabase PKCE login requests Google calendar read scope |
| Microsoft OAuth | Implemented, configuration required | Microsoft maps to the Supabase Azure provider and requests `Calendars.Read` |
| Apple OAuth | Implemented for identity | Apple login works through Supabase, but Apple Sign-In does not provide a calendar API |
| Weekly calendar data | Working, credentials required | Google Calendar and Microsoft Graph normalize the current week into day groups, all-day/timed rows, conflict counts, and validated join/open links |
| Daily agenda and reminders | Working, macOS live-tested | Home and Calendar show agenda summaries; the local scheduler supports configurable 30/10/5-minute native reminders with a persistent deduplication ledger |
| Off-app email reminders | Implemented, deployment pending | Opt-in deterministic reminder reconciliation through shared auth, a durable Rust gateway, NATS request/reply, and the existing SendGrid contact service |
| Gmail invitation discovery | Planned | Calendar API is primary; Gmail polling is optional for invitations not yet represented as events |
| Calendly sync | Planned | Native OAuth/polling can remain serverless; webhooks require a public relay |
| Weather | Working and live-tested | Up to five locations, current conditions, five-day forecast, and radar links |
| Weather provider | Working | Open-Meteo is primary; OpenWeather is an optional fallback |
| Stocks/watchlist | Implemented, key required | Up to 20 Finnhub symbols, one quote request per symbol |
| News | Implemented, key required | NewsAPI query followed by local keyword enforcement, URL validation, deduplication, and a five-item cap |
| Bluetooth devices | Implemented, hardware acceptance pending | Native filtered BLE scan, connect/disconnect, and versioned bounded preview-alarm write |
| External links | Working | HTTP/HTTPS validation followed by system-browser launch; no embedded webview |
| Local configuration | Working | Sanitized JSON with atomic replacement and restrictive Unix permissions |
| Supabase config mirror | Partial | Saves a redacted config snapshot; broader remote config hydration is not wired into startup |
| Supabase onboarding state | Working in code | Dedicated table and per-user REST reads/upserts; live project access still requires credentials |
| Supabase RLS schema | Implemented declaratively | Idempotent SQL enables and forces RLS for config and onboarding tables |
| Git backup | Not implemented | The repository/path is collected in onboarding and Settings, but no clone/commit/push engine exists |
| Production packaging | Planned | No checked-in DMG/MSI/AppImage/Flatpak pipeline yet |
| Automatic updates | Planned | No update channel or signed updater yet |

## Recent Improvement Pass

The July 2026 modernization pass added or changed the following:

- Added a bounded shared HTTP layer with connection timeout, request timeout, connection pooling, limited redirects, transient GET retries, and a 2 MiB JSON response cap.
- Prevented API keys from appearing in user-facing request URL errors.
- Added structured provider error extraction with bounded, control-character-free messages.
- Added Open-Meteo current conditions and five-day forecasts with WMO weather-code mapping.
- Kept OpenWeather as a fallback when a key is configured.
- Parallelized weather fetches across the user's five locations.
- Reduced Finnhub usage from two calls per stock to one call per stock by removing repeated cosmetic company-profile requests.
- Added loading state and duplicate-refresh suppression for calendar, weather, stocks, and news.
- Added explicit partial-success reporting when some locations or symbols fail.
- Improved NewsAPI handling with a larger candidate set, local keyword matching, URL validation, duplicate suppression, and invalid-image filtering.
- Fixed `.env` loading. The previous code created a dotenv iterator without applying values to the process.
- Made Supabase config calls fail clearly when `SUPABASE_ANON_KEY` is absent.
- Rebuilt the Weather screen around scan-friendly current conditions and a five-day strip.
- Fixed unstable Home and Weather grid sizing inside `ScrollView`, which had caused compressed and overlapping content.
- Added Open-Meteo attribution and separate free/paid endpoint settings.
- Corrected the calendar week window to use local Monday midnight through the following Monday and normalized Google/Microsoft all-day, canceled, location, meeting-link, and provider-link fields.
- Added a daily agenda model with today's remaining events, meeting minutes, next event, and overlap counts for Home and Calendar.
- Added a native reminder worker with configurable offsets, late-refresh reconciliation, cancellation/all-day filtering, a 31-day atomic deduplication ledger, and retry after OS delivery failure.
- Added macOS application identity handling so notifications fail clearly when the app is not running from a registered bundle.
- Updated `quinn-proto` to `0.11.15` for `RUSTSEC-2026-0185` and aligned the CXX runtime/code generator at ABI `1.0.195` for `RUSTSEC-2026-0202` without forcing a broader CXX-Qt migration.
- Added an in-memory shared-auth token exchange/cache and kept service credentials out of desktop JSON.
- Added opt-in cloud email reminders, deterministic reconciliation IDs, bounded payloads, and a Settings test action.
- Added a generated OpenAPI 3.1 Rust gateway with shared-auth introspection, verified-email targeting, PVC-backed atomic JSON state, retry/recovery, and Prometheus metrics.
- Upgraded the contact email NATS consumer to request/reply so the gateway records delivery only after a matching idempotency key and successful provider outcome.

## Verification Performed

The following checks passed during the modernization pass:

- Desktop `cargo test --locked`: 51 tests passed; one network test remained ignored by default.
- Desktop `cargo clippy --all-targets --locked -- -D warnings`: passed.
- Gateway `cargo test --locked` and `cargo clippy --all-targets --locked -- -D warnings`: passed.
- Contact service `cargo check --locked` and `cargo clippy --all-targets --locked -- -D warnings`: passed.
- Kubernetes Kustomize rendering passed for the runtime and observability overlays; focused Node contract tests passed.
- `cargo audit`: no vulnerabilities or informational warnings in the resolved 292-package graph.
- `cargo test open_meteo_live_smoke -- --ignored`: passed against the real Open-Meteo API.
- Local HTTP retry test: a temporary server returned `503` and then `200`; the client recovered and parsed the second response.
- `cargo build`: produced the native macOS debug executable.
- `qmllint qml/*.qml`: no parse errors. The existing code still has legacy unqualified-property warnings.
- Live native QA: launched as a temporary macOS `.app`, opened Weather, loaded three locations concurrently, rendered five forecast days, and accepted Refresh clicks.
- Live onboarding QA: completed onboarding and persisted `completed: true`, `current_step: "complete"`, starter weather, stocks, and news choices.
- Live calendar QA: fetched deterministic Google-shaped events over a loopback fixture, rendered all-day/timed groups and conflict totals, and exposed validated Join/Open actions.
- Live reminder QA: delivered a native notification from a registered macOS app bundle, saved custom offsets, restarted, and restored the selected reminder settings.
- Live cloud-reminder UI QA: a fresh native macOS bundle completed all onboarding Continue actions, opened the dashboard, rendered the new reminder controls, kept cloud actions unavailable while signed out, persisted mode-`0600` config, and reopened directly to Home.

Finnhub, NewsAPI, authenticated Supabase calls, and end-to-end cloud reminder delivery were not live-tested because their keys were not configured in the test environment. The public platform TLS endpoint was reachable, but shared auth returned HTTP 500 through nginx and must be restored before deployment acceptance. The gateway manifests reconciled in AWS, but the cluster had no registered EBS CSI driver for the shared `dd-block` class, no shared-auth Argo application, no provider-credentials secret, and no node-role read grant for that secret path. Provider parsing, validation, and control flow compile and are covered where practical by unit tests.

## Known Gaps

1. Calendar UX still needs a full weekly time grid, provider pagination/delta tokens, token refresh, and simultaneous multi-account aggregation.
2. Reminders still need snooze/actions, a durable event cache, wake/login lifecycle integration, installed-package verification on Windows/Linux, and JetStream/contact-worker idempotency for crash-safe cloud delivery.
3. OAuth/session tokens should move from JSON into the OS credential vault.
4. Git backup needs a real repository lifecycle and conflict policy.
5. Supabase should hydrate the redacted config snapshot at startup and define field-level merge semantics.
6. News and market providers need cache/refresh policies and optional alternate providers.
7. Bluetooth needs real Happy Wakey peripheral firmware fixtures and physical
   CoreBluetooth, WinRT, and BlueZ acceptance evidence.
8. QML should be moved toward bound components and qualified references to remove `qmllint` warnings.
9. Windows and Linux builds need real CI and installer acceptance tests.
10. Production builds need signing, notarization, update delivery, telemetry/privacy decisions, and crash reporting.

## Primary Product Goal

The next major milestone is not merely displaying a calendar. Happy Wakey should help the user tackle the day through a morning agenda, dependable upcoming-event notifications, snooze/join/open actions, and consistent sync across Google Calendar, Microsoft 365/Outlook, Apple calendars, Calendly, and relevant Gmail invitations. See [Calendar notifications and reminders](./CALENDAR_NOTIFICATIONS_AND_REMINDERS.md) for the target architecture.

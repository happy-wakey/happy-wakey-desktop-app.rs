# happy-wakey-desktop-app.rs

A cross-platform Rust desktop app for calendar, weather, markets, news, native
Bluetooth alarm hardware, and frequently used external links. This repository
revives the complete history of `happy-wakey.rs` under the organization-wide
`*-desktop-app.rs` convention. The interface is native Qt/QML, the application
core is Rust, and Supabase provides optional auth and config sync. It contains
no React surface, embedded browser, or webview. URLs open in the user's system
browser. Local reminders need no Happy Wakey server; opt-in off-app email
reminders use the Shared Auth-backed product gateway.

Dependencies and repository scripts are declared in `.zpkg.toml`; use the released `zed-pkg` CLI as the dependency-management entry point.

## Prerequisites

- **Rust** 1.89 (install via [rustup](https://rustup.rs))
- **Qt 6** Quick and Controls (installed via Homebrew, or system package manager)
- On macOS: `brew install qt@6` (ensure `qmake6` is in PATH)

## Quick Start

```bash
# Clone and enter the project
git clone https://github.com/happy-wakey/happy-wakey-desktop-app.rs.git && cd happy-wakey-desktop-app.rs

# Copy the local-only env template and fill in runtime credentials
cp .env.example .env

# Build and run
cargo run
```

## Configuration

Priority (highest to lowest):

1. **CLI flags** — only non-secret settings declared in `.cli-flags.toml`
2. **System environment variables**
3. **`.env` file** — key=value pairs in project root
4. **Schema defaults** — service URLs only; credentials never have defaults

The executable uses the bundled Rust client from canonical
`flags-2-env/flags-2-env` commit
`b07214e30b4da675a0f591e362a2039cb47e9055`. Unknown or invalid options fail
before Qt and worker threads start. Release packaging must place
`.cli-flags.toml` beside the executable or in the macOS `Resources` directory.

### CLI flags

| Flag | Env var | Short | Description |
|------|---------|-------|-------------|
| `--supabase-url` | `SUPABASE_URL` | `-s` | Supabase project URL |
| `--open-meteo-base-url` | `OPEN_METEO_BASE_URL` | | Open-Meteo endpoint |
| `--git-repo` | `GIT_REPO_PATH` | | Path to git config backup |
| `--config-dir` | `CONFIG_DIR` | | Override config directory |
| `--platform-url` | `HAPPY_WAKEY_PLATFORM_URL` | | Platform base URL for shared-auth and gateway. No default; HTTPS hostname only |
| `--shared-auth-url` | `HAPPY_WAKEY_SHARED_AUTH_URL` | | Development override for shared auth |
| `--happy-wakey-gateway-url` | `HAPPY_WAKEY_GATEWAY_URL` | | Development override for the Happy Wakey gateway |

Credentials such as `SUPABASE_ANON_KEY`, provider API keys, and access tokens
are environment/secret-store only. Passing a secret-shaped CLI option fails
without echoing its value. The `.env` file is ignored by Git and is appropriate
for local development only.

## External services

- **Weather:** Open-Meteo supplies current conditions and a five-day forecast without a key for eligible non-commercial use. OpenWeather is used as a fallback when `OPENWEATHER_API_KEY` is set. Commercial distributions should use an Open-Meteo paid customer endpoint and key.
- **Markets:** Finnhub supplies quotes and company profiles. Set `FINNHUB_API_KEY`.
- **News:** NewsAPI supplies up to five keyword-matched headlines. Set `NEWSAPI_KEY`.
- **Calendar:** Google Calendar and Microsoft Graph use provider OAuth tokens obtained through Supabase login.
- **Reminders:** a local Rust scheduler delivers configurable desktop alerts and persists a deduplication ledger; macOS builds require a stable registered `HAPPY_WAKEY_BUNDLE_ID`.
- **Off-app reminders:** an opt-in setting reconciles future calendar reminders to the Happy Wakey gateway. The desktop exchanges its Supabase token for a short-lived shared-auth token; the gateway derives the verified email from that identity and delegates delivery through the contact service.
- **Bluetooth:** native BLE discovery is filtered to the Happy Wakey service UUID. The Devices panel can connect, disconnect, and send a bounded versioned preview-alarm command over the product command characteristic. BLE payloads never contain Shared Auth credentials or customer identifiers.

All GET integrations share a pooled HTTP client with connection and request timeouts, bounded JSON responses, limited redirects, and retries for transient failures. API keys are sent in headers where the provider supports it.

## Supabase OAuth Setup

Configure Google, Apple, and Microsoft as upstream identity providers in the
Supabase project, and restrict Happy Wakey cloud operations to the Shared Auth
token exchange enforced by the product gateway.

## Project Structure

```
src/
  main.rs              # Entry point, Backend QObject, Qt event loop
  config.rs            # Local config (JSON in ~/.config/happy-wakey/)
  env_config.rs        # canonical bundled flags2env runtime boundary
  gateway.rs           # Shared-auth exchange + off-app reminder reconciliation
  reminders.rs         # Native reminder scheduler + delivery ledger
  supabase.rs          # PKCE OAuth login flow
  supabase_config.rs   # Config sync to Supabase REST API
  services/
    calendar.rs        # Google Calendar + Outlook via OAuth tokens
    weather.rs         # Open-Meteo + OpenWeather fallback
    stocks.rs          # Finnhub
    news.rs            # NewsAPI
qml/
  MainWindow.qml       # Sidebar nav + status bar
  CalendarPanel.qml    # Weekly calendar view
  WeatherPanel.qml     # Weather cards
  StocksPanel.qml      # Stock watchlist
  NewsPanel.qml        # News feed
  DevicesPanel.qml     # Native BLE discovery and alarm-device controls
  SettingsPanel.qml    # Auth buttons, bookmarks, config
```

## Tests

```bash
cargo test

# Explicit live smoke test against Open-Meteo
cargo test open_meteo_live_smoke -- --ignored
```

## License

MIT

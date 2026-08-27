mod bluetooth;
mod config;
mod destinations;
mod env_config;
mod gateway;
mod http;
mod reminders;
mod services;
mod supabase;
mod supabase_config;
mod url_safety;

use core::pin::Pin;
use cxx_qt::Threading;
use cxx_qt_lib::QString;

// ---------------------------------------------------------------------------
// Rust <-> Qt bridge
//
// cxx-qt generates a real QObject from `BackendRust`. Properties are typed and
// auto-emit `<name>Changed`; invokables are typed methods. Background work is
// marshalled back to the GUI thread with `qt_thread().queue(...)` — no unsafe
// pointers, no manual command channel, no polling timer.
// ---------------------------------------------------------------------------
#[cxx_qt::bridge]
mod qobject {
    extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
    }

    unsafe extern "C++" {
        include!("webengine_shim.h");
        #[rust_name = "init_web_engine"]
        fn happy_init_web_engine();
    }

    extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[qml_singleton]
        // Auth
        #[qproperty(bool, logged_in)]
        #[qproperty(QString, user_email)]
        #[qproperty(QString, user_id)]
        // Feed data (JSON payloads, parsed in QML)
        #[qproperty(QString, calendar_json)]
        #[qproperty(QString, calendar_agenda_json)]
        #[qproperty(bool, calendar_loading)]
        #[qproperty(QString, weather_json)]
        #[qproperty(bool, weather_loading)]
        #[qproperty(QString, stocks_json)]
        #[qproperty(bool, stocks_loading)]
        #[qproperty(QString, news_json)]
        #[qproperty(bool, news_loading)]
        // Config + onboarding
        #[qproperty(QString, app_config_json)]
        #[qproperty(QString, onboarding_json)]
        // Status bar
        #[qproperty(QString, status_msg)]
        type Backend = super::BackendRust;

        #[qinvokable]
        fn startup(self: Pin<&mut Backend>);
        #[qinvokable]
        fn login(self: Pin<&mut Backend>, provider: &QString);
        #[qinvokable]
        fn logout(self: Pin<&mut Backend>);
        #[qinvokable]
        fn refresh_calendar(self: Pin<&mut Backend>);
        #[qinvokable]
        fn test_notification(self: Pin<&mut Backend>);
        #[qinvokable]
        fn test_cloud_notification(self: Pin<&mut Backend>);
        #[qinvokable]
        fn refresh_weather(self: Pin<&mut Backend>);
        #[qinvokable]
        fn refresh_stocks(self: Pin<&mut Backend>);
        #[qinvokable]
        fn refresh_news(self: Pin<&mut Backend>);
        #[qinvokable]
        fn save_config(self: Pin<&mut Backend>, json: &QString);
        #[qinvokable]
        fn save_onboarding_state(
            self: Pin<&mut Backend>,
            step: &QString,
            step_index: i32,
            completed: bool,
        );
        #[qinvokable]
        fn open_url(self: Pin<&mut Backend>, url: &QString);
        #[qinvokable]
        fn set_status(self: Pin<&mut Backend>, msg: &QString);
        #[qinvokable]
        fn reload_config(self: Pin<&mut Backend>);
    }

    impl cxx_qt::Threading for Backend {}
}

// Convenient alias for the generated QObject type.
use qobject::Backend;
type BackendThread = cxx_qt::CxxQtThread<Backend>;

// The Rust-side state backing the QObject's properties.
pub struct BackendRust {
    logged_in: bool,
    user_email: QString,
    user_id: QString,
    calendar_json: QString,
    calendar_agenda_json: QString,
    calendar_loading: bool,
    weather_json: QString,
    weather_loading: bool,
    stocks_json: QString,
    stocks_loading: bool,
    news_json: QString,
    news_loading: bool,
    app_config_json: QString,
    onboarding_json: QString,
    status_msg: QString,
}

impl Default for BackendRust {
    fn default() -> Self {
        // env_config::init() runs in main() before the QML engine creates this
        // singleton, so config::load() already sees the resolved environment.
        let cfg = config::load();
        let email = cfg
            .supabase_session
            .as_ref()
            .and_then(|s| s.email.clone())
            .unwrap_or_default();
        Self {
            logged_in: cfg.supabase_session.is_some(),
            user_email: QString::from(email.as_str()),
            user_id: QString::from(cfg.user_id.as_str()),
            calendar_json: json_qstring(&Vec::<services::calendar::CalendarEvent>::new()),
            calendar_agenda_json: json_qstring(&services::calendar::build_agenda(&[])),
            calendar_loading: false,
            weather_json: QString::default(),
            weather_loading: false,
            stocks_json: QString::default(),
            stocks_loading: false,
            news_json: QString::default(),
            news_loading: false,
            app_config_json: serialize_ui_config(&cfg),
            onboarding_json: serialize_onboarding(&cfg.onboarding),
            status_msg: QString::default(),
        }
    }
}

// ---------------------------------------------------------------------------
// Invokable implementations
// ---------------------------------------------------------------------------
impl qobject::Backend {
    /// Called once from MainWindow.qml on completion: pull onboarding state from
    /// Supabase (if signed in) and merge it in.
    fn startup(self: Pin<&mut Self>) {
        let cfg = config::load();
        reminders::update_settings(cfg.reminder_settings.clone());
        reminders::start_worker();
        hydrate_onboarding(self, &cfg);
    }

    fn login(self: Pin<&mut Self>, provider: &QString) {
        let provider = provider.to_string();
        match supabase::normalize_provider(&provider) {
            Ok(p) => {
                let thread = self.qt_thread();
                std::thread::spawn(move || {
                    let result = supabase::login_with_provider(&p);
                    thread.queue(move |b| on_login_result(b, result)).ok();
                });
            }
            Err(e) => emit_status(self, e),
        }
    }

    fn logout(mut self: Pin<&mut Self>) {
        let mut cfg = config::load();
        cfg.supabase_session = None;
        gateway::clear_session();
        if let Err(e) = config::save(&cfg) {
            emit_status(self.as_mut(), format!("Logout config save failed: {e}"));
        }
        reminders::replace_events(Vec::new(), cfg.reminder_settings.clone());
        apply_config_snapshot(self.as_mut(), &cfg);
        emit_status(self, "Logged out".to_string());
    }

    fn refresh_calendar(mut self: Pin<&mut Self>) {
        if *self.calendar_loading() {
            return;
        }
        let cfg = config::load();
        let Some(session) = cfg.supabase_session.clone() else {
            emit_status(self, "Sign in before refreshing calendars".to_string());
            return;
        };

        // Calendar APIs need the *provider's* OAuth token, not the Supabase JWT.
        let Some(provider_token) = session
            .provider_token
            .clone()
            .filter(|t| !t.trim().is_empty())
        else {
            emit_status(
                self,
                "Calendar access wasn't granted at sign-in. Sign out and sign back in to allow calendar access.".to_string(),
            );
            return;
        };

        let provider = session.provider.clone();
        let reminder_settings = cfg.reminder_settings.clone();
        let supabase_access_token = session.access_token.clone();
        self.as_mut().set_calendar_loading(true);
        emit_status(self.as_mut(), "Refreshing calendar...".to_string());
        let thread = self.qt_thread();
        std::thread::spawn(move || {
            let result = match provider.as_str() {
                "google" => services::calendar::fetch_google_events(&provider_token),
                "azure" => services::calendar::fetch_outlook_events(&provider_token),
                "apple" => Err("Apple sign-in doesn't provide a calendar API".to_string()),
                other => Err(format!(
                    "Calendar sync isn't supported for '{other}' sign-in"
                )),
            }
            .map(|events| {
                let agenda = services::calendar::build_agenda(&events);
                reminders::replace_events(events.clone(), reminder_settings.clone());
                let cloud_result = if reminder_settings.cloud_email_enabled {
                    Some(gateway::sync_calendar_reminders(
                        &supabase_access_token,
                        &events,
                        &reminder_settings,
                    ))
                } else {
                    None
                };
                (events, agenda, cloud_result)
            });
            thread
                .queue(move |mut b| {
                    b.as_mut().set_calendar_loading(false);
                    match result {
                        Ok((events, agenda, cloud_result)) => {
                            let count = events.len();
                            b.as_mut().set_calendar_json(json_qstring(&events));
                            b.as_mut().set_calendar_agenda_json(json_qstring(&agenda));
                            let message = match cloud_result {
                                Some(Ok(result)) => format!(
                                    "Calendar updated: {count} events; {} cloud reminder(s) pending",
                                    result.pending
                                ),
                                Some(Err(error)) => format!(
                                    "Calendar updated: {count} events; cloud reminder sync failed: {error}"
                                ),
                                None => format!("Calendar updated: {count} events"),
                            };
                            emit_status(b, message);
                        }
                        Err(e) => emit_status(b, format!("Calendar refresh failed: {e}")),
                    }
                })
                .ok();
        });
    }

    fn test_notification(self: Pin<&mut Self>) {
        let thread = self.qt_thread();
        std::thread::spawn(move || {
            let result = reminders::show_test_notification();
            thread
                .queue(move |b| match result {
                    Ok(()) => emit_status(b, "Test reminder sent".to_string()),
                    Err(error) => emit_status(b, error),
                })
                .ok();
        });
    }

    fn test_cloud_notification(self: Pin<&mut Self>) {
        let cfg = config::load();
        let Some(session) = cfg.supabase_session else {
            emit_status(self, "Sign in before testing cloud reminders".to_string());
            return;
        };
        let thread = self.qt_thread();
        std::thread::spawn(move || {
            let result = gateway::queue_test_reminder(&session.access_token);
            thread
                .queue(move |b| match result {
                    Ok(()) => emit_status(b, "Cloud reminder queued".to_string()),
                    Err(error) => emit_status(b, format!("Cloud reminder failed: {error}")),
                })
                .ok();
        });
    }

    fn refresh_weather(mut self: Pin<&mut Self>) {
        if *self.weather_loading() {
            return;
        }
        let cfg = config::load();
        let locs = cfg.weather_locations.clone();
        if locs.is_empty() {
            self.as_mut()
                .set_weather_json(json_qstring(&Vec::<services::weather::WeatherData>::new()));
            emit_status(self, "Add a weather location in Settings".to_string());
            return;
        }

        self.as_mut().set_weather_loading(true);
        emit_status(
            self.as_mut(),
            format!("Refreshing weather for {} location(s)...", locs.len()),
        );
        let thread = self.qt_thread();
        std::thread::spawn(move || {
            let results = std::thread::scope(|scope| {
                let handles: Vec<_> = locs
                    .into_iter()
                    .map(|loc| {
                        scope.spawn(move || {
                            let name = loc.name.clone();
                            (
                                name,
                                services::weather::fetch_weather(loc.lat, loc.lon, &loc.name),
                            )
                        })
                    })
                    .collect();
                handles
                    .into_iter()
                    .map(|handle| handle.join())
                    .collect::<Vec<_>>()
            });

            let mut data = Vec::new();
            let mut errors = Vec::new();
            for result in results {
                match result {
                    Ok((_, Ok(weather))) => data.push(weather),
                    Ok((name, Err(error))) => errors.push(format!("{name}: {error}")),
                    Err(_) => errors.push("A weather worker stopped unexpectedly".to_string()),
                }
            }
            thread
                .queue(move |mut b| {
                    b.as_mut().set_weather_json(json_qstring(&data));
                    b.as_mut().set_weather_loading(false);
                    let message = if errors.is_empty() {
                        format!("Weather updated: {} location(s)", data.len())
                    } else if data.is_empty() {
                        format!("Weather refresh failed: {}", errors.join("; "))
                    } else {
                        format!(
                            "Weather updated for {}; {} failed: {}",
                            data.len(),
                            errors.len(),
                            errors.join("; ")
                        )
                    };
                    emit_status(b, message);
                })
                .ok();
        });
    }

    fn refresh_stocks(mut self: Pin<&mut Self>) {
        // A stocks refresh is a sequential sweep of up to 20 symbols (2 requests
        // each). Guard against overlapping sweeps — a second trigger while one is
        // in flight is a no-op rather than another ~40-request burst (which also
        // risks tripping Finnhub's free-tier rate limit).
        if *self.stocks_loading() {
            return;
        }
        let cfg = config::load();
        let syms = cfg.stock_symbols.clone();
        if syms.is_empty() {
            emit_status(self, "Add a stock symbol in Settings".to_string());
            return;
        }
        self.as_mut().set_stocks_loading(true);
        emit_status(
            self.as_mut(),
            format!("Refreshing {} market symbol(s)...", syms.len()),
        );
        let thread = self.qt_thread();
        std::thread::spawn(move || {
            let mut data = Vec::new();
            let mut errors = Vec::new();
            for sym in &syms {
                match services::stocks::fetch_stock(sym) {
                    Ok(s) => data.push(s),
                    Err(e) => errors.push(format!("{sym}: {e}")),
                }
            }
            thread
                .queue(move |mut b| {
                    b.as_mut().set_stocks_json(json_qstring(&data));
                    b.as_mut().set_stocks_loading(false);
                    let message = if errors.is_empty() {
                        format!("Markets updated: {} symbol(s)", data.len())
                    } else if data.is_empty() {
                        format!("Market refresh failed: {}", errors.join("; "))
                    } else {
                        format!(
                            "Markets updated for {}; {} failed",
                            data.len(),
                            errors.len()
                        )
                    };
                    emit_status(b, message);
                })
                .ok();
        });
    }

    fn refresh_news(mut self: Pin<&mut Self>) {
        if *self.news_loading() {
            return;
        }
        let cfg = config::load();
        let kw = cfg.news_keywords.clone();
        self.as_mut().set_news_loading(true);
        emit_status(self.as_mut(), "Refreshing headlines...".to_string());
        let thread = self.qt_thread();
        std::thread::spawn(move || {
            let result = services::news::fetch_news(&kw);
            thread
                .queue(move |mut b| {
                    b.as_mut().set_news_loading(false);
                    match result {
                        Ok(news) => {
                            let count = news.len();
                            b.as_mut().set_news_json(json_qstring(&news));
                            emit_status(b, format!("Headlines updated: {count} matched"));
                        }
                        Err(e) => emit_status(b, format!("News refresh failed: {e}")),
                    }
                })
                .ok();
        });
    }

    fn save_config(mut self: Pin<&mut Self>, json: &QString) {
        let s = json.to_string();
        match serde_json::from_str::<config::Config>(&s) {
            Ok(incoming) => {
                let current = config::load();
                let cfg = config::merge_editable_config(current, incoming);
                match config::save(&cfg) {
                    Ok(()) => {
                        reminders::update_settings(cfg.reminder_settings.clone());
                        sync_config_to_supabase(&cfg, self.qt_thread());
                        apply_config_snapshot(self.as_mut(), &cfg);
                        emit_status(self, "Config saved".to_string());
                    }
                    Err(e) => emit_status(self, format!("Config save failed: {e}")),
                }
            }
            Err(_) => emit_status(self, "Invalid config JSON".to_string()),
        }
    }

    fn save_onboarding_state(
        self: Pin<&mut Self>,
        step: &QString,
        step_index: i32,
        completed: bool,
    ) {
        let cfg =
            config::set_onboarding_state(config::load(), &step.to_string(), step_index, completed);
        match config::save(&cfg) {
            Ok(()) => {
                sync_onboarding_to_supabase(&cfg, self.qt_thread());
                apply_config_snapshot(self, &cfg);
            }
            Err(e) => emit_status(self, format!("Onboarding save failed: {e}")),
        }
    }

    fn open_url(self: Pin<&mut Self>, url: &QString) {
        match safe_external_url(&url.to_string()) {
            Ok(url) => {
                if let Err(e) = webbrowser::open(url.as_str()) {
                    emit_status(self, format!("Failed to open URL: {e}"));
                }
            }
            Err(e) => emit_status(self, e),
        }
    }

    fn set_status(self: Pin<&mut Self>, msg: &QString) {
        emit_status(self, msg.to_string());
    }

    fn reload_config(mut self: Pin<&mut Self>) {
        let cfg = config::load();
        apply_config_snapshot(self.as_mut(), &cfg);
        emit_status(self, "Config reloaded".to_string());
    }
}

// ---------------------------------------------------------------------------
// Helpers operating on the pinned QObject (run on the GUI thread)
// ---------------------------------------------------------------------------
fn emit_status(mut b: Pin<&mut Backend>, msg: String) {
    b.as_mut().set_status_msg(QString::from(msg.as_str()));
}

fn json_qstring<T: serde::Serialize>(value: &T) -> QString {
    QString::from(serde_json::to_string(value).unwrap_or_default().as_str())
}

fn serialize_ui_config(config: &config::Config) -> QString {
    json_qstring(&config::sync_safe_config(config))
}

fn serialize_onboarding(state: &config::OnboardingState) -> QString {
    json_qstring(state)
}

fn apply_config_snapshot(mut b: Pin<&mut Backend>, cfg: &config::Config) {
    b.as_mut().set_logged_in(cfg.supabase_session.is_some());
    b.as_mut().set_user_id(QString::from(cfg.user_id.as_str()));
    let email = cfg
        .supabase_session
        .as_ref()
        .and_then(|s| s.email.clone())
        .unwrap_or_default();
    b.as_mut().set_user_email(QString::from(email.as_str()));
    b.as_mut().set_app_config_json(serialize_ui_config(cfg));
    b.as_mut()
        .set_onboarding_json(serialize_onboarding(&cfg.onboarding));
}

fn on_login_result(mut b: Pin<&mut Backend>, result: Result<supabase::SupabaseSession, String>) {
    match result {
        Ok(session) => {
            let mut cfg = config::load();
            cfg.user_id = session.user_id.clone();
            cfg.supabase_session = Some(config::SupabaseSession {
                access_token: session.access_token,
                refresh_token: session.refresh_token,
                expires_at: session.expires_at,
                user_id: session.user_id,
                email: session.email,
                provider: session.provider,
                provider_token: session.provider_token,
                provider_refresh_token: session.provider_refresh_token,
            });
            if let Err(e) = config::save(&cfg) {
                emit_status(
                    b.as_mut(),
                    format!("Login saved locally, but config save failed: {e}"),
                );
            }
            sync_config_to_supabase(&cfg, b.qt_thread());
            hydrate_onboarding(b.as_mut(), &cfg);
            apply_config_snapshot(b.as_mut(), &cfg);
            emit_status(b, "Logged in".to_string());
        }
        Err(e) => emit_status(b, format!("Login failed: {e}")),
    }
}

fn on_onboarding_synced(mut b: Pin<&mut Backend>, remote_state: config::OnboardingState) {
    let mut cfg = config::load();
    cfg.onboarding = remote_state;
    match config::save(&cfg) {
        Ok(()) => {
            apply_config_snapshot(b.as_mut(), &cfg);
            sync_onboarding_to_supabase(&cfg, b.qt_thread());
        }
        Err(e) => emit_status(b, format!("Onboarding sync save failed: {e}")),
    }
}

// ---------------------------------------------------------------------------
// Background sync helpers (own thread; report failures back via the GUI thread)
// ---------------------------------------------------------------------------
fn hydrate_onboarding(b: Pin<&mut Backend>, cfg: &config::Config) {
    if !cfg.supabase_sync_enabled {
        return;
    }
    let Some(session) = cfg.supabase_session.as_ref() else {
        return;
    };
    let access_token = session.access_token.clone();
    let local_state = cfg.onboarding.clone();
    let thread = b.qt_thread();
    std::thread::spawn(
        move || match supabase_config::fetch_onboarding_state(&access_token) {
            Ok(Some(remote)) => {
                let merged = config::merge_onboarding(&local_state, &remote);
                thread.queue(move |b| on_onboarding_synced(b, merged)).ok();
            }
            Ok(None) => {
                let _ = supabase_config::save_onboarding_state(&access_token, &local_state);
            }
            Err(e) => {
                thread
                    .queue(move |b| {
                        emit_status(b, format!("Supabase onboarding fetch failed: {e}"))
                    })
                    .ok();
            }
        },
    );
}

fn sync_config_to_supabase(cfg: &config::Config, thread: BackendThread) {
    if !cfg.supabase_sync_enabled {
        return;
    }
    let Some(session) = cfg.supabase_session.as_ref() else {
        return;
    };
    let access_token = session.access_token.clone();
    let snapshot = cfg.clone();
    std::thread::spawn(move || {
        if let Err(e) = supabase_config::save_config(&access_token, &snapshot) {
            thread
                .clone()
                .queue(move |b| emit_status(b, format!("Supabase config sync failed: {e}")))
                .ok();
        }
        if let Err(e) = supabase_config::save_onboarding_state(&access_token, &snapshot.onboarding)
        {
            thread
                .queue(move |b| emit_status(b, format!("Supabase onboarding sync failed: {e}")))
                .ok();
        }
    });
}

fn sync_onboarding_to_supabase(cfg: &config::Config, thread: BackendThread) {
    if !cfg.supabase_sync_enabled {
        return;
    }
    let Some(session) = cfg.supabase_session.as_ref() else {
        return;
    };
    let access_token = session.access_token.clone();
    let state = cfg.onboarding.clone();
    std::thread::spawn(move || {
        if let Err(e) = supabase_config::save_onboarding_state(&access_token, &state) {
            thread
                .queue(move |b| emit_status(b, format!("Supabase onboarding sync failed: {e}")))
                .ok();
        }
    });
}

// ---------------------------------------------------------------------------
// URL safety (shared by open_url + tests)
// ---------------------------------------------------------------------------
fn safe_external_url(raw: &str) -> Result<url::Url, String> {
    let mut input = raw.trim().to_string();
    if input.is_empty() || input.len() > 2048 {
        return Err("URL is empty or too long".into());
    }
    if !input.contains("://") {
        input = format!("https://{input}");
    }

    let parsed = url::Url::parse(&input).map_err(|_| "Invalid URL".to_string())?;
    if !matches!(parsed.scheme(), "https" | "http") || parsed.host_str().is_none() {
        return Err("Only http and https URLs can be opened".into());
    }
    Ok(parsed)
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------
fn main() {
    env_config::init();

    // WebEngine must be initialized before the QML engine loads a WebEngineView.
    qobject::init_web_engine();

    let mut app = cxx_qt_lib::QGuiApplication::new();
    let mut engine = cxx_qt_lib::QQmlApplicationEngine::new();

    if let Some(engine) = engine.as_mut() {
        engine.load(&cxx_qt_lib::QUrl::from(&QString::from(
            "qrc:/qt/qml/com/happywakey/qml/MainWindow.qml",
        )));
    }

    if let Some(app) = app.as_mut() {
        app.exec();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn safe_external_url_adds_scheme_and_accepts_http_s() {
        assert_eq!(
            safe_external_url("example.com").unwrap().as_str(),
            "https://example.com/"
        );
        assert!(safe_external_url("http://example.com/path").is_ok());
        assert!(safe_external_url("  https://example.com  ").is_ok());
    }

    #[test]
    fn safe_external_url_rejects_dangerous_or_empty() {
        assert!(safe_external_url("").is_err());
        assert!(safe_external_url("   ").is_err());
        assert!(safe_external_url("javascript:alert(1)").is_err());
        assert!(safe_external_url("file:///etc/passwd").is_err());
        assert!(safe_external_url("ftp://example.com").is_err());
        assert!(safe_external_url(&format!("https://{}", "a".repeat(3000))).is_err());
    }
}

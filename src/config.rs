use serde::{Deserialize, Serialize};
use std::io::Write;
use std::path::PathBuf;

pub const ONBOARDING_STEP_WELCOME: &str = "welcome";
pub const ONBOARDING_STEP_ACCOUNT: &str = "account";
pub const ONBOARDING_STEP_BACKUP: &str = "backup";
pub const ONBOARDING_STEP_ESSENTIALS: &str = "essentials";
pub const ONBOARDING_STEP_READY: &str = "ready";
pub const ONBOARDING_STEP_COMPLETE: &str = "complete";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub version: String,
    pub user_id: String,
    pub supabase_session: Option<SupabaseSession>,
    pub calendar_providers: Vec<CalendarProvider>,
    pub weather_locations: Vec<WeatherLocation>,
    pub stock_symbols: Vec<String>,
    pub news_keywords: Vec<String>,
    pub browser_bookmarks: Vec<Bookmark>,
    pub git_repo_path: Option<String>,
    pub supabase_sync_enabled: bool,
    pub onboarding: OnboardingState,
    pub reminder_settings: ReminderSettings,
    pub tasks: Vec<DailyTask>,
    pub focus_minutes: u16,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct SupabaseSession {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_at: i64,
    pub user_id: String,
    pub email: Option<String>,
    pub provider: String,
    pub provider_token: Option<String>,
    pub provider_refresh_token: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct CalendarProvider {
    pub provider: String,
    pub email: String,
    pub access_token: String,
    pub refresh_token: String,
    pub expires_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeatherLocation {
    pub name: String,
    pub lat: f64,
    pub lon: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bookmark {
    pub id: String,
    pub title: String,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct DailyTask {
    pub id: String,
    pub title: String,
    pub completed: bool,
}

impl Default for DailyTask {
    fn default() -> Self {
        Self {
            id: String::new(),
            title: String::new(),
            completed: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct OnboardingState {
    pub completed: bool,
    pub current_step: String,
    pub step_index: u8,
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ReminderSettings {
    pub enabled: bool,
    pub cloud_email_enabled: bool,
    pub offsets_minutes: Vec<u16>,
}

impl Default for ReminderSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            cloud_email_enabled: false,
            offsets_minutes: vec![30, 10],
        }
    }
}

impl Default for OnboardingState {
    fn default() -> Self {
        Self {
            completed: false,
            current_step: ONBOARDING_STEP_WELCOME.into(),
            step_index: 0,
            updated_at: None,
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            version: env!("CARGO_PKG_VERSION").to_string(),
            user_id: String::new(),
            supabase_session: None,
            calendar_providers: Vec::new(),
            weather_locations: Vec::new(),
            // Finnhub's free /quote endpoint prices US equities and ETFs, but not
            // "BTC-USD"-style crypto pseudo-tickers (those need e.g. BINANCE:BTCUSDT),
            // so the default watchlist sticks to symbols that actually return data.
            stock_symbols: vec![
                "AAPL".into(),
                "GOOGL".into(),
                "MSFT".into(),
                "AMZN".into(),
                "NVDA".into(),
                "META".into(),
                "TSLA".into(),
                "SPY".into(),
                "QQQ".into(),
                "GLD".into(),
                "AMD".into(),
                "WMT".into(),
                "JPM".into(),
                "V".into(),
                "KO".into(),
                "DIS".into(),
                "NFLX".into(),
                "BA".into(),
                "XOM".into(),
                "PG".into(),
            ],
            news_keywords: vec!["technology".into(), "AI".into(), "markets".into()],
            browser_bookmarks: Vec::new(),
            git_repo_path: None,
            supabase_sync_enabled: true,
            onboarding: OnboardingState::default(),
            reminder_settings: ReminderSettings::default(),
            tasks: Vec::new(),
            focus_minutes: 25,
        }
    }
}

pub fn config_dir() -> PathBuf {
    if let Ok(path) = std::env::var("CONFIG_DIR") {
        let trimmed = path.trim();
        if !trimmed.is_empty() {
            return PathBuf::from(trimmed);
        }
    }

    let base = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
    base.join("happy-wakey")
}

pub fn config_path() -> PathBuf {
    config_dir().join("config.json")
}

pub fn load() -> Config {
    let path = config_path();
    if path.exists() {
        match std::fs::read_to_string(&path) {
            Ok(content) => serde_json::from_str(&content)
                .map(sanitize)
                .unwrap_or_default(),
            Err(_) => Config::default(),
        }
    } else {
        Config::default()
    }
}

pub fn save(config: &Config) -> Result<(), String> {
    let config = sanitize(config.clone());
    let dir = config_dir();
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let content = serde_json::to_string_pretty(&config).map_err(|e| e.to_string())?;

    let path = config_path();
    let tmp_path = path.with_extension("json.tmp");

    let mut options = std::fs::OpenOptions::new();
    options.create(true).truncate(true).write(true);

    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }

    let mut file = options.open(&tmp_path).map_err(|e| e.to_string())?;
    file.write_all(content.as_bytes())
        .map_err(|e| e.to_string())?;
    file.write_all(b"\n").map_err(|e| e.to_string())?;
    file.sync_all().map_err(|e| e.to_string())?;
    drop(file);

    std::fs::rename(&tmp_path, &path).map_err(|e| e.to_string())?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Ok(metadata) = std::fs::metadata(&path) {
            let mut permissions = metadata.permissions();
            permissions.set_mode(0o600);
            let _ = std::fs::set_permissions(&path, permissions);
        }
    }

    Ok(())
}

pub fn sanitize(mut config: Config) -> Config {
    config.version = env!("CARGO_PKG_VERSION").to_string();
    config.user_id = clean_text(&config.user_id, 128);

    config.weather_locations = config
        .weather_locations
        .into_iter()
        .filter_map(sanitize_weather_location)
        .take(5)
        .collect();

    config.stock_symbols = config
        .stock_symbols
        .into_iter()
        .filter_map(|symbol| sanitize_symbol(&symbol))
        .take(20)
        .collect();

    config.news_keywords = config
        .news_keywords
        .into_iter()
        .filter_map(|keyword| {
            let keyword = clean_text(&keyword, 64);
            if keyword.is_empty() {
                None
            } else {
                Some(keyword)
            }
        })
        .take(20)
        .collect();

    config.browser_bookmarks = config
        .browser_bookmarks
        .into_iter()
        .filter_map(sanitize_bookmark)
        .take(50)
        .collect();

    config.git_repo_path = config.git_repo_path.and_then(|path| {
        let path = clean_text(&path, 512);
        if path.is_empty() {
            None
        } else {
            Some(path)
        }
    });

    config.calendar_providers = config
        .calendar_providers
        .into_iter()
        .map(sanitize_calendar_provider)
        .collect();

    if let Some(mut session) = config.supabase_session {
        session.user_id = clean_text(&session.user_id, 128);
        session.email = session.email.map(|email| clean_text(&email, 254));
        session.provider = clean_text(&session.provider, 32);
        // Tokens are deliberately left untouched: they are opaque and must not be
        // trimmed or length-clamped.
        config.supabase_session = Some(session);
    }

    config.onboarding = sanitize_onboarding_state(config.onboarding);
    config.reminder_settings = sanitize_reminder_settings(config.reminder_settings);
    config.tasks = config
        .tasks
        .into_iter()
        .filter_map(sanitize_task)
        .take(100)
        .collect();
    config.focus_minutes = config.focus_minutes.clamp(5, 120);
    config
}

pub fn sync_safe_config(config: &Config) -> Config {
    let mut safe = sanitize(config.clone());
    safe.supabase_session = None;
    for provider in &mut safe.calendar_providers {
        provider.access_token.clear();
        provider.refresh_token.clear();
    }
    safe
}

pub fn merge_editable_config(mut current: Config, incoming: Config) -> Config {
    let incoming = sanitize(incoming);
    current.weather_locations = incoming.weather_locations;
    current.stock_symbols = incoming.stock_symbols;
    current.news_keywords = incoming.news_keywords;
    current.browser_bookmarks = incoming.browser_bookmarks;
    current.git_repo_path = incoming.git_repo_path;
    current.supabase_sync_enabled = incoming.supabase_sync_enabled;
    current.onboarding = incoming.onboarding;
    current.reminder_settings = incoming.reminder_settings;
    current.tasks = incoming.tasks;
    current.focus_minutes = incoming.focus_minutes;
    sanitize(current)
}

pub fn set_onboarding_state(
    mut config: Config,
    current_step: &str,
    step_index: i32,
    completed: bool,
) -> Config {
    config.onboarding = OnboardingState {
        completed,
        current_step: normalize_onboarding_step(current_step, completed),
        step_index: step_index.clamp(0, 4) as u8,
        updated_at: Some(chrono::Utc::now().to_rfc3339()),
    };
    sanitize(config)
}

pub fn merge_onboarding(local: &OnboardingState, remote: &OnboardingState) -> OnboardingState {
    if remote.completed && !local.completed {
        return sanitize_onboarding_state(remote.clone());
    }

    match (
        local.updated_at.as_deref().and_then(parse_timestamp),
        remote.updated_at.as_deref().and_then(parse_timestamp),
    ) {
        (Some(local_time), Some(remote_time)) if remote_time > local_time => {
            sanitize_onboarding_state(remote.clone())
        }
        (None, Some(_)) => sanitize_onboarding_state(remote.clone()),
        _ => sanitize_onboarding_state(local.clone()),
    }
}

fn sanitize_weather_location(location: WeatherLocation) -> Option<WeatherLocation> {
    let name = clean_text(&location.name, 80);
    if name.is_empty()
        || !location.lat.is_finite()
        || !location.lon.is_finite()
        || !(-90.0..=90.0).contains(&location.lat)
        || !(-180.0..=180.0).contains(&location.lon)
    {
        return None;
    }

    Some(WeatherLocation {
        name,
        lat: location.lat,
        lon: location.lon,
    })
}

pub fn sanitize_symbol(symbol: &str) -> Option<String> {
    let symbol = symbol.trim().to_uppercase();
    if symbol.is_empty()
        || symbol.len() > 24
        || !symbol
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '.' | '-' | '_' | '='))
    {
        return None;
    }
    Some(symbol)
}

fn sanitize_bookmark(bookmark: Bookmark) -> Option<Bookmark> {
    let url = normalize_http_url(&bookmark.url)?;
    let title = clean_text(&bookmark.title, 120);
    Some(Bookmark {
        id: clean_text(&bookmark.id, 64),
        title: if title.is_empty() { url.clone() } else { title },
        url,
    })
}

fn sanitize_task(task: DailyTask) -> Option<DailyTask> {
    let title = clean_text(&task.title, 160);
    if title.is_empty() {
        return None;
    }
    Some(DailyTask {
        id: clean_text(&task.id, 64),
        title,
        completed: task.completed,
    })
}

fn sanitize_calendar_provider(mut provider: CalendarProvider) -> CalendarProvider {
    provider.provider = clean_text(&provider.provider, 64);
    provider.email = clean_text(&provider.email, 254);
    provider
}

pub fn sanitize_onboarding_state(mut state: OnboardingState) -> OnboardingState {
    state.step_index = state.step_index.min(4);
    state.current_step = normalize_onboarding_step(&state.current_step, state.completed);
    state.updated_at = state
        .updated_at
        .filter(|timestamp| parse_timestamp(timestamp).is_some());
    state
}

fn sanitize_reminder_settings(mut settings: ReminderSettings) -> ReminderSettings {
    settings
        .offsets_minutes
        .retain(|minutes| (1..=24 * 60).contains(minutes));
    settings
        .offsets_minutes
        .sort_unstable_by(|left, right| right.cmp(left));
    settings.offsets_minutes.dedup();
    settings.offsets_minutes.truncate(5);
    settings
}

fn normalize_onboarding_step(step: &str, completed: bool) -> String {
    if completed {
        return ONBOARDING_STEP_COMPLETE.into();
    }

    match step.trim() {
        ONBOARDING_STEP_ACCOUNT => ONBOARDING_STEP_ACCOUNT.into(),
        ONBOARDING_STEP_BACKUP => ONBOARDING_STEP_BACKUP.into(),
        ONBOARDING_STEP_ESSENTIALS => ONBOARDING_STEP_ESSENTIALS.into(),
        ONBOARDING_STEP_READY => ONBOARDING_STEP_READY.into(),
        _ => ONBOARDING_STEP_WELCOME.into(),
    }
}

fn normalize_http_url(raw: &str) -> Option<String> {
    let mut input = raw.trim().to_string();
    if input.is_empty() || input.len() > 2048 {
        return None;
    }

    if !input.contains("://") {
        input = format!("https://{input}");
    }

    let parsed = url::Url::parse(&input).ok()?;
    if parsed.host_str().is_none() || !crate::url_safety::is_safe_http_url(&parsed) {
        return None;
    }

    Some(parsed.to_string())
}

fn clean_text(value: &str, max_chars: usize) -> String {
    value
        .trim()
        .chars()
        .filter(|ch| !ch.is_control())
        .take(max_chars)
        .collect()
}

fn parse_timestamp(value: &str) -> Option<chrono::DateTime<chrono::Utc>> {
    chrono::DateTime::parse_from_rfc3339(value)
        .ok()
        .map(|dt| dt.with_timezone(&chrono::Utc))
}

#[cfg(test)]
#[allow(clippy::field_reassign_with_default)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_symbol_normalizes_and_validates() {
        assert_eq!(sanitize_symbol(" aapl "), Some("AAPL".to_string()));
        assert_eq!(sanitize_symbol("brk.b"), Some("BRK.B".to_string()));
        assert_eq!(sanitize_symbol("btc-usd"), Some("BTC-USD".to_string()));
        assert_eq!(sanitize_symbol(""), None);
        assert_eq!(sanitize_symbol("has space"), None);
        assert_eq!(sanitize_symbol("bad$"), None);
        assert_eq!(sanitize_symbol(&"A".repeat(25)), None);
    }

    #[test]
    fn clean_text_strips_controls_trims_and_truncates() {
        assert_eq!(clean_text("  hi\nthere\t ", 100), "hithere");
        assert_eq!(clean_text("abcdef", 3), "abc");
        // truncation is by characters, not bytes
        assert_eq!(clean_text("héllo", 2), "hé");
    }

    #[test]
    fn sanitize_clamps_collection_sizes() {
        let mut cfg = Config::default();
        cfg.stock_symbols = (0..50).map(|i| format!("SYM{i}")).collect();
        cfg.news_keywords = (0..50).map(|i| format!("kw{i}")).collect();
        cfg.weather_locations = (0..20)
            .map(|i| WeatherLocation {
                name: format!("L{i}"),
                lat: 1.0,
                lon: 2.0,
            })
            .collect();
        let cfg = sanitize(cfg);
        assert_eq!(cfg.stock_symbols.len(), 20);
        assert_eq!(cfg.news_keywords.len(), 20);
        assert_eq!(cfg.weather_locations.len(), 5);
    }

    #[test]
    fn sanitize_drops_empty_news_keywords() {
        let mut cfg = Config::default();
        cfg.news_keywords = vec!["  ".into(), "tech".into(), "".into()];
        let cfg = sanitize(cfg);
        assert_eq!(cfg.news_keywords, vec!["tech".to_string()]);
    }

    #[test]
    fn sanitize_weather_location_rejects_out_of_range() {
        assert!(sanitize_weather_location(WeatherLocation {
            name: "x".into(),
            lat: 91.0,
            lon: 0.0,
        })
        .is_none());
        assert!(sanitize_weather_location(WeatherLocation {
            name: "x".into(),
            lat: 0.0,
            lon: 181.0,
        })
        .is_none());
        assert!(sanitize_weather_location(WeatherLocation {
            name: "x".into(),
            lat: f64::NAN,
            lon: 0.0,
        })
        .is_none());
        assert!(sanitize_weather_location(WeatherLocation {
            name: "  ".into(),
            lat: 0.0,
            lon: 0.0,
        })
        .is_none());
        assert!(sanitize_weather_location(WeatherLocation {
            name: " Chicago ".into(),
            lat: 41.88,
            lon: -87.63,
        })
        .is_some());
    }

    #[test]
    fn normalize_http_url_behaviour() {
        assert_eq!(
            normalize_http_url("example.com").as_deref(),
            Some("https://example.com/")
        );
        assert!(normalize_http_url("http://example.com").is_none());
        assert!(normalize_http_url("http://127.0.0.1:8080").is_some());
        assert!(normalize_http_url("https://98.90.186.114").is_none());
        assert!(normalize_http_url("ftp://example.com").is_none());
        assert!(normalize_http_url("javascript:alert(1)").is_none());
        assert!(normalize_http_url("   ").is_none());
        assert!(normalize_http_url(&format!("https://{}", "a".repeat(3000))).is_none());
    }

    #[test]
    fn sanitize_keeps_planner_and_drops_unsafe_bookmarks() {
        let mut config = Config::default();
        config.tasks = vec![
            DailyTask {
                id: "blank".into(),
                title: "   ".into(),
                completed: false,
            },
            DailyTask {
                id: "keep".into(),
                title: " Write tests ".into(),
                completed: true,
            },
        ];
        config.focus_minutes = 999;
        config.browser_bookmarks = vec![Bookmark {
            id: "xss".into(),
            title: "Nope".into(),
            url: "javascript:alert(1)".into(),
        }];
        let sanitized = sanitize(config);
        assert_eq!(sanitized.tasks.len(), 1);
        assert_eq!(sanitized.tasks[0].title, "Write tests");
        assert_eq!(sanitized.focus_minutes, 120);
        assert!(sanitized.browser_bookmarks.is_empty());
    }

    #[test]
    fn merge_editable_config_preserves_identity_and_session() {
        let mut current = Config::default();
        current.user_id = "user-123".into();
        current.supabase_session = Some(SupabaseSession {
            access_token: "keep-me".into(),
            provider: "google".into(),
            provider_token: Some("g-token".into()),
            ..Default::default()
        });

        let mut incoming = Config::default();
        // A malicious/stale client tries to overwrite identity + inject a session.
        incoming.user_id = "attacker".into();
        incoming.supabase_session = Some(SupabaseSession {
            access_token: "evil".into(),
            ..Default::default()
        });
        incoming.stock_symbols = vec!["TSLA".into()];

        let merged = merge_editable_config(current, incoming);
        assert_eq!(merged.user_id, "user-123");
        let session = merged.supabase_session.expect("session preserved");
        assert_eq!(session.access_token, "keep-me");
        assert_eq!(session.provider_token.as_deref(), Some("g-token"));
        assert_eq!(merged.stock_symbols, vec!["TSLA".to_string()]);
    }

    #[test]
    fn sync_safe_config_strips_secrets() {
        let mut cfg = Config::default();
        cfg.supabase_session = Some(SupabaseSession {
            access_token: "secret".into(),
            provider_token: Some("provider-secret".into()),
            ..Default::default()
        });
        cfg.calendar_providers = vec![CalendarProvider {
            provider: "google".into(),
            access_token: "cal-token".into(),
            refresh_token: "cal-refresh".into(),
            ..Default::default()
        }];
        let safe = sync_safe_config(&cfg);
        assert!(safe.supabase_session.is_none());
        assert!(safe.calendar_providers[0].access_token.is_empty());
        assert!(safe.calendar_providers[0].refresh_token.is_empty());
    }

    #[test]
    fn set_onboarding_state_completed_and_clamped() {
        let done = set_onboarding_state(Config::default(), "essentials", 9, true);
        assert!(done.onboarding.completed);
        assert_eq!(done.onboarding.current_step, ONBOARDING_STEP_COMPLETE);
        assert_eq!(done.onboarding.step_index, 4);
        assert!(done.onboarding.updated_at.is_some());

        let mid = set_onboarding_state(Config::default(), "backup", 2, false);
        assert!(!mid.onboarding.completed);
        assert_eq!(mid.onboarding.current_step, ONBOARDING_STEP_BACKUP);
        assert_eq!(mid.onboarding.step_index, 2);

        let unknown = set_onboarding_state(Config::default(), "bogus", -3, false);
        assert_eq!(unknown.onboarding.current_step, ONBOARDING_STEP_WELCOME);
        assert_eq!(unknown.onboarding.step_index, 0);
    }

    #[test]
    fn merge_onboarding_prefers_completion_and_recency() {
        let local = OnboardingState {
            completed: false,
            current_step: ONBOARDING_STEP_ACCOUNT.into(),
            step_index: 1,
            updated_at: Some("2026-01-01T00:00:00Z".into()),
        };
        let remote_done = OnboardingState {
            completed: true,
            current_step: ONBOARDING_STEP_COMPLETE.into(),
            step_index: 4,
            updated_at: Some("2025-01-01T00:00:00Z".into()),
        };
        // Completion wins even though it's older.
        assert!(merge_onboarding(&local, &remote_done).completed);

        let remote_newer = OnboardingState {
            completed: false,
            current_step: ONBOARDING_STEP_BACKUP.into(),
            step_index: 2,
            updated_at: Some("2026-06-01T00:00:00Z".into()),
        };
        assert_eq!(
            merge_onboarding(&local, &remote_newer).current_step,
            ONBOARDING_STEP_BACKUP
        );

        let remote_older = OnboardingState {
            updated_at: Some("2020-01-01T00:00:00Z".into()),
            ..remote_newer.clone()
        };
        // Local is newer, so local wins.
        assert_eq!(
            merge_onboarding(&local, &remote_older).current_step,
            ONBOARDING_STEP_ACCOUNT
        );
    }

    #[test]
    fn sanitize_onboarding_drops_bad_timestamp_and_clamps() {
        let state = OnboardingState {
            completed: false,
            current_step: "essentials".into(),
            step_index: 200,
            updated_at: Some("not-a-date".into()),
        };
        let cleaned = sanitize_onboarding_state(state);
        assert_eq!(cleaned.step_index, 4);
        assert!(cleaned.updated_at.is_none());
        assert_eq!(cleaned.current_step, ONBOARDING_STEP_ESSENTIALS);
    }

    #[test]
    fn sanitize_reminder_offsets_sorts_deduplicates_and_clamps() {
        let cleaned = sanitize_reminder_settings(ReminderSettings {
            enabled: true,
            cloud_email_enabled: false,
            offsets_minutes: vec![10, 30, 10, 0, 1441, 5, 120, 60, 15],
        });
        assert_eq!(cleaned.offsets_minutes, vec![120, 60, 30, 15, 10]);
    }

    #[test]
    fn sanitize_keeps_session_tokens_but_cleans_provider() {
        let mut cfg = Config::default();
        let long_token = "a.b-c_".repeat(100);
        cfg.supabase_session = Some(SupabaseSession {
            access_token: long_token.clone(),
            provider: "  google  ".into(),
            provider_token: Some(long_token.clone()),
            ..Default::default()
        });
        let cfg = sanitize(cfg);
        let session = cfg.supabase_session.unwrap();
        assert_eq!(
            session.access_token, long_token,
            "tokens must not be truncated"
        );
        assert_eq!(session.provider, "google");
    }
}

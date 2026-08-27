use crate::config::ReminderSettings;
use crate::services::calendar::CalendarEvent;
use crate::url_safety::is_safe_http_url;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::sync::{Mutex, OnceLock};
use url::Url;

const SESSION_EXPIRY_MARGIN_SECONDS: u64 = 60;
const REMINDER_POLICY_VERSION: &str = "v1";

#[derive(Clone)]
struct CachedSession {
    source_fingerprint: [u8; 32],
    access_token: String,
    expires_at: u64,
}

#[derive(Deserialize)]
struct ExchangeResponse {
    access_token: String,
    expires_at: u64,
}

#[derive(Serialize)]
struct ReminderSyncRequest {
    jobs: Vec<ReminderJob>,
}

#[derive(Serialize)]
struct ReminderJob {
    job_id: String,
    idempotency_key: String,
    title: String,
    body: String,
    trigger_at: i64,
    channel: &'static str,
}

#[derive(Debug, Deserialize)]
struct SyncResponse {
    result: SyncResult,
}

#[derive(Debug, Deserialize)]
struct SyncResult {
    accepted: usize,
    unchanged: usize,
}

#[derive(Debug, Deserialize)]
struct StatusResponse {
    reminders: ReminderCounts,
}

#[derive(Debug, Deserialize)]
struct ReminderCounts {
    pending: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CloudSyncResult {
    pub accepted: usize,
    pub unchanged: usize,
    pub pending: usize,
}

static SESSION: OnceLock<Mutex<Option<CachedSession>>> = OnceLock::new();

pub fn clear_session() {
    if let Ok(mut session) = session_cache().lock() {
        *session = None;
    }
}

pub fn sync_calendar_reminders(
    supabase_access_token: &str,
    events: &[CalendarEvent],
    settings: &ReminderSettings,
) -> Result<CloudSyncResult, String> {
    if !settings.cloud_email_enabled {
        return Ok(CloudSyncResult {
            accepted: 0,
            unchanged: 0,
            pending: 0,
        });
    }

    let now = chrono::Utc::now().timestamp();
    let jobs = build_reminder_jobs(events, settings, now);
    let access_token = shared_auth_access_token(supabase_access_token)?;
    let sync_url = gateway_url("v1/reminders/sync")?;
    let response: SyncResponse = crate::http::send_json(
        "Happy Wakey cloud reminders",
        crate::http::shared_client()
            .put(sync_url)
            .bearer_auth(&access_token)
            .json(&ReminderSyncRequest { jobs }),
    )?;
    let status_url = gateway_url("v1/reminders/status")?;
    let status: StatusResponse = crate::http::get_json(
        "Happy Wakey cloud reminder status",
        crate::http::shared_client()
            .get(status_url)
            .bearer_auth(&access_token),
    )?;

    Ok(CloudSyncResult {
        accepted: response.result.accepted,
        unchanged: response.result.unchanged,
        pending: status.reminders.pending,
    })
}

pub fn queue_test_reminder(supabase_access_token: &str) -> Result<(), String> {
    let access_token = shared_auth_access_token(supabase_access_token)?;
    let url = gateway_url("v1/reminders/test")?;
    let _: serde_json::Value = crate::http::send_json(
        "Happy Wakey cloud reminder test",
        crate::http::shared_client()
            .post(url)
            .bearer_auth(access_token)
            .json(&serde_json::json!({})),
    )?;
    Ok(())
}

fn shared_auth_access_token(supabase_access_token: &str) -> Result<String, String> {
    if supabase_access_token.trim().is_empty() || supabase_access_token.len() > 16 * 1024 {
        return Err("The Supabase session is missing or invalid".to_string());
    }
    let fingerprint: [u8; 32] = Sha256::digest(supabase_access_token.as_bytes()).into();
    let now = chrono::Utc::now().timestamp().max(0) as u64;
    if let Ok(session) = session_cache().lock() {
        if let Some(cached) = session.as_ref() {
            if cached.source_fingerprint == fingerprint
                && cached.expires_at > now.saturating_add(SESSION_EXPIRY_MARGIN_SECONDS)
            {
                return Ok(cached.access_token.clone());
            }
        }
    }

    let exchange_url = shared_auth_url("auth/exchange")?;
    let response: ExchangeResponse = crate::http::send_json(
        "Shared auth",
        crate::http::shared_client()
            .post(exchange_url)
            .bearer_auth(supabase_access_token)
            .json(&serde_json::json!({})),
    )?;
    if response.access_token.trim().is_empty()
        || response.access_token.len() > 16 * 1024
        || response.expires_at <= now.saturating_add(SESSION_EXPIRY_MARGIN_SECONDS)
    {
        return Err("Shared auth returned an invalid session".to_string());
    }
    if let Ok(mut session) = session_cache().lock() {
        *session = Some(CachedSession {
            source_fingerprint: fingerprint,
            access_token: response.access_token.clone(),
            expires_at: response.expires_at,
        });
    }
    Ok(response.access_token)
}

fn build_reminder_jobs(
    events: &[CalendarEvent],
    settings: &ReminderSettings,
    now: i64,
) -> Vec<ReminderJob> {
    if !settings.enabled {
        return Vec::new();
    }
    let mut jobs = Vec::new();
    for event in events {
        if event.all_day || event.status == "cancelled" || event.start_unix <= now {
            continue;
        }
        for offset_minutes in &settings.offsets_minutes {
            let trigger_at = event.start_unix - i64::from(*offset_minutes) * 60;
            if trigger_at < now {
                continue;
            }
            let key = reminder_key(event, *offset_minutes);
            let mut body = format!(
                "Starts in {offset_minutes} minute{} at {}",
                if *offset_minutes == 1 { "" } else { "s" },
                event.time_label
            );
            if let Some(location) = event
                .location
                .as_deref()
                .filter(|value| !value.trim().is_empty())
            {
                body.push('\n');
                body.push_str(location);
            }
            jobs.push(ReminderJob {
                job_id: key.clone(),
                idempotency_key: key,
                title: event.title.clone(),
                body,
                trigger_at,
                channel: "email",
            });
        }
    }
    jobs
}

fn reminder_key(event: &CalendarEvent, offset_minutes: u16) -> String {
    let identity = format!(
        "{REMINDER_POLICY_VERSION}|{}|{}|{}|{offset_minutes}",
        event.provider, event.id, event.start_unix
    );
    format!("{:x}", Sha256::digest(identity.as_bytes()))
}

fn session_cache() -> &'static Mutex<Option<CachedSession>> {
    SESSION.get_or_init(|| Mutex::new(None))
}

fn shared_auth_url(path: &str) -> Result<Url, String> {
    service_url("HAPPY_WAKEY_SHARED_AUTH_URL", "shared-auth/", path)
}

fn gateway_url(path: &str) -> Result<Url, String> {
    service_url("HAPPY_WAKEY_GATEWAY_URL", "happy-wakey/", path)
}

fn service_url(override_name: &str, default_prefix: &str, path: &str) -> Result<Url, String> {
    let raw = std::env::var(override_name)
        .ok()
        .filter(|value| !value.trim().is_empty())
        .or_else(|| {
            let platform = std::env::var("HAPPY_WAKEY_PLATFORM_URL")
                .ok()
                .filter(|value| !value.trim().is_empty())?;
            Some(format!(
                "{}/{default_prefix}",
                platform.trim_end_matches('/')
            ))
        })
        .ok_or_else(|| format!("{override_name} or HAPPY_WAKEY_PLATFORM_URL must be set"))?;
    let mut base =
        Url::parse(raw.trim()).map_err(|_| format!("{override_name} must be an absolute URL"))?;
    if base.host_str().is_none()
        || !base.username().is_empty()
        || base.password().is_some()
        || base.query().is_some()
        || base.fragment().is_some()
        || !is_safe_http_url(&base)
    {
        return Err(format!("{override_name} is not a safe HTTP service URL"));
    }
    if !base.path().ends_with('/') {
        let directory = format!("{}/", base.path());
        base.set_path(&directory);
    }
    base.join(path)
        .map_err(|_| format!("{override_name} could not form a service endpoint"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn event(start_unix: i64) -> CalendarEvent {
        CalendarEvent {
            id: "provider-event-123".into(),
            ical_uid: None,
            title: "Planning".into(),
            start: String::new(),
            end: String::new(),
            start_unix,
            end_unix: start_unix + 1800,
            all_day: false,
            provider: "google".into(),
            status: "confirmed".into(),
            description: None,
            location: Some("Room 2".into()),
            join_url: None,
            event_url: None,
            day_key: String::new(),
            day_label: String::new(),
            time_label: "9:00 AM".into(),
        }
    }

    #[test]
    fn reminder_jobs_are_deterministic_and_future_only() {
        let now = 1_800_000_000;
        let settings = ReminderSettings {
            enabled: true,
            cloud_email_enabled: true,
            offsets_minutes: vec![30, 10],
        };
        let jobs = build_reminder_jobs(&[event(now + 20 * 60)], &settings, now);
        assert_eq!(jobs.len(), 1);
        assert_eq!(jobs[0].trigger_at, now + 10 * 60);
        assert_eq!(jobs[0].job_id, jobs[0].idempotency_key);
        assert_eq!(jobs[0].job_id.len(), 64);
        assert_eq!(
            jobs[0].job_id,
            build_reminder_jobs(&[event(now + 20 * 60)], &settings, now)[0].job_id
        );
    }

    #[test]
    fn service_urls_require_https_except_loopback() {
        std::env::set_var(
            "HAPPY_WAKEY_GATEWAY_URL",
            "http://example.test/happy-wakey/",
        );
        assert!(gateway_url("v1/bootstrap").is_err());
        std::env::set_var("HAPPY_WAKEY_GATEWAY_URL", "http://127.0.0.1:8128/");
        assert_eq!(
            gateway_url("v1/bootstrap").unwrap().as_str(),
            "http://127.0.0.1:8128/v1/bootstrap"
        );
        std::env::remove_var("HAPPY_WAKEY_GATEWAY_URL");
    }

    #[test]
    fn service_urls_fail_closed_without_platform_and_reject_public_ips() {
        std::env::remove_var("HAPPY_WAKEY_GATEWAY_URL");
        std::env::remove_var("HAPPY_WAKEY_PLATFORM_URL");
        assert!(gateway_url("v1/bootstrap").is_err());
        std::env::set_var("HAPPY_WAKEY_GATEWAY_URL", "https://98.90.186.114/");
        assert!(gateway_url("v1/bootstrap").is_err());
        std::env::set_var("HAPPY_WAKEY_GATEWAY_URL", "https://gateway.example.test/");
        assert_eq!(
            gateway_url("v1/bootstrap").unwrap().as_str(),
            "https://gateway.example.test/v1/bootstrap"
        );
        std::env::remove_var("HAPPY_WAKEY_GATEWAY_URL");
    }
}

use crate::url_safety::is_safe_http_url;
use chrono::{DateTime, Local, NaiveDate, SecondsFormat, Utc};
use serde::{Deserialize, Serialize};
use url::Url;

const MAX_MESSAGE_ITEMS: usize = 20;
const MAX_MESSAGE_PLATFORMS: usize = 12;
const MAX_PLATFORM_THREADS: usize = 50;
const MAX_ANOMALIES: usize = 8;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct DirectMessageItem {
    pub id: String,
    pub source: String,
    pub access_level: String,
    pub conversation: String,
    pub preview: String,
    pub received_at: String,
    pub unread_count: u64,
    pub mentions_owner: bool,
    pub url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct MessageDigestView {
    pub state: String,
    pub unread_total: Option<u64>,
    pub items: Vec<DirectMessageItem>,
    pub note: String,
}

impl MessageDigestView {
    pub fn unavailable(note: impl Into<String>) -> Self {
        Self {
            state: "not_connected".to_owned(),
            unread_total: None,
            items: Vec::new(),
            note: note.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SleepView {
    pub night_of: String,
    pub source: String,
    pub measurement_basis: String,
    pub confidence: String,
    pub asleep_minutes: u16,
    pub light_minutes: Option<u16>,
    pub deep_minutes: Option<u16>,
    pub rem_minutes: Option<u16>,
    pub awake_minutes: Option<u16>,
    pub unspecified_minutes: Option<u16>,
    pub sleep_score: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct BiometricAnomalyView {
    pub metric: String,
    pub direction: String,
    pub severity: String,
    pub observed: f64,
    pub baseline_mean: f64,
    pub z_score: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct BiometricView {
    pub day: String,
    pub source: String,
    pub measurement_basis: String,
    pub confidence: String,
    pub readiness_score: Option<f64>,
    pub steps: Option<u64>,
    pub active_energy_kcal: Option<f64>,
    pub resting_heart_rate_bpm: Option<f64>,
    pub heart_rate_variability_ms: Option<f64>,
    pub anomalies: Vec<BiometricAnomalyView>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct HealthBriefView {
    pub state: String,
    pub sleep: Option<SleepView>,
    pub biometrics: Option<BiometricView>,
    pub note: String,
}

impl HealthBriefView {
    pub fn unavailable(note: impl Into<String>) -> Self {
        Self {
            state: "not_connected".to_owned(),
            sleep: None,
            biometrics: None,
            note: note.into(),
        }
    }
}

#[derive(Debug, Deserialize)]
struct MessageDigest {
    schema: String,
    generated_at: String,
    window_start: String,
    window_end: String,
    #[serde(rename = "generation")]
    _generation: u64,
    #[serde(default)]
    platforms: Vec<PlatformDigest>,
}

#[derive(Debug, Deserialize)]
struct PlatformDigest {
    platform: String,
    access_level: String,
    status: String,
    unread_total: Option<u64>,
    #[serde(default)]
    threads: Vec<MessageThread>,
    #[serde(default)]
    truncated: bool,
    completeness_note: Option<String>,
}

#[derive(Debug, Deserialize)]
struct MessageThread {
    thread_ref: String,
    counterpart_display: String,
    unread_count: u64,
    last_message_at: String,
    preview: Option<String>,
    #[serde(default)]
    mentions_owner: bool,
    deep_link: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SleepSummary {
    night_of: String,
    source_provider: String,
    measurement_basis: String,
    confidence: String,
    asleep_minutes: u16,
    stage_minutes: Option<SleepStages>,
    sleep_score: Option<f64>,
}

#[derive(Debug, Deserialize)]
struct SleepStages {
    awake: u16,
    light: u16,
    deep: u16,
    rem: u16,
    unspecified: u16,
}

#[derive(Debug, Deserialize)]
struct BiometricSummary {
    day: String,
    source_provider: String,
    measurement_basis: String,
    confidence: String,
    readiness_score: Option<f64>,
    steps: Option<u64>,
    active_energy_kcal: Option<f64>,
    resting_heart_rate_bpm: Option<f64>,
    heart_rate_variability_ms: Option<f64>,
    #[serde(default)]
    anomalies: Vec<BiometricAnomaly>,
}

#[derive(Debug, Deserialize)]
struct BiometricAnomaly {
    metric: String,
    direction: String,
    severity: String,
    observed: f64,
    baseline_mean: f64,
    z_score: f64,
}

/// Fetch the canonical, policy-aware direct-message digest.
///
/// Platform OAuth credentials remain in the product backend. The desktop sends
/// only its short-lived Shared Auth bearer and honors each platform's declared
/// access level; unavailable platforms never manufacture message previews.
pub fn fetch_messages(supabase_access_token: &str) -> Result<MessageDigestView, String> {
    let digest: MessageDigest = crate::gateway::get_authorized_json(
        supabase_access_token,
        "v1/messages/digest",
        "Happy Wakey message digest",
    )
    .map_err(|_| "The direct-message digest is unavailable".to_owned())?;
    normalize_messages(digest).map_err(|_| "The direct-message digest is unavailable".to_owned())
}

/// Fetch sleep and biometrics as two independent reads inside one health lane.
/// A single missing source yields an explicit degraded result rather than
/// zero-filled measurements; both missing sources fail the lane.
pub fn fetch_health(supabase_access_token: &str) -> Result<HealthBriefView, String> {
    fetch_health_for_day(supabase_access_token, Local::now().date_naive())
}

fn fetch_health_for_day(
    supabase_access_token: &str,
    day: NaiveDate,
) -> Result<HealthBriefView, String> {
    let sleep_day = day.pred_opt().unwrap_or(day);
    let sleep_path = format!("v1/health/sleep/{sleep_day}");
    let biometric_path = format!("v1/health/biometrics/{day}");
    let (sleep_result, biometric_result) = std::thread::scope(|scope| {
        let sleep = scope.spawn(|| {
            crate::gateway::get_authorized_json::<SleepSummary>(
                supabase_access_token,
                &sleep_path,
                "Happy Wakey sleep summary",
            )
        });
        let biometrics = scope.spawn(|| {
            crate::gateway::get_authorized_json::<BiometricSummary>(
                supabase_access_token,
                &biometric_path,
                "Happy Wakey biometric summary",
            )
        });
        (
            sleep
                .join()
                .unwrap_or_else(|_| Err("sleep worker stopped".to_owned())),
            biometrics
                .join()
                .unwrap_or_else(|_| Err("biometric worker stopped".to_owned())),
        )
    });

    let sleep = sleep_result
        .ok()
        .and_then(|summary| normalize_sleep(summary, sleep_day));
    let biometrics = biometric_result
        .ok()
        .and_then(|summary| normalize_biometrics(summary, day));
    match (sleep, biometrics) {
        (None, None) => Err("Health summaries are unavailable".to_owned()),
        (sleep, biometrics) => {
            let complete = sleep.is_some() && biometrics.is_some();
            Ok(HealthBriefView {
                state: if complete { "ready" } else { "degraded" }.to_owned(),
                sleep,
                biometrics,
                note: if complete {
                    "Consented health summaries loaded".to_owned()
                } else {
                    "One consented health source is unavailable".to_owned()
                },
            })
        }
    }
}

fn normalize_messages(digest: MessageDigest) -> Result<MessageDigestView, String> {
    let generated_at = DateTime::parse_from_rfc3339(&digest.generated_at)
        .map_err(|_| "invalid generated timestamp")?;
    let window_start =
        DateTime::parse_from_rfc3339(&digest.window_start).map_err(|_| "invalid window start")?;
    let window_end =
        DateTime::parse_from_rfc3339(&digest.window_end).map_err(|_| "invalid window end")?;
    if digest.schema != "happy-wakey/message-digest/v1"
        || window_start > window_end
        || generated_at < window_start
    {
        return Err("invalid message digest envelope".to_owned());
    }

    let mut items = Vec::new();
    let mut unread_total = 0_u64;
    let mut has_total = false;
    let mut degraded_notes = Vec::new();
    if digest.platforms.len() > MAX_MESSAGE_PLATFORMS {
        degraded_notes.push("Additional message platforms were omitted".to_owned());
    }
    for platform in digest.platforms.into_iter().take(MAX_MESSAGE_PLATFORMS) {
        let known_platform = valid_message_platform(&platform.platform);
        let usable_status = known_platform
            && matches!(
                platform.status.as_str(),
                "connected" | "degraded" | "rate_limited"
            );
        let readable = usable_status
            && matches!(
                platform.access_level.as_str(),
                "full_read" | "throttled_read"
            );
        let count_visible =
            usable_status && (readable || platform.access_level.as_str() == "count_only");
        if let Some(total) = platform
            .unread_total
            .filter(|total| count_visible && *total <= 1_000_000)
        {
            has_total = true;
            unread_total = unread_total.saturating_add(total);
        }
        if !known_platform || platform.status != "connected" || platform.truncated {
            degraded_notes.push(
                platform
                    .completeness_note
                    .as_deref()
                    .map(|note| bounded(note, 180))
                    .filter(|note| !note.is_empty())
                    .unwrap_or_else(|| format!("{} is {}", platform.platform, platform.status)),
            );
        }
        if !readable {
            continue;
        }
        let mut invalid_threads = 0_usize;
        if platform.threads.len() > MAX_PLATFORM_THREADS {
            invalid_threads += platform.threads.len() - MAX_PLATFORM_THREADS;
        }
        for thread in platform.threads.into_iter().take(MAX_PLATFORM_THREADS) {
            let id = bounded(thread.thread_ref, 180);
            let conversation = bounded(thread.counterpart_display, 120);
            let received_at = DateTime::parse_from_rfc3339(&thread.last_message_at)
                .ok()
                .map(|value| {
                    value
                        .with_timezone(&Utc)
                        .to_rfc3339_opts(SecondsFormat::Secs, true)
                });
            if id.is_empty()
                || conversation.is_empty()
                || thread.unread_count > 100_000
                || received_at.is_none()
            {
                invalid_threads += 1;
                continue;
            }
            items.push(DirectMessageItem {
                id,
                source: bounded(&platform.platform, 40),
                access_level: bounded(&platform.access_level, 32),
                conversation,
                preview: bounded(thread.preview.unwrap_or_default(), 300),
                received_at: received_at.expect("validated message timestamp"),
                unread_count: thread.unread_count,
                mentions_owner: thread.mentions_owner,
                url: thread.deep_link.as_deref().and_then(safe_url),
            });
        }
        if invalid_threads > 0 {
            degraded_notes.push(format!(
                "{} omitted {invalid_threads} invalid or excess thread(s)",
                bounded(&platform.platform, 40)
            ));
        }
    }
    items.sort_by(|left, right| right.received_at.cmp(&left.received_at));
    items.dedup_by(|left, right| left.source == right.source && left.id == right.id);
    items.truncate(MAX_MESSAGE_ITEMS);
    degraded_notes.truncate(3);
    let state = if !degraded_notes.is_empty() {
        "degraded"
    } else if items.is_empty() {
        "empty"
    } else {
        "ready"
    };
    Ok(MessageDigestView {
        state: state.to_owned(),
        unread_total: has_total.then_some(unread_total),
        items,
        note: if degraded_notes.is_empty() {
            "Policy-scoped message digest".to_owned()
        } else {
            degraded_notes.join("; ")
        },
    })
}

fn normalize_sleep(summary: SleepSummary, expected_day: NaiveDate) -> Option<SleepView> {
    let night_of = NaiveDate::parse_from_str(&summary.night_of, "%Y-%m-%d").ok()?;
    if night_of != expected_day
        || !valid_health_source(&summary.source_provider)
        || !valid_measurement_basis(&summary.measurement_basis)
        || !valid_confidence(&summary.confidence)
        || summary.asleep_minutes > 1_440
        || summary.stage_minutes.as_ref().is_some_and(|stages| {
            [
                stages.awake,
                stages.light,
                stages.deep,
                stages.rem,
                stages.unspecified,
            ]
            .into_iter()
            .any(|minutes| minutes > 1_440)
        })
        || summary
            .sleep_score
            .is_some_and(|score| !score.is_finite() || !(0.0..=100.0).contains(&score))
    {
        return None;
    }
    let (awake_minutes, light_minutes, deep_minutes, rem_minutes, unspecified_minutes) = summary
        .stage_minutes
        .map(|stages| {
            (
                Some(stages.awake),
                Some(stages.light),
                Some(stages.deep),
                Some(stages.rem),
                Some(stages.unspecified),
            )
        })
        .unwrap_or((None, None, None, None, None));
    Some(SleepView {
        night_of: summary.night_of,
        source: bounded(summary.source_provider, 64),
        measurement_basis: bounded(summary.measurement_basis, 32),
        confidence: bounded(summary.confidence, 32),
        asleep_minutes: summary.asleep_minutes,
        light_minutes,
        deep_minutes,
        rem_minutes,
        awake_minutes,
        unspecified_minutes,
        sleep_score: summary.sleep_score,
    })
}

fn normalize_biometrics(
    summary: BiometricSummary,
    expected_day: NaiveDate,
) -> Option<BiometricView> {
    let day = NaiveDate::parse_from_str(&summary.day, "%Y-%m-%d").ok()?;
    if day != expected_day
        || !valid_health_source(&summary.source_provider)
        || !valid_measurement_basis(&summary.measurement_basis)
        || !valid_confidence(&summary.confidence)
        || summary
            .readiness_score
            .is_some_and(|value| !bounded_number(value, 0.0, 100.0))
        || summary.steps.is_some_and(|value| value > 500_000)
        || summary
            .active_energy_kcal
            .is_some_and(|value| !bounded_number(value, 0.0, 30_000.0))
        || summary
            .resting_heart_rate_bpm
            .is_some_and(|value| !bounded_number(value, 20.0, 200.0))
        || summary
            .heart_rate_variability_ms
            .is_some_and(|value| !bounded_number(value, 0.0, 500.0))
    {
        return None;
    }
    let anomalies = summary
        .anomalies
        .into_iter()
        .filter(|value| {
            value.observed.is_finite()
                && value.baseline_mean.is_finite()
                && bounded_number(value.z_score, -20.0, 20.0)
                && matches!(
                    value.metric.as_str(),
                    "resting_heart_rate"
                        | "heart_rate_variability"
                        | "respiratory_rate"
                        | "blood_oxygen"
                        | "body_temperature"
                        | "sleep_duration"
                        | "sleep_efficiency"
                        | "steps"
                        | "active_energy"
                        | "cardio_fitness"
                        | "stress_load"
                )
                && matches!(
                    value.direction.as_str(),
                    "above_baseline" | "below_baseline"
                )
                && matches!(value.severity.as_str(), "notable" | "marked" | "extreme")
        })
        .take(MAX_ANOMALIES)
        .map(|value| BiometricAnomalyView {
            metric: bounded(value.metric, 48),
            direction: bounded(value.direction, 32),
            severity: bounded(value.severity, 24),
            observed: value.observed,
            baseline_mean: value.baseline_mean,
            z_score: value.z_score,
        })
        .collect();
    Some(BiometricView {
        day: summary.day,
        source: bounded(summary.source_provider, 64),
        measurement_basis: bounded(summary.measurement_basis, 32),
        confidence: bounded(summary.confidence, 32),
        readiness_score: summary.readiness_score,
        steps: summary.steps,
        active_energy_kcal: summary.active_energy_kcal,
        resting_heart_rate_bpm: summary.resting_heart_rate_bpm,
        heart_rate_variability_ms: summary.heart_rate_variability_ms,
        anomalies,
    })
}

fn safe_url(raw: &str) -> Option<String> {
    if raw.len() > 2_048 {
        return None;
    }
    let value = Url::parse(raw).ok()?;
    is_safe_http_url(&value).then(|| value.into())
}

fn valid_health_source(value: &str) -> bool {
    matches!(
        value,
        "apple_healthkit"
            | "android_health_connect"
            | "oura"
            | "whoop"
            | "google_health"
            | "garmin"
            | "withings"
            | "health_aggregator"
    )
}

fn valid_message_platform(value: &str) -> bool {
    matches!(
        value,
        "slack"
            | "telegram"
            | "discord"
            | "whatsapp"
            | "instagram"
            | "linkedin"
            | "imessage"
            | "x"
            | "os_notifications"
    )
}

fn valid_measurement_basis(value: &str) -> bool {
    matches!(value, "wearable" | "phone" | "modeled" | "manual")
}

fn valid_confidence(value: &str) -> bool {
    matches!(value, "high" | "medium" | "low" | "insufficient_data")
}

fn bounded_number(value: f64, minimum: f64, maximum: f64) -> bool {
    value.is_finite() && (minimum..=maximum).contains(&value)
}

fn bounded(value: impl AsRef<str>, max_chars: usize) -> String {
    let cleaned = value
        .as_ref()
        .chars()
        .map(|character| {
            if character.is_control() {
                ' '
            } else {
                character
            }
        })
        .collect::<String>();
    let trimmed = cleaned.trim();
    if trimmed.chars().count() <= max_chars {
        return trimmed.to_owned();
    }
    trimmed.chars().take(max_chars).collect::<String>() + "…"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_message_digest_respects_platform_policy_and_bounds() {
        let digest: MessageDigest = serde_json::from_value(serde_json::json!({
            "schema": "happy-wakey/message-digest/v1",
            "generated_at": "2026-09-05T12:00:00Z",
            "window_start": "2026-09-05T00:00:00Z",
            "window_end": "2026-09-05T12:00:00Z",
            "generation": 7,
            "platforms": [
                {
                    "platform": "telegram",
                    "access_level": "full_read",
                    "status": "connected",
                    "unread_total": 3,
                    "threads": [{
                        "thread_ref": "thread-1",
                        "counterpart_display": "Site crew",
                        "unread_count": 3,
                        "last_message_at": "2026-09-05T10:51:00Z",
                        "preview": "Concrete moved to Monday",
                        "mentions_owner": true,
                        "deep_link": "https://t.me/c/123/456"
                    }],
                    "truncated": false,
                    "completeness_note": null
                },
                {
                    "platform": "whatsapp",
                    "access_level": "policy_unavailable",
                    "status": "policy_unavailable",
                    "unread_total": 77,
                    "threads": [{
                        "thread_ref": "must-not-surface",
                        "counterpart_display": "Hidden",
                        "unread_count": 9,
                        "last_message_at": "2026-09-05T11:00:00Z",
                        "preview": "Hidden",
                        "mentions_owner": false,
                        "deep_link": null
                    }],
                    "truncated": false,
                    "completeness_note": "No personal-message API"
                }
            ]
        }))
        .unwrap();
        let view = normalize_messages(digest).unwrap();
        assert_eq!(view.items.len(), 1);
        assert_eq!(view.items[0].source, "telegram");
        assert_eq!(view.unread_total, Some(3));
        assert_eq!(view.state, "degraded");
        assert!(view.note.contains("No personal-message API"));
    }

    #[test]
    fn health_normalization_never_zero_fills_invalid_measurements() {
        let valid: SleepSummary = serde_json::from_value(serde_json::json!({
            "night_of": "2026-09-04",
            "source_provider": "oura",
            "measurement_basis": "wearable",
            "confidence": "high",
            "asleep_minutes": 401,
            "stage_minutes": {"awake": 45, "light": 214, "deep": 71, "rem": 116, "unspecified": 0},
            "sleep_score": 81
        }))
        .unwrap();
        assert_eq!(
            normalize_sleep(valid, NaiveDate::from_ymd_opt(2026, 9, 4).unwrap())
                .unwrap()
                .deep_minutes,
            Some(71)
        );

        let invalid: SleepSummary = serde_json::from_value(serde_json::json!({
            "night_of": "2026-09-04",
            "source_provider": "modeled",
            "measurement_basis": "modeled",
            "confidence": "low",
            "asleep_minutes": 2000,
            "stage_minutes": {"awake": 0, "light": 0, "deep": 0, "rem": 0, "unspecified": 0},
            "sleep_score": 50
        }))
        .unwrap();
        assert!(normalize_sleep(invalid, NaiveDate::from_ymd_opt(2026, 9, 4).unwrap()).is_none());
    }

    #[test]
    fn biometric_anomalies_are_bounded_and_non_diagnostic() {
        let summary: BiometricSummary = serde_json::from_value(serde_json::json!({
            "day": "2026-09-05",
            "source_provider": "oura",
            "measurement_basis": "wearable",
            "confidence": "medium",
            "readiness_score": 68,
            "steps": 3120,
            "active_energy_kcal": 210.5,
            "resting_heart_rate_bpm": 58.4,
            "heart_rate_variability_ms": 44.0,
            "anomalies": (0..12).map(|_| serde_json::json!({
                "metric": "resting_heart_rate",
                "direction": "above_baseline",
                "severity": "notable",
                "observed": 58.4,
                "baseline_mean": 53.1,
                "z_score": 2.1
            })).collect::<Vec<_>>()
        }))
        .unwrap();
        let view =
            normalize_biometrics(summary, NaiveDate::from_ymd_opt(2026, 9, 5).unwrap()).unwrap();
        assert_eq!(view.anomalies.len(), MAX_ANOMALIES);
        assert_eq!(view.steps, Some(3120));
    }

    #[test]
    fn nullable_sleep_stages_remain_absent_and_wrong_dates_fail_closed() {
        let summary: SleepSummary = serde_json::from_value(serde_json::json!({
            "night_of": "2026-09-04",
            "source_provider": "apple_healthkit",
            "measurement_basis": "phone",
            "confidence": "insufficient_data",
            "asleep_minutes": 388,
            "stage_minutes": null,
            "sleep_score": null
        }))
        .unwrap();
        let view = normalize_sleep(summary, NaiveDate::from_ymd_opt(2026, 9, 4).unwrap()).unwrap();
        assert_eq!(view.deep_minutes, None);

        let wrong_day: BiometricSummary = serde_json::from_value(serde_json::json!({
            "day": "2026-09-03",
            "source_provider": "oura",
            "measurement_basis": "wearable",
            "confidence": "high",
            "readiness_score": null,
            "steps": null,
            "active_energy_kcal": null,
            "resting_heart_rate_bpm": null,
            "heart_rate_variability_ms": null,
            "anomalies": []
        }))
        .unwrap();
        assert!(
            normalize_biometrics(wrong_day, NaiveDate::from_ymd_opt(2026, 9, 4).unwrap()).is_none()
        );
    }
}

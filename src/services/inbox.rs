use crate::url_safety::is_safe_http_url;
use chrono::{DateTime, TimeZone, Utc};
use serde::{Deserialize, Serialize};
use url::Url;

const MAX_INBOX_ITEMS: usize = 20;
const MAX_TOKEN_BYTES: usize = 16 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct InboxItem {
    pub id: String,
    pub source: String,
    pub sender_display: String,
    pub sender_address: String,
    pub subject: String,
    pub preview: String,
    pub received_at: String,
    pub unread: bool,
    pub important: bool,
    pub url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct InboxDigestView {
    pub provider: String,
    pub state: String,
    pub unread_total: usize,
    pub items: Vec<InboxItem>,
    pub note: String,
}

impl InboxDigestView {
    pub fn unavailable(note: impl Into<String>) -> Self {
        Self {
            provider: String::new(),
            state: "not_connected".to_owned(),
            unread_total: 0,
            items: Vec::new(),
            note: note.into(),
        }
    }
}

#[derive(Debug, Deserialize)]
struct GmailList {
    #[serde(default)]
    messages: Vec<GmailReference>,
    #[serde(rename = "resultSizeEstimate", default)]
    result_size_estimate: usize,
}

#[derive(Debug, Deserialize)]
struct GmailReference {
    id: String,
}

#[derive(Debug, Deserialize)]
struct GmailMessage {
    id: Option<String>,
    #[serde(rename = "internalDate")]
    internal_date: Option<String>,
    #[serde(rename = "labelIds", default)]
    label_ids: Vec<String>,
    payload: Option<GmailPayload>,
}

#[derive(Debug, Deserialize)]
struct GmailPayload {
    #[serde(default)]
    headers: Vec<GmailHeader>,
}

#[derive(Debug, Deserialize)]
struct GmailHeader {
    name: String,
    value: String,
}

#[derive(Debug, Deserialize)]
struct MicrosoftList {
    #[serde(default)]
    value: Vec<MicrosoftMessage>,
    #[serde(rename = "@odata.nextLink")]
    next_link: Option<String>,
}

#[derive(Debug, Deserialize)]
struct MicrosoftMessage {
    id: String,
    subject: Option<String>,
    from: Option<MicrosoftFrom>,
    #[serde(rename = "receivedDateTime")]
    received_at: Option<String>,
    #[serde(rename = "isRead", default)]
    is_read: bool,
    importance: Option<String>,
    #[serde(rename = "webLink")]
    web_link: Option<String>,
}

#[derive(Debug, Deserialize)]
struct MicrosoftFrom {
    #[serde(rename = "emailAddress")]
    email_address: Option<MicrosoftAddress>,
}

#[derive(Debug, Deserialize)]
struct MicrosoftAddress {
    name: Option<String>,
    address: Option<String>,
}

/// Read the small, provider-scoped inbox window used by the morning brief.
///
/// The OAuth token is used only as an Authorization header on the worker
/// thread. It is never returned, serialized, cached, or included in errors.
pub fn fetch(provider: &str, provider_token: &str) -> Result<InboxDigestView, String> {
    if provider_token.trim().is_empty() || provider_token.len() > MAX_TOKEN_BYTES {
        return Err("Inbox access was not granted at sign-in".to_owned());
    }
    match provider.trim().to_ascii_lowercase().as_str() {
        "google" => fetch_gmail(provider_token),
        "azure" | "microsoft" => fetch_microsoft(provider_token),
        "apple" => Err("Apple Sign In does not grant mail access".to_owned()),
        _ => Err("The current sign-in provider does not support inbox reads".to_owned()),
    }
}

fn fetch_gmail(provider_token: &str) -> Result<InboxDigestView, String> {
    let mut list_url = Url::parse("https://gmail.googleapis.com/gmail/v1/users/me/messages")
        .expect("static Gmail URL");
    list_url
        .query_pairs_mut()
        // gmail.metadata is the least-privilege scope that still exposes labels
        // and selected headers. Gmail does not allow the `q` parameter with that
        // scope, so select unread inbox mail by label and discard low-priority
        // categories after reading each bounded metadata record.
        .append_pair("labelIds", "INBOX")
        .append_pair("labelIds", "UNREAD")
        .append_pair("maxResults", &MAX_INBOX_ITEMS.to_string());
    let list: GmailList = crate::http::get_json(
        "Gmail inbox",
        crate::http::authorized_client()
            .get(list_url)
            .bearer_auth(provider_token)
            .header("Accept", "application/json"),
    )
    .map_err(|_| "Gmail inbox could not be read".to_owned())?;

    let mut items = Vec::new();
    let mut skipped = 0_usize;
    for reference in list.messages.iter().take(MAX_INBOX_ITEMS) {
        let Some(message_id) = provider_identifier(&reference.id) else {
            skipped += 1;
            continue;
        };
        let mut detail_url = Url::parse("https://gmail.googleapis.com/gmail/v1/users/me/messages/")
            .expect("static Gmail URL");
        detail_url
            .path_segments_mut()
            .expect("static Gmail URL can accept a path segment")
            .push(message_id);
        detail_url
            .query_pairs_mut()
            .append_pair("format", "metadata")
            .append_pair("metadataHeaders", "From")
            .append_pair("metadataHeaders", "Subject")
            .append_pair("metadataHeaders", "Date");
        let detail: Result<GmailMessage, _> = crate::http::get_json(
            "Gmail message metadata",
            crate::http::authorized_client()
                .get(detail_url)
                .bearer_auth(provider_token)
                .header("Accept", "application/json"),
        );
        match detail {
            Ok(message) if is_low_priority_gmail_category(&message) => {}
            Ok(message) => match parse_gmail_message(message) {
                Some(item) => items.push(item),
                None => skipped += 1,
            },
            Err(_) => skipped += 1,
        }
    }
    Ok(finish_digest(
        "gmail",
        list.result_size_estimate,
        items,
        skipped,
    ))
}

fn fetch_microsoft(provider_token: &str) -> Result<InboxDigestView, String> {
    let mut url = Url::parse("https://graph.microsoft.com/v1.0/me/mailFolders/inbox/messages")
        .expect("static Microsoft Graph URL");
    url.query_pairs_mut()
        .append_pair("$top", &MAX_INBOX_ITEMS.to_string())
        .append_pair(
            "$select",
            "id,subject,from,receivedDateTime,isRead,importance,webLink",
        )
        .append_pair("$filter", "isRead eq false")
        .append_pair("$orderby", "receivedDateTime desc");
    let list: MicrosoftList = crate::http::get_json(
        "Microsoft inbox",
        crate::http::authorized_client()
            .get(url)
            .bearer_auth(provider_token)
            .header("Accept", "application/json"),
    )
    .map_err(|_| "Microsoft inbox could not be read".to_owned())?;
    let unread_total = list.value.iter().filter(|message| !message.is_read).count();
    let skipped = usize::from(list.next_link.is_some());
    let items = list
        .value
        .into_iter()
        .take(MAX_INBOX_ITEMS)
        .filter_map(parse_microsoft_message)
        .collect();
    Ok(finish_digest(
        "microsoft_graph_mail",
        unread_total,
        items,
        skipped,
    ))
}

fn parse_gmail_message(message: GmailMessage) -> Option<InboxItem> {
    let id = provider_identifier(message.id.as_deref()?)?.to_owned();
    let payload = message.payload?;
    let header = |name: &str| {
        payload
            .headers
            .iter()
            .find(|header| header.name.eq_ignore_ascii_case(name))
            .map(|header| header.value.as_str())
            .unwrap_or_default()
    };
    let received_at = parse_provider_time(header("Date"), message.internal_date.as_deref())?;
    let (sender_display, sender_address) = parse_address(header("From"));
    let important = message.label_ids.iter().any(|label| label == "IMPORTANT");
    let unread = message.label_ids.iter().any(|label| label == "UNREAD");
    let mut url = Url::parse("https://mail.google.com/mail/u/0/").expect("static Gmail URL");
    url.set_fragment(Some(&format!("inbox/{id}")));
    Some(InboxItem {
        id,
        source: "gmail".to_owned(),
        sender_display,
        sender_address,
        subject: bounded(header("Subject"), 180),
        preview: String::new(),
        received_at,
        unread,
        important,
        url: Some(url.into()),
    })
}

fn is_low_priority_gmail_category(message: &GmailMessage) -> bool {
    message.label_ids.iter().any(|label| {
        matches!(
            label.as_str(),
            "CATEGORY_PROMOTIONS" | "CATEGORY_SOCIAL" | "SPAM" | "TRASH"
        )
    })
}

fn parse_microsoft_message(message: MicrosoftMessage) -> Option<InboxItem> {
    let id = bounded(message.id, 180);
    let received_at = DateTime::parse_from_rfc3339(message.received_at.as_deref()?)
        .ok()?
        .to_rfc3339();
    if id.is_empty() {
        return None;
    }
    let address = message
        .from
        .and_then(|from| from.email_address)
        .unwrap_or(MicrosoftAddress {
            name: None,
            address: None,
        });
    Some(InboxItem {
        id,
        source: "microsoft_graph_mail".to_owned(),
        sender_display: bounded(address.name.unwrap_or_default(), 96),
        sender_address: bounded(address.address.unwrap_or_default(), 160),
        subject: bounded(message.subject.unwrap_or_default(), 180),
        preview: String::new(),
        received_at,
        unread: !message.is_read,
        important: message
            .importance
            .is_some_and(|value| value.eq_ignore_ascii_case("high")),
        url: message.web_link.as_deref().and_then(safe_url),
    })
}

fn finish_digest(
    provider: &str,
    unread_total: usize,
    mut items: Vec<InboxItem>,
    skipped: usize,
) -> InboxDigestView {
    items.sort_by(|left, right| {
        let left_priority = u8::from(left.important) * 2 + u8::from(left.unread);
        let right_priority = u8::from(right.important) * 2 + u8::from(right.unread);
        right_priority
            .cmp(&left_priority)
            .then_with(|| right.received_at.cmp(&left.received_at))
    });
    items.dedup_by(|left, right| left.source == right.source && left.id == right.id);
    items.truncate(MAX_INBOX_ITEMS);
    let (state, note) = if skipped > 0 {
        (
            "degraded",
            format!("{skipped} provider result(s) were unavailable or truncated"),
        )
    } else if items.is_empty() {
        (
            "empty",
            "No unread priority mail in the bounded window".to_owned(),
        )
    } else {
        ("ready", "Metadata-only inbox read".to_owned())
    };
    InboxDigestView {
        provider: provider.to_owned(),
        state: state.to_owned(),
        unread_total: unread_total.min(1_000_000),
        items,
        note,
    }
}

fn parse_provider_time(header: &str, epoch_millis: Option<&str>) -> Option<String> {
    DateTime::parse_from_rfc2822(header)
        .or_else(|_| DateTime::parse_from_rfc3339(header))
        .map(|value| value.to_rfc3339())
        .ok()
        .or_else(|| {
            let millis = epoch_millis?.parse::<i64>().ok()?;
            Utc.timestamp_millis_opt(millis)
                .single()
                .map(|value| value.to_rfc3339())
        })
}

fn parse_address(raw: &str) -> (String, String) {
    let value = bounded(raw, 220);
    let Some(open) = value.rfind('<') else {
        return (value.clone(), value);
    };
    let Some(close) = value[open..].find('>').map(|offset| open + offset) else {
        return (value.clone(), value);
    };
    let address = bounded(&value[open + 1..close], 160);
    let display = bounded(value[..open].trim().trim_matches('"'), 96);
    (
        if display.is_empty() {
            address.clone()
        } else {
            display
        },
        address,
    )
}

fn safe_url(raw: &str) -> Option<String> {
    if raw.len() > 2_048 {
        return None;
    }
    let value = Url::parse(raw).ok()?;
    is_safe_http_url(&value).then(|| value.into())
}

fn provider_identifier(raw: &str) -> Option<&str> {
    (!raw.is_empty()
        && raw.len() <= 180
        && raw
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_')))
    .then_some(raw)
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
    fn gmail_metadata_is_bounded_ranked_and_body_free() {
        let message: GmailMessage = serde_json::from_value(serde_json::json!({
            "id": "message-1",
            "internalDate": "1788600000000",
            "labelIds": ["INBOX", "UNREAD", "IMPORTANT"],
            "payload": {"headers": [
                {"name": "From", "value": "Dana Example <dana@example.test>"},
                {"name": "Subject", "value": "A".repeat(240)},
                {"name": "Date", "value": "Sat, 05 Sep 2026 12:00:00 +0000"}
            ]}
        }))
        .unwrap();
        let item = parse_gmail_message(message).unwrap();
        assert_eq!(item.sender_display, "Dana Example");
        assert_eq!(item.sender_address, "dana@example.test");
        assert!(item.subject.chars().count() <= 181);
        assert!(item.preview.is_empty());
        assert!(item.unread && item.important);
        assert!(item.url.unwrap().starts_with("https://mail.google.com/"));
    }

    #[test]
    fn gmail_low_priority_categories_are_excluded_without_reading_bodies() {
        let message: GmailMessage = serde_json::from_value(serde_json::json!({
            "id": "message-2",
            "internalDate": "1788600000000",
            "labelIds": ["INBOX", "UNREAD", "CATEGORY_PROMOTIONS"],
            "payload": {"headers": []}
        }))
        .unwrap();
        assert!(is_low_priority_gmail_category(&message));
    }

    #[test]
    fn microsoft_links_fail_closed_and_results_are_capped() {
        let mut items = (0..24)
            .map(|index| InboxItem {
                id: format!("id-{index}"),
                source: "microsoft_graph_mail".to_owned(),
                sender_display: "Sender".to_owned(),
                sender_address: "sender@example.test".to_owned(),
                subject: "Subject".to_owned(),
                preview: String::new(),
                received_at: format!("2026-09-05T12:{index:02}:00+00:00"),
                unread: true,
                important: index == 0,
                url: safe_url("javascript:alert(1)"),
            })
            .collect::<Vec<_>>();
        items.push(items[0].clone());
        let digest = finish_digest("microsoft_graph_mail", 25, items, 1);
        assert_eq!(digest.items.len(), MAX_INBOX_ITEMS);
        assert_eq!(digest.items[0].id, "id-0");
        assert!(digest.items.iter().all(|item| item.url.is_none()));
        assert_eq!(digest.state, "degraded");
    }

    #[test]
    fn provider_time_uses_bounded_epoch_fallback() {
        assert!(parse_provider_time("invalid", Some("1788600000000")).is_some());
        assert!(parse_provider_time("invalid", Some("invalid")).is_none());
    }

    #[test]
    fn gmail_provider_identifiers_cannot_change_the_request_target() {
        assert_eq!(provider_identifier("18f0aBc_42-7"), Some("18f0aBc_42-7"));
        assert!(provider_identifier("../messages/other?format=full").is_none());
        assert!(provider_identifier(&"a".repeat(181)).is_none());
    }
}

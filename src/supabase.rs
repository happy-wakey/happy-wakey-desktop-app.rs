use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use rand::Rng;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use url::Url;

fn supabase_url() -> String {
    std::env::var("SUPABASE_URL")
        .unwrap_or_else(|_| "https://vgzyyfhnendriyrhakkp.supabase.co".into())
}

fn anon_key() -> String {
    std::env::var("SUPABASE_ANON_KEY").unwrap_or_default()
}

fn require_anon_key() -> Result<String, String> {
    let key = anon_key();
    if key.trim().is_empty() {
        Err("SUPABASE_ANON_KEY is not configured".into())
    } else {
        Ok(key)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SupabaseSession {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_at: i64,
    pub user_id: String,
    pub email: Option<String>,
    /// Which OAuth provider this session came from ("google" | "apple" | "azure").
    pub provider: String,
    /// The provider's own OAuth access token (e.g. a Google or MS Graph token),
    /// which is what the calendar APIs actually require — the Supabase JWT will not work.
    pub provider_token: Option<String>,
    pub provider_refresh_token: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AuthTokenResponse {
    access_token: String,
    refresh_token: String,
    expires_in: i64,
    #[serde(default)]
    provider_token: Option<String>,
    #[serde(default)]
    provider_refresh_token: Option<String>,
    user: AuthUser,
}

#[derive(Debug, Deserialize)]
struct AuthUser {
    id: String,
    email: Option<String>,
}

pub fn generate_pkce() -> (String, String) {
    let verifier: Vec<u8> = (0..64).map(|_| rand::thread_rng().gen()).collect();
    let code_verifier = URL_SAFE_NO_PAD.encode(&verifier);
    let mut hasher = Sha256::new();
    hasher.update(code_verifier.as_bytes());
    let code_challenge = URL_SAFE_NO_PAD.encode(hasher.finalize());
    (code_verifier, code_challenge)
}

fn generate_nonce() -> String {
    let bytes: Vec<u8> = (0..32).map(|_| rand::thread_rng().gen()).collect();
    URL_SAFE_NO_PAD.encode(bytes)
}

pub fn normalize_provider(provider: &str) -> Result<String, String> {
    match provider.trim().to_lowercase().as_str() {
        "google" => Ok("google".into()),
        "apple" => Ok("apple".into()),
        "microsoft" | "azure" => Ok("azure".into()),
        _ => Err("Unsupported login provider".into()),
    }
}

/// Least-privilege provider OAuth scopes used by the native calendar and inbox
/// lanes. Gmail metadata excludes message bodies; Microsoft Mail.ReadBasic
/// excludes bodies, previews, attachments, and extended properties.
/// Returns `None` for providers without a usable calendar or mail API (Apple).
fn provider_scopes(provider: &str) -> Option<&'static str> {
    match provider {
        "google" => Some(
            "email profile https://www.googleapis.com/auth/calendar.readonly https://www.googleapis.com/auth/gmail.metadata",
        ),
        "azure" => Some(
            "email profile offline_access https://graph.microsoft.com/Calendars.Read https://graph.microsoft.com/Mail.ReadBasic",
        ),
        _ => None,
    }
}

/// The loopback port the OAuth redirect comes back on. A *fixed* port is required:
/// the resulting `http://127.0.0.1:<port>/callback` must be present in Supabase's
/// redirect allow-list, which is impossible with a random ephemeral port.
fn redirect_port() -> u16 {
    std::env::var("HAPPY_WAKEY_OAUTH_PORT")
        .ok()
        .and_then(|v| v.trim().parse::<u16>().ok())
        .filter(|p| *p != 0)
        .unwrap_or(47217)
}

pub fn login_with_provider(provider: &str) -> Result<SupabaseSession, String> {
    let provider = normalize_provider(provider)?;
    let (code_verifier, code_challenge) = generate_pkce();
    let state = generate_nonce();

    let port = redirect_port();
    let listener = TcpListener::bind(("127.0.0.1", port)).map_err(|e| {
        format!(
            "Couldn't start the local sign-in listener on 127.0.0.1:{port} ({e}). \
             Close whatever is using that port, or set HAPPY_WAKEY_OAUTH_PORT to a free \
             one and add http://127.0.0.1:<port>/callback to the Supabase redirect allow-list."
        )
    })?;
    let port = listener.local_addr().map_err(|e| e.to_string())?.port();
    let redirect_uri = format!("http://127.0.0.1:{port}/callback");

    let mut auth_url = Url::parse(&format!(
        "{}/auth/v1/authorize",
        supabase_url().trim_end_matches('/')
    ))
    .map_err(|e| format!("Invalid Supabase auth URL: {e}"))?;
    auth_url
        .query_pairs_mut()
        .append_pair("provider", &provider)
        .append_pair("redirect_to", &redirect_uri)
        .append_pair("code_challenge", &code_challenge)
        .append_pair("code_challenge_method", "s256")
        .append_pair("response_type", "code")
        .append_pair("state", &state);

    if let Some(scopes) = provider_scopes(&provider) {
        auth_url.query_pairs_mut().append_pair("scopes", scopes);
    }

    webbrowser::open(auth_url.as_str()).map_err(|e| format!("Failed to open browser: {e}"))?;

    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else {
                continue;
            };
            // Don't let a silent client wedge the read.
            let _ = stream.set_read_timeout(Some(Duration::from_secs(10)));

            match read_callback_code(&mut stream, &state) {
                // Browsers issue stray hits (e.g. /favicon.ico) to the loopback
                // server; ignore anything that isn't the real callback and keep
                // listening rather than aborting the whole sign-in.
                Err(CallbackError::NotCallback) => {
                    let _ = send_html(&mut stream, "404 Not Found", "Waiting for sign-in…");
                    continue;
                }
                outcome => {
                    let result = outcome.map_err(|e| e.into_message());
                    let body = if result.is_ok() {
                        "Authentication complete. You can close this window."
                    } else {
                        "Authentication failed. You can close this window and try again."
                    };
                    let _ = send_html(&mut stream, "200 OK", body);
                    let _ = tx.send(result);
                    break;
                }
            }
        }
    });

    match rx.recv_timeout(Duration::from_secs(300)) {
        Ok(Ok(code)) => {
            let mut session = exchange_code_for_session(&code_verifier, &code, &redirect_uri)?;
            session.provider = provider;
            Ok(session)
        }
        Ok(Err(e)) => Err(e),
        Err(_) => Err("Authentication timed out".into()),
    }
}

/// Distinguishes "this request wasn't the OAuth callback" (keep listening) from
/// a real failure that should end the sign-in attempt.
enum CallbackError {
    NotCallback,
    Message(String),
}

impl CallbackError {
    fn into_message(self) -> String {
        match self {
            CallbackError::NotCallback => "Unknown OAuth callback path".into(),
            CallbackError::Message(m) => m,
        }
    }
}

fn read_callback_code(
    stream: &mut TcpStream,
    expected_state: &str,
) -> Result<String, CallbackError> {
    let mut buf = [0u8; 8192];
    let n = stream
        .read(&mut buf)
        .map_err(|e| CallbackError::Message(format!("Failed to read OAuth callback: {e}")))?;
    let request = String::from_utf8_lossy(&buf[..n]);
    let target = request_target(&request).ok_or(CallbackError::NotCallback)?;

    if !target.starts_with("/callback?") {
        return Err(CallbackError::NotCallback);
    }

    let url = Url::parse(&format!("http://127.0.0.1{target}"))
        .map_err(|_| CallbackError::Message("Invalid OAuth callback URL".into()))?;
    parse_callback(url, expected_state).map_err(CallbackError::Message)
}

fn request_target(request: &str) -> Option<&str> {
    request.lines().next()?.split_whitespace().nth(1)
}

fn parse_callback(url: Url, expected_state: &str) -> Result<String, String> {
    let mut code = None;
    let mut state = None;
    let mut error = None;
    let mut error_description = None;

    for (key, value) in url.query_pairs() {
        match key.as_ref() {
            "code" => code = Some(value.to_string()),
            "state" => state = Some(value.to_string()),
            "error" => error = Some(value.to_string()),
            "error_description" => error_description = Some(value.to_string()),
            _ => {}
        }
    }

    if state.as_deref() != Some(expected_state) {
        return Err("OAuth state check failed".into());
    }

    if let Some(error) = error {
        return Err(error_description.unwrap_or(error));
    }

    code.ok_or_else(|| "OAuth callback did not include an authorization code".into())
}

fn send_html(stream: &mut TcpStream, status: &str, body: &str) -> std::io::Result<()> {
    let escaped = body
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;");
    let html = format!(
        "<!DOCTYPE html><html><head><meta charset=\"utf-8\"><title>Happy Wakey</title></head><body><p>{escaped}</p><script>window.close();</script></body></html>"
    );
    let response = format!(
        "HTTP/1.1 {status}\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        html.len(),
        html
    );
    stream.write_all(response.as_bytes())
}

fn exchange_code_for_session(
    code_verifier: &str,
    code: &str,
    redirect_uri: &str,
) -> Result<SupabaseSession, String> {
    let client = crate::http::shared_client();
    let params = [
        ("grant_type", "authorization_code"),
        ("code", code),
        ("redirect_uri", redirect_uri),
        ("code_verifier", code_verifier),
    ];

    let resp: AuthTokenResponse = client
        .post(format!("{}/auth/v1/token", supabase_url()))
        .header("apikey", require_anon_key()?)
        .form(&params)
        .send()
        .map_err(|e| format!("Token exchange failed: {e}"))?
        .error_for_status()
        .map_err(|e| format!("Token exchange rejected: {e}"))?
        .json()
        .map_err(|e| format!("Failed to parse token response: {e}"))?;

    let expires_at = unix_now()? + resp.expires_in;

    Ok(SupabaseSession {
        access_token: resp.access_token,
        refresh_token: resp.refresh_token,
        expires_at,
        user_id: resp.user.id,
        email: resp.user.email,
        provider: String::new(),
        provider_token: resp.provider_token,
        provider_refresh_token: resp.provider_refresh_token,
    })
}

#[allow(dead_code)]
pub fn refresh_session(refresh_token: &str) -> Result<SupabaseSession, String> {
    let params = [
        ("grant_type", "refresh_token"),
        ("refresh_token", refresh_token),
    ];

    let resp: AuthTokenResponse = crate::http::shared_client()
        .post(format!("{}/auth/v1/token", supabase_url()))
        .header("apikey", require_anon_key()?)
        .form(&params)
        .send()
        .map_err(|e| format!("Token refresh failed: {e}"))?
        .error_for_status()
        .map_err(|e| format!("Token refresh rejected: {e}"))?
        .json()
        .map_err(|e| format!("Failed to parse refresh response: {e}"))?;

    let expires_at = unix_now()? + resp.expires_in;

    Ok(SupabaseSession {
        access_token: resp.access_token,
        refresh_token: resp.refresh_token,
        expires_at,
        user_id: resp.user.id,
        email: resp.user.email,
        provider: String::new(),
        provider_token: resp.provider_token,
        provider_refresh_token: resp.provider_refresh_token,
    })
}

fn unix_now() -> Result<i64, String> {
    Ok(SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| format!("System clock error: {e}"))?
        .as_secs() as i64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_provider_maps_aliases() {
        assert_eq!(normalize_provider("Google").unwrap(), "google");
        assert_eq!(normalize_provider(" apple ").unwrap(), "apple");
        assert_eq!(normalize_provider("microsoft").unwrap(), "azure");
        assert_eq!(normalize_provider("azure").unwrap(), "azure");
        assert!(normalize_provider("facebook").is_err());
    }

    #[test]
    fn provider_scopes_cover_calendar_and_metadata_only_mail() {
        let google = provider_scopes("google").unwrap();
        assert!(google.contains("calendar.readonly"));
        assert!(google.contains("gmail.metadata"));
        assert!(!google.contains("gmail.readonly"));
        let microsoft = provider_scopes("azure").unwrap();
        assert!(microsoft.contains("Calendars.Read"));
        assert!(microsoft.contains("Mail.ReadBasic"));
        assert!(!microsoft.contains("Mail.ReadWrite"));
        assert!(provider_scopes("apple").is_none());
    }

    #[test]
    fn generate_pkce_challenge_is_s256_of_verifier() {
        let (verifier, challenge) = generate_pkce();
        let mut hasher = Sha256::new();
        hasher.update(verifier.as_bytes());
        let expected = URL_SAFE_NO_PAD.encode(hasher.finalize());
        assert_eq!(challenge, expected);
        // URL-safe base64 has no padding or +/ characters.
        assert!(!challenge.contains('=') && !challenge.contains('+') && !challenge.contains('/'));
        assert!(verifier.len() >= 43);
    }

    #[test]
    fn request_target_extracts_path() {
        let req = "GET /callback?code=abc&state=xyz HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n";
        assert_eq!(request_target(req), Some("/callback?code=abc&state=xyz"));
        assert_eq!(request_target(""), None);
    }

    fn cb(query: &str) -> Url {
        Url::parse(&format!("http://127.0.0.1/callback?{query}")).unwrap()
    }

    #[test]
    fn parse_callback_returns_code_on_state_match() {
        let url = cb("code=the-code&state=expected");
        assert_eq!(parse_callback(url, "expected").unwrap(), "the-code");
    }

    #[test]
    fn parse_callback_rejects_state_mismatch() {
        let url = cb("code=the-code&state=other");
        assert!(parse_callback(url, "expected").is_err());
    }

    #[test]
    fn parse_callback_surfaces_provider_error() {
        let url = cb("error=access_denied&error_description=User%20said%20no&state=expected");
        let err = parse_callback(url, "expected").unwrap_err();
        assert_eq!(err, "User said no");
    }

    #[test]
    fn parse_callback_missing_code_is_error() {
        let url = cb("state=expected");
        assert!(parse_callback(url, "expected").is_err());
    }

    #[test]
    fn stray_paths_are_not_treated_as_callback() {
        // A /favicon.ico style hit must be classified as "not the callback" so the
        // listener keeps waiting instead of aborting sign-in.
        let req = "GET /favicon.ico HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n";
        let target = request_target(req).unwrap();
        assert!(!target.starts_with("/callback?"));
    }
}

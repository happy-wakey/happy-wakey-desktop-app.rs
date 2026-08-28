use reqwest::blocking::{Client, RequestBuilder, Response};
use reqwest::{header::RETRY_AFTER, StatusCode};
use serde::de::DeserializeOwned;
use std::io::Read;
use std::sync::OnceLock;
use std::time::Duration;

const MAX_JSON_BYTES: usize = 2 * 1024 * 1024;
const MAX_GET_ATTEMPTS: usize = 3;

static CLIENT: OnceLock<Client> = OnceLock::new();

/// Process-wide client used only from worker threads.
pub fn shared_client() -> &'static Client {
    CLIENT.get_or_init(|| {
        Client::builder()
            .connect_timeout(Duration::from_secs(5))
            .timeout(Duration::from_secs(15))
            .pool_idle_timeout(Duration::from_secs(90))
            .tcp_keepalive(Duration::from_secs(30))
            .redirect(reqwest::redirect::Policy::limited(5))
            .user_agent(concat!(
                "happy-wakey/",
                env!("CARGO_PKG_VERSION"),
                " (+https://github.com/happy-wakey/happy-wakey-desktop-app.rs)"
            ))
            .build()
            .expect("failed to build the shared HTTP client")
    })
}

/// Execute an idempotent JSON request with bounded retries and response size.
pub fn get_json<T>(service: &str, request: RequestBuilder) -> Result<T, String>
where
    T: DeserializeOwned,
{
    for attempt in 0..MAX_GET_ATTEMPTS {
        let response = request
            .try_clone()
            .ok_or_else(|| format!("{service} request could not be retried"))?
            .send();

        match response {
            Ok(response)
                if is_transient_status(response.status()) && attempt + 1 < MAX_GET_ATTEMPTS =>
            {
                std::thread::sleep(retry_delay(&response, attempt));
            }
            Ok(response) => return parse_json_response(service, response),
            Err(error) if is_transient_error(&error) && attempt + 1 < MAX_GET_ATTEMPTS => {
                std::thread::sleep(backoff(attempt));
            }
            Err(error) => return Err(transport_error(service, &error)),
        }
    }

    Err(format!("{service} request failed after retries"))
}

/// Execute one JSON request without retrying a potentially non-idempotent operation.
pub fn send_json<T>(service: &str, request: RequestBuilder) -> Result<T, String>
where
    T: DeserializeOwned,
{
    let response = request
        .send()
        .map_err(|error| transport_error(service, &error))?;
    parse_json_response(service, response)
}

fn parse_json_response<T>(service: &str, mut response: Response) -> Result<T, String>
where
    T: DeserializeOwned,
{
    let status = response.status();
    if response
        .content_length()
        .is_some_and(|length| length > MAX_JSON_BYTES as u64)
    {
        return Err(format!("{service} returned an unexpectedly large response"));
    }

    let mut body = Vec::new();
    response
        .by_ref()
        .take(MAX_JSON_BYTES as u64 + 1)
        .read_to_end(&mut body)
        .map_err(|error| format!("{service} response read failed: {error}"))?;

    if body.len() > MAX_JSON_BYTES {
        return Err(format!("{service} returned an unexpectedly large response"));
    }

    if !status.is_success() {
        let detail = error_detail(&body);
        return if detail.is_empty() {
            Err(format!("{service} rejected the request ({status})"))
        } else {
            Err(format!(
                "{service} rejected the request ({status}): {detail}"
            ))
        };
    }

    serde_json::from_slice(&body)
        .map_err(|error| format!("{service} returned invalid JSON: {error}"))
}

fn is_transient_status(status: StatusCode) -> bool {
    status == StatusCode::TOO_MANY_REQUESTS
        || status == StatusCode::REQUEST_TIMEOUT
        || status.is_server_error()
}

fn is_transient_error(error: &reqwest::Error) -> bool {
    error.is_timeout() || error.is_connect()
}

fn transport_error(service: &str, error: &reqwest::Error) -> String {
    if error.is_timeout() {
        format!("{service} timed out")
    } else if error.is_connect() {
        format!("{service} could not be reached")
    } else {
        format!("{service} request failed: {error}")
    }
}

fn retry_delay(response: &Response, attempt: usize) -> Duration {
    response
        .headers()
        .get(RETRY_AFTER)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse::<u64>().ok())
        .map(|seconds| Duration::from_secs(seconds.min(2)))
        .unwrap_or_else(|| backoff(attempt))
}

fn backoff(attempt: usize) -> Duration {
    Duration::from_millis(150 * (attempt as u64 + 1))
}

fn error_detail(body: &[u8]) -> String {
    let structured = serde_json::from_slice::<serde_json::Value>(body)
        .ok()
        .and_then(|value| {
            ["message", "error_description", "error", "reason"]
                .into_iter()
                .find_map(|key| value.get(key).and_then(|item| item.as_str()))
                .map(ToOwned::to_owned)
        });
    let raw = structured.unwrap_or_else(|| String::from_utf8_lossy(body).into_owned());

    raw.chars()
        .filter(|ch| !ch.is_control())
        .take(240)
        .collect::<String>()
        .trim()
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    #[test]
    fn extracts_bounded_structured_error_details() {
        let body = br#"{"message":"rate limited\ntry later","secret":"ignored"}"#;
        assert_eq!(error_detail(body), "rate limitedtry later");
        assert!(error_detail(&vec![b'x'; 500]).len() <= 240);
    }

    #[test]
    fn retries_transient_get_then_parses_json() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind test server");
        let address = listener.local_addr().expect("test server address");
        let calls = Arc::new(AtomicUsize::new(0));
        let server_calls = Arc::clone(&calls);

        let server = std::thread::spawn(move || {
            for index in 0..2 {
                let (mut stream, _) = listener.accept().expect("accept request");
                let mut request = [0_u8; 1024];
                let _ = stream.read(&mut request);
                server_calls.fetch_add(1, Ordering::SeqCst);
                let response = if index == 0 {
                    "HTTP/1.1 503 Service Unavailable\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
                } else {
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 11\r\nConnection: close\r\n\r\n{\"ok\":true}"
                };
                stream
                    .write_all(response.as_bytes())
                    .expect("write response");
            }
        });

        #[derive(serde::Deserialize)]
        struct TestResponse {
            ok: bool,
        }

        let value: TestResponse = get_json(
            "Test service",
            shared_client().get(format!("http://{address}/test")),
        )
        .expect("request should recover");

        server.join().expect("join test server");
        assert!(value.ok);
        assert_eq!(calls.load(Ordering::SeqCst), 2);
    }
}

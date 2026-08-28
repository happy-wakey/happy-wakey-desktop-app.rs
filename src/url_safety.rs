use std::net::IpAddr;

use url::Url;

fn canonicalize_host(host: &str) -> &str {
    host.strip_prefix('[')
        .and_then(|inner| inner.strip_suffix(']'))
        .unwrap_or(host)
}

pub fn is_loopback_host(host: &str) -> bool {
    matches!(canonicalize_host(host), "127.0.0.1" | "localhost" | "::1")
}

pub fn is_numeric_ip_host(host: &str) -> bool {
    let host = canonicalize_host(host);
    host.parse::<IpAddr>().is_ok() || host.contains(':')
}

/// HTTPS to a DNS name, or HTTP(S) to loopback. Public IP literals fail closed.
pub fn is_safe_http_url(url: &Url) -> bool {
    let Some(host) = url.host_str() else {
        return false;
    };
    if !url.username().is_empty() || url.password().is_some() {
        return false;
    }
    let loopback = is_loopback_host(host);
    match url.scheme() {
        "http" => loopback,
        "https" => loopback || !is_numeric_ip_host(host),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_public_ip_literals_and_cleartext_internet() {
        assert!(!is_safe_http_url(
            &Url::parse("https://98.90.186.114/").unwrap()
        ));
        assert!(!is_safe_http_url(
            &Url::parse("http://example.test/").unwrap()
        ));
        assert!(!is_safe_http_url(
            &Url::parse("https://[2001:db8::1]/").unwrap()
        ));
        assert!(!is_safe_http_url(
            &Url::parse("https://user:pass@example.test/").unwrap()
        ));
        assert!(!Url::parse("javascript:alert(1)")
            .map(|url| is_safe_http_url(&url))
            .unwrap_or(false));
        assert!(!is_safe_http_url(
            &Url::parse("file:///etc/passwd").unwrap()
        ));
        assert!(is_safe_http_url(
            &Url::parse("https://example.test/v1").unwrap()
        ));
        assert!(is_safe_http_url(
            &Url::parse("http://127.0.0.1:8128/").unwrap()
        ));
        assert!(is_safe_http_url(&Url::parse("https://127.0.0.1/").unwrap()));
        assert!(is_safe_http_url(&Url::parse("http://[::1]/").unwrap()));
        assert!(is_safe_http_url(
            &Url::parse("https://[::1]/healthz").unwrap()
        ));
    }
}

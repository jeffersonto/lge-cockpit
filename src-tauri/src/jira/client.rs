//! Reqwest-based adapter that talks to the Jira Cloud REST API.

use std::time::Duration;

use reqwest::{header, StatusCode};

use super::converter::html_to_markdown;
use super::{JiraConfig, JiraError, JiraIssue, JiraSelf};

/// Port: anything that can fetch Jira data. The real adapter is
/// `ReqwestJiraClient`; tests can supply a fake.
pub trait JiraClient {
    fn get_issue(&self, key: &str) -> impl std::future::Future<Output = Result<JiraIssue, JiraError>> + Send;
    fn verify_connection(&self) -> impl std::future::Future<Output = Result<JiraSelf, JiraError>> + Send;
}

/// Production adapter backed by `reqwest`. Holds a pre-built HTTP client
/// and the resolved Jira config.
#[derive(Debug)]
pub struct ReqwestJiraClient {
    config: JiraConfig,
    http: reqwest::Client,
}

impl ReqwestJiraClient {
    pub fn new(config: JiraConfig) -> Result<Self, JiraError> {
        if !config.is_complete() {
            return Err(JiraError::NotConfigured);
        }
        // Surface a missing URL scheme explicitly rather than letting
        // reqwest fail later with a generic "relative URL without base"
        // network error that's hard for the user to map back to Settings.
        let scheme = config.base_url_trimmed();
        if !scheme.starts_with("http://") && !scheme.starts_with("https://") {
            return Err(JiraError::InvalidBaseUrl(config.base_url.clone()));
        }
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|e| JiraError::Network(e.to_string()))?;
        Ok(Self { config, http })
    }

    fn issue_url(&self, key: &str) -> String {
        format!(
            "{}/rest/api/3/issue/{}?fields=summary,status,description&expand=renderedFields",
            self.config.base_url_trimmed(),
            urlencoding::encode_path(key),
        )
    }

    fn myself_url(&self) -> String {
        format!("{}/rest/api/3/myself", self.config.base_url_trimmed())
    }

    async fn get_text(&self, url: &str) -> Result<String, JiraError> {
        let resp = self
            .http
            .get(url)
            .basic_auth(&self.config.email, Some(&self.config.api_token))
            .header(header::ACCEPT, "application/json")
            .send()
            .await
            .map_err(|e| JiraError::Network(e.to_string()))?;
        let status = resp.status();
        let body = resp
            .text()
            .await
            .map_err(|e| JiraError::Network(e.to_string()))?;
        match status {
            StatusCode::OK => Ok(body),
            StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => Err(JiraError::Unauthorized),
            StatusCode::NOT_FOUND => Err(JiraError::NotFound(url.to_string())),
            StatusCode::TOO_MANY_REQUESTS => Err(JiraError::RateLimited),
            s => Err(JiraError::UnexpectedStatus {
                status: s.as_u16(),
                body,
            }),
        }
    }
}

impl JiraClient for ReqwestJiraClient {
    async fn get_issue(&self, key: &str) -> Result<JiraIssue, JiraError> {
        validate_key(key)?;
        let url = self.issue_url(key);
        let body = self.get_text(&url).await?;
        let parsed = parse_issue_response(&body, key, &self.config.base_url)?;
        Ok(parsed)
    }

    async fn verify_connection(&self) -> Result<JiraSelf, JiraError> {
        let url = self.myself_url();
        let body = self.get_text(&url).await?;
        let value: serde_json::Value = serde_json::from_str(&body)
            .map_err(|e| JiraError::Parse(format!("myself: {e}")))?;
        Ok(JiraSelf {
            account_id: value
                .get("accountId")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            display_name: value
                .get("displayName")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            email: value
                .get("emailAddress")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
        })
    }
}

/// Separated from the adapter so it can be unit-tested without HTTP.
fn parse_issue_response(body: &str, key: &str, raw_base_url: &str) -> Result<JiraIssue, JiraError> {
    let value: serde_json::Value =
        serde_json::from_str(body).map_err(|e| JiraError::Parse(format!("issue JSON: {e}")))?;

    let resolved_key = value
        .get("key")
        .and_then(|v| v.as_str())
        .unwrap_or(key)
        .to_string();
    let fields = value.get("fields").cloned().unwrap_or(serde_json::Value::Null);

    let summary = fields
        .get("summary")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| format!("[{key}]"));

    let status = fields
        .get("status")
        .and_then(|s| s.get("name"))
        .and_then(|n| n.as_str())
        .map(|s| s.to_string());

    // Prefer rendered HTML (Atlassian-rendered); fall back to None.
    let description_html = value
        .get("renderedFields")
        .and_then(|r| r.get("description"))
        .and_then(|d| d.as_str());
    let description = description_html.and_then(html_to_markdown);

    // Build the browse URL from the configured base.
    let browse = if !raw_base_url.trim().is_empty() {
        format!(
            "{}/browse/{}",
            raw_base_url.trim().trim_end_matches('/'),
            resolved_key
        )
    } else {
        format!("/{resolved_key}")
    };

    Ok(JiraIssue {
        key: resolved_key,
        summary,
        description,
        status,
        url: browse,
    })
}

/// Rejects obviously malformed keys before we hit the network. Accepts
/// the canonical `PROJECT-123` form, case-insensitive on input.
fn validate_key(key: &str) -> Result<(), JiraError> {
    let k = key.trim();
    if k.is_empty() {
        return Err(JiraError::InvalidKey(key.to_string()));
    }
    let mut parts = k.split('-');
    let project = parts.next();
    let number = parts.next();
    let rest = parts.next();
    let project = match project {
        Some(p) if !p.is_empty() && p.chars().all(|c| c.is_ascii_alphanumeric()) => p,
        _ => return Err(JiraError::InvalidKey(key.to_string())),
    };
    let number = match number {
        Some(n) if !n.is_empty() && n.chars().all(|c| c.is_ascii_digit()) => n,
        _ => return Err(JiraError::InvalidKey(key.to_string())),
    };
    if rest.is_some() {
        return Err(JiraError::InvalidKey(key.to_string()));
    }
    // Silence unused warnings while keeping the values documented.
    let _ = (project, number);
    Ok(())
}

/// Minimal path-segment encoder so the key may be safely placed in the URL
/// path without pulling in another crate. Jira keys are alphanumeric plus
/// hyphen, so this only encodes the rare stray character. Non-ASCII bytes are
/// percent-encoded per UTF-8 so the encoder stays correct for arbitrary input
/// (it never silently truncates a `char > 255`).
mod urlencoding {
    pub fn encode_path(segment: &str) -> String {
        let mut out = String::with_capacity(segment.len());
        for c in segment.chars() {
            if c.is_ascii_alphanumeric() || c == '-' {
                out.push(c);
            } else {
                let mut buf = [0u8; 4];
                for b in c.encode_utf8(&mut buf).as_bytes() {
                    out.push_str(&format!("%{:02X}", b));
                }
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_garbage_keys() {
        assert!(matches!(validate_key(""), Err(JiraError::InvalidKey(_))));
        assert!(matches!(validate_key("abc"), Err(JiraError::InvalidKey(_))));
        assert!(matches!(validate_key("ABC-"), Err(JiraError::InvalidKey(_))));
        assert!(matches!(validate_key("ABC-1-2"), Err(JiraError::InvalidKey(_))));
        assert!(matches!(validate_key("ABC DEF-1"), Err(JiraError::InvalidKey(_))));
    }

    #[test]
    fn accepts_well_formed_keys() {
        assert!(validate_key("PROJ-123").is_ok());
        assert!(validate_key("ab-1").is_ok());
    }

    #[test]
    fn new_rejects_missing_scheme() {
        let cfg = JiraConfig {
            base_url: "yourcompany.atlassian.net".to_string(),
            email: "u@example.com".to_string(),
            api_token: "tok".to_string(),
        };
        let err = ReqwestJiraClient::new(cfg).unwrap_err();
        assert!(matches!(err, JiraError::InvalidBaseUrl(_)));
    }

    #[test]
    fn new_accepts_https_scheme() {
        let cfg = JiraConfig {
            base_url: "https://yourcompany.atlassian.net".to_string(),
            email: "u@example.com".to_string(),
            api_token: "tok".to_string(),
        };
        assert!(ReqwestJiraClient::new(cfg).is_ok());
    }

    #[test]
    fn new_rejects_unset_credentials_with_not_configured() {
        let cfg = JiraConfig {
            base_url: "https://x.atlassian.net".to_string(),
            email: String::new(),
            api_token: "tok".to_string(),
        };
        let err = ReqwestJiraClient::new(cfg).unwrap_err();
        assert!(matches!(err, JiraError::NotConfigured));
    }

    #[test]
    fn parses_summary_and_status() {
        let body = r#"{
            "key": "PROJ-1",
            "fields": {
                "summary": "Do the thing",
                "status": { "name": "In Progress" }
            },
            "renderedFields": { "description": "<p>Hello</p>" }
        }"#;
        let issue = parse_issue_response(body, "PROJ-1", "https://x.atlassian.net").unwrap();
        assert_eq!(issue.key, "PROJ-1");
        assert_eq!(issue.summary, "Do the thing");
        assert_eq!(issue.status.as_deref(), Some("In Progress"));
        assert!(issue.description.as_deref().unwrap().contains("Hello"));
        assert_eq!(issue.url, "https://x.atlassian.net/browse/PROJ-1");
    }

    #[test]
    fn encode_path_preserves_ascii_and_encodes_utf8() {
        assert_eq!(urlencoding::encode_path("PROJ-123"), "PROJ-123");
        // Space and reserved punctuation get percent-encoded.
        assert_eq!(urlencoding::encode_path("a b/c"), "a%20b%2Fc");
        // Non-ASCII is encoded per UTF-8 bytes, not truncated.
        // 'ç' = U+00E7 -> UTF-8 0xC3 0xA7
        assert_eq!(urlencoding::encode_path("caça"), "ca%C3%A7a");
    }

    #[test]
    fn summary_falls_back_to_key() {
        let body = r#"{ "key": "X-1", "fields": {} }"#;
        let issue = parse_issue_response(body, "X-1", "https://x.atlassian.net").unwrap();
        assert_eq!(issue.summary, "[X-1]");
        assert!(issue.description.is_none());
    }
}

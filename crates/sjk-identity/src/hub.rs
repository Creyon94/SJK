//! Talking to the hub: the [`Hub`] operations and their HTTPS implementation.

use crate::keys::{Identity, random_bytes};
use crate::report::{BugReport, WorldNote};
use crate::wire::{Presence, Profile, authorization};
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Longest the hub may take to answer.
const TIMEOUT: Duration = Duration::from_secs(10);
/// Largest answer read, in bytes.
const ANSWER_MAX: u64 = 256 * 1024;

/// Why a hub request failed.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum HubError {
    /// The hub could not be reached.
    Network(String),
    /// The hub answered with an error.
    Rejected {
        /// HTTP status.
        status: u16,
        /// The hub's machine-readable code (`name_taken`, `slot_taken`, ...).
        code: String,
        /// The hub's text.
        message: String,
    },
    /// The answer was not what `PROTOCOL.md` describes.
    Protocol(String),
}

impl std::fmt::Display for HubError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Network(why) => write!(f, "cannot reach the hub: {why}"),
            Self::Rejected { message, code, .. } => {
                write!(f, "the hub refused it ({code}): {message}")
            }
            Self::Protocol(why) => write!(f, "unexpected answer from the hub: {why}"),
        }
    }
}

impl std::error::Error for HubError {}

/// What the client asks of a hub (`PROTOCOL.md`, "Endpoints").
pub trait Hub: Send {
    /// Register the identity's key, or fetch its profile if it is known, telling
    /// the hub the in-game `name` the player wears, which it keeps in the key's
    /// name history.
    fn register(&mut self, identity: &Identity, name: Option<&str>) -> Result<Profile, HubError>;
    /// Set the identity's bio (the name is the one worn in game).
    fn set_bio(&mut self, identity: &Identity, bio: &str) -> Result<Profile, HubError>;
    /// Any player's public profile.
    fn profile(&mut self, key_id: &str) -> Result<Profile, HubError>;
    /// Say the identity's player is in `slot` of `server` as `name`.
    fn claim(
        &mut self,
        identity: &Identity,
        server: &str,
        slot: u8,
        name: &str,
    ) -> Result<(), HubError>;
    /// Withdraw the identity's claim on `server`.
    fn release(&mut self, identity: &Identity, server: &str) -> Result<(), HubError>;
    /// The live claims on `server`.
    fn presence(&mut self, server: &str) -> Result<Vec<Presence>, HubError>;
    /// Send a world note signed by the identity; the hub answers with its number.
    fn note(&mut self, _identity: &Identity, _note: &WorldNote) -> Result<i64, HubError> {
        Err(HubError::Protocol(
            "this hub client does not send notes".to_owned(),
        ))
    }
    /// Attach a JPEG to note `id`, signed by the identity that sent the note.
    fn note_image(&mut self, _identity: &Identity, _id: i64, _jpeg: &[u8]) -> Result<(), HubError> {
        Err(HubError::Protocol(
            "this hub client does not send pictures".to_owned(),
        ))
    }
    /// Send a bug report signed by the identity; the hub answers with its number.
    fn report(&mut self, _identity: &Identity, _report: &BugReport) -> Result<i64, HubError> {
        Err(HubError::Protocol(
            "this hub client does not send reports".to_owned(),
        ))
    }
}

/// Whether `url` may be used as the hub's address: `https://host[:port]`, or
/// plain `http://` only for this machine (local testing). No path: requests are
/// signed with their path, which must be the one the hub sees.
pub fn valid_base_url(url: &str) -> bool {
    let (secure, authority) = if let Some(rest) = url.strip_prefix("https://") {
        (true, rest)
    } else if let Some(rest) = url.strip_prefix("http://") {
        (false, rest)
    } else {
        return false;
    };
    let authority = authority.strip_suffix('/').unwrap_or(authority);
    if authority.is_empty() || authority.contains(['/', '?', '#', '@', ' ']) {
        return false;
    }
    let host = match authority.strip_prefix('[') {
        Some(v6) => v6.split(']').next().unwrap_or_default(),
        None => authority.split(':').next().unwrap_or_default(),
    };
    !host.is_empty() && (secure || matches!(host, "localhost" | "127.0.0.1" | "::1"))
}

fn unix_now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_secs() as i64)
}

/// The hub over HTTPS.
pub struct HttpHub {
    base: String,
    agent: ureq::Agent,
    /// Added to the local clock to get the hub's, learned from a `clock` refusal.
    clock_offset: i64,
}

impl HttpHub {
    /// A client for the hub at `base_url` (`https://hub.example`, no trailing slash needed).
    pub fn new(base_url: &str, user_agent: &str) -> Result<Self, HubError> {
        let base = base_url.trim().trim_end_matches('/').to_owned();
        if !valid_base_url(&base) {
            return Err(HubError::Network(
                "the hub address must start with https://".to_owned(),
            ));
        }
        let agent = ureq::Agent::config_builder()
            .timeout_global(Some(TIMEOUT))
            .user_agent(user_agent)
            .http_status_as_error(false)
            .build()
            .into();
        Ok(Self {
            base,
            agent,
            clock_offset: 0,
        })
    }

    /// One request, signed when `signer` is given. Retries once with the hub's
    /// clock when it refuses the timestamp.
    fn send(
        &mut self,
        signer: Option<&Identity>,
        method: &str,
        path: &str,
        body: Option<Value>,
    ) -> Result<Value, HubError> {
        let body = body.map(|value| value.to_string()).unwrap_or_default();
        self.send_bytes(signer, method, path, body.as_bytes(), "application/json")
    }

    /// [`HttpHub::send`] with a raw body of `content_type`.
    fn send_bytes(
        &mut self,
        signer: Option<&Identity>,
        method: &str,
        path: &str,
        body: &[u8],
        content_type: &str,
    ) -> Result<Value, HubError> {
        for attempt in 0..2 {
            let url = format!("{}{path}", self.base);
            let nonce =
                random_bytes::<12>().map_err(|error| HubError::Network(error.to_string()))?;
            let header = signer.map(|identity| {
                authorization(
                    identity,
                    method,
                    path,
                    body,
                    unix_now() + self.clock_offset,
                    nonce,
                )
            });
            let mut response = self.call(method, &url, header.as_deref(), body, content_type)?;
            let status = response.status().as_u16();
            let text = response
                .body_mut()
                .with_config()
                .limit(ANSWER_MAX)
                .read_to_string()
                .map_err(|error| HubError::Network(error.to_string()))?;
            let value: Value = serde_json::from_str(&text)
                .map_err(|_| HubError::Protocol(format!("status {status}, not JSON")))?;
            if status < 400 {
                return Ok(value);
            }
            let code = value["error"].as_str().unwrap_or("error").to_owned();
            if code == "clock"
                && attempt == 0
                && let Some(server_time) = value["server_time"].as_i64()
            {
                self.clock_offset = server_time - unix_now();
                continue;
            }
            return Err(HubError::Rejected {
                status,
                code,
                message: value["message"].as_str().unwrap_or_default().to_owned(),
            });
        }
        Err(HubError::Protocol(
            "the hub's clock keeps disagreeing".to_owned(),
        ))
    }

    fn call(
        &self,
        method: &str,
        url: &str,
        authorization: Option<&str>,
        body: &[u8],
        content_type: &str,
    ) -> Result<ureq::http::Response<ureq::Body>, HubError> {
        let network = |error: ureq::Error| HubError::Network(error.to_string());
        let with_header =
            |builder: ureq::RequestBuilder<ureq::typestate::WithBody>| match authorization {
                Some(value) => builder.header("Authorization", value),
                None => builder,
            };
        match method {
            "GET" => {
                let mut request = self.agent.get(url);
                if let Some(value) = authorization {
                    request = request.header("Authorization", value);
                }
                request.call().map_err(network)
            }
            "DELETE" => {
                let mut request = self.agent.delete(url).force_send_body();
                if let Some(value) = authorization {
                    request = request.header("Authorization", value);
                }
                request
                    .header("Content-Type", content_type)
                    .send(body)
                    .map_err(network)
            }
            "PUT" => with_header(self.agent.put(url))
                .header("Content-Type", content_type)
                .send(body)
                .map_err(network),
            _ => with_header(self.agent.post(url))
                .header("Content-Type", content_type)
                .send(body)
                .map_err(network),
        }
    }
}

fn parse<T: DeserializeOwned>(value: Value) -> Result<T, HubError> {
    serde_json::from_value(value).map_err(|error| HubError::Protocol(error.to_string()))
}

impl Hub for HttpHub {
    fn register(&mut self, identity: &Identity, name: Option<&str>) -> Result<Profile, HubError> {
        let body = match name {
            Some(name) => json!({ "name": name }),
            None => json!({}),
        };
        parse(self.send(Some(identity), "POST", "/v1/register", Some(body))?)
    }

    fn report(&mut self, identity: &Identity, report: &BugReport) -> Result<i64, HubError> {
        let mut body = json!({
            "text": report.text,
            "map": report.map,
            "build": report.build,
            "server": report.server,
        });
        if !report.name.is_empty() {
            body["name"] = json!(report.name);
        }
        let answer = self.send(Some(identity), "POST", "/v1/report", Some(body))?;
        answer
            .get("id")
            .and_then(serde_json::Value::as_i64)
            .ok_or_else(|| HubError::Protocol("the report answer has no id".to_owned()))
    }

    fn note(&mut self, identity: &Identity, note: &WorldNote) -> Result<i64, HubError> {
        let mut body = json!({
            "text": note.text,
            "map": note.map,
            "build": note.build,
            "server": note.server,
            "view": note.view,
            "hit": note.hit,
            "normal": note.normal,
            "shader": note.shader,
            "surface": note.surface,
            "lighting": note.lighting,
            "distance": note.distance,
            "entity": note.entity,
        });
        if !note.name.is_empty() {
            body["name"] = json!(note.name);
        }
        let answer = self.send(Some(identity), "POST", "/v1/note", Some(body))?;
        answer
            .get("id")
            .and_then(serde_json::Value::as_i64)
            .ok_or_else(|| HubError::Protocol("the note answer has no id".to_owned()))
    }

    fn note_image(&mut self, identity: &Identity, id: i64, jpeg: &[u8]) -> Result<(), HubError> {
        let path = format!("/v1/note/{id}/image");
        self.send_bytes(Some(identity), "PUT", &path, jpeg, "image/jpeg")
            .map(|_| ())
    }

    fn set_bio(&mut self, identity: &Identity, bio: &str) -> Result<Profile, HubError> {
        let body = json!({ "bio": bio });
        parse(self.send(Some(identity), "PUT", "/v1/profile", Some(body))?)
    }

    fn profile(&mut self, key_id: &str) -> Result<Profile, HubError> {
        if key_id.len() != 16 || !key_id.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err(HubError::Protocol("a key id is 16 hex digits".to_owned()));
        }
        parse(self.send(None, "GET", &format!("/v1/profile/{key_id}"), None)?)
    }

    fn claim(
        &mut self,
        identity: &Identity,
        server: &str,
        slot: u8,
        name: &str,
    ) -> Result<(), HubError> {
        let body = json!({"server": server, "slot": slot, "name": name});
        self.send(Some(identity), "POST", "/v1/claim", Some(body))
            .map(|_| ())
    }

    fn release(&mut self, identity: &Identity, server: &str) -> Result<(), HubError> {
        self.send(
            Some(identity),
            "DELETE",
            "/v1/claim",
            Some(json!({"server": server})),
        )
        .map(|_| ())
    }

    fn presence(&mut self, server: &str) -> Result<Vec<Presence>, HubError> {
        let query: String = server
            .bytes()
            .map(|byte| match byte {
                b'0'..=b'9' | b'.' => (byte as char).to_string(),
                _ => format!("%{byte:02X}"),
            })
            .collect();
        let answer = self.send(None, "GET", &format!("/v1/presence?server={query}"), None)?;
        parse(answer["players"].clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_hub_address_must_be_https_or_local() {
        assert!(valid_base_url("https://hub.example"));
        assert!(valid_base_url("https://hub.example:8443/"));
        assert!(valid_base_url("http://localhost:8787"));
        assert!(valid_base_url("http://127.0.0.1:8787"));
        assert!(valid_base_url("http://[::1]:8787"));
        assert!(!valid_base_url("https://hub.example/prefix"));
        assert!(!valid_base_url("http://127.0.0.1.evil.example"));
        assert!(!valid_base_url("http://localhost.evil.example:80"));
        assert!(!valid_base_url("http://hub.example"));
        assert!(!valid_base_url("hub.example"));
        assert!(!valid_base_url("https://"));
        assert!(!valid_base_url("https://user@hub.example"));
        assert!(!valid_base_url("ftp://hub.example"));
        assert!(!valid_base_url(""));
    }

    #[test]
    fn a_plain_http_address_is_refused_at_construction() {
        assert!(HttpHub::new("http://hub.example", "t").is_err());
        assert!(HttpHub::new("https://hub.example/", "t").is_ok());
    }

    #[test]
    fn a_key_id_is_checked_before_a_request_is_made() {
        let mut hub = HttpHub::new("https://hub.example", "t").unwrap();
        assert!(matches!(hub.profile("../etc"), Err(HubError::Protocol(_))));
    }
}

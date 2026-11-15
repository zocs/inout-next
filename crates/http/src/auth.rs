use crate::routes::ApiError;
use http::{HeaderMap, Uri};
use sha2::{Digest, Sha256};
use share_domain::Error;
use std::{
    collections::HashMap,
    sync::Mutex,
    time::{Duration, Instant},
};
use subtle::ConstantTimeEq;

const COOKIE: &str = "share_session";
const SESSION_TTL: Duration = Duration::from_secs(12 * 60 * 60);

pub(crate) struct Auth {
    key_hash: Option<[u8; 32]>,
    sessions: Mutex<HashMap<[u8; 32], Instant>>,
}
fn hash(value: &str) -> [u8; 32] {
    Sha256::digest(value.as_bytes()).into()
}
impl Auth {
    pub fn new(key: Option<&str>) -> Self {
        Self {
            key_hash: key.map(hash),
            sessions: Mutex::new(HashMap::new()),
        }
    }
    pub fn required(&self) -> bool {
        self.key_hash.is_some()
    }
    pub fn verify(&self, key: &str) -> bool {
        self.key_hash
            .is_some_and(|expected| bool::from(hash(key).ct_eq(&expected)))
    }
    pub fn authorize(&self, headers: &HeaderMap, mutation: bool) -> Result<(), ApiError> {
        if mutation && headers.contains_key(http::header::ORIGIN) {
            require_origin(headers)?;
        }
        if !self.required() {
            return Ok(());
        }
        if let Some(value) = headers.get(http::header::AUTHORIZATION) {
            let bearer = value
                .to_str()
                .ok()
                .and_then(|value| value.strip_prefix("Bearer "));
            return if bearer.is_some_and(|key| self.verify(key)) {
                Ok(())
            } else {
                Err(Error::Unauthorized.into())
            };
        }
        let token = cookie_token(headers).ok_or(Error::Unauthorized)?;
        let mut sessions = self.sessions.lock().map_err(|_| Error::Conflict)?;
        sessions.retain(|_, created| created.elapsed() < SESSION_TTL);
        if !sessions.contains_key(&hash(token)) {
            return Err(Error::Unauthorized.into());
        }
        if mutation {
            require_origin(headers)?;
        }
        Ok(())
    }
    pub fn login(&self, headers: &HeaderMap, key: &str) -> Result<String, ApiError> {
        if headers.contains_key(http::header::ORIGIN) {
            require_origin(headers)?;
        }
        if !self.verify(key) {
            return Err(Error::Unauthorized.into());
        }
        let token = format!(
            "{:032x}{:032x}",
            rand::random::<u128>(),
            rand::random::<u128>()
        );
        let mut sessions = self.sessions.lock().map_err(|_| Error::Conflict)?;
        sessions.retain(|_, created| created.elapsed() < SESSION_TTL);
        if sessions.len() >= 1024 {
            return Err(Error::Conflict.into());
        }
        sessions.insert(hash(&token), Instant::now());
        Ok(format!(
            "{COOKIE}={token}; Path=/api/v1; HttpOnly; SameSite=Strict; Max-Age=43200"
        ))
    }
    pub fn logout(&self, headers: &HeaderMap) -> Result<&'static str, ApiError> {
        // Logged-out clients can clear an expired cookie; active sessions still
        // require same-origin mutation authorization.
        if let Some(token) = cookie_token(headers) {
            let mut sessions = self.sessions.lock().map_err(|_| Error::Conflict)?;
            if sessions.contains_key(&hash(token)) {
                require_origin(headers)?;
            }
            sessions.remove(&hash(token));
        }
        Ok("share_session=; Path=/api/v1; HttpOnly; SameSite=Strict; Max-Age=0")
    }
}
fn cookie_token(headers: &HeaderMap) -> Option<&str> {
    headers
        .get(http::header::COOKIE)?
        .to_str()
        .ok()?
        .split(';')
        .filter_map(|part| part.trim().split_once('='))
        .find_map(|(name, value)| (name == COOKIE).then_some(value))
}
fn require_origin(headers: &HeaderMap) -> Result<(), ApiError> {
    let origin: Uri = headers
        .get(http::header::ORIGIN)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse().ok())
        .ok_or(Error::Forbidden)?;
    let host = headers
        .get(http::header::HOST)
        .and_then(|value| value.to_str().ok())
        .ok_or(Error::Forbidden)?;
    if origin.scheme_str() != Some("http")
        || origin
            .authority()
            .is_none_or(|authority| !authority.as_str().eq_ignore_ascii_case(host))
        || origin
            .path_and_query()
            .is_some_and(|path| path.as_str() != "/" && !path.as_str().is_empty())
    {
        return Err(Error::Forbidden.into());
    }
    Ok(())
}

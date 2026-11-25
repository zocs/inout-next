use crate::routes::ApiError;
use axum::{
    body::Body,
    extract::Request,
    response::{IntoResponse, Response},
};
use bytes::Bytes;
use http::{Method, StatusCode};
use share_domain::{Error, RelativePath};
use share_storage::{WebAsset, MAX_ASSET_BYTES, MAX_ASSET_FILES};
use std::{collections::HashMap, sync::Arc};

/// Immutable public pages held in memory for the server lifetime. A bundle
/// compiled into the host needs no directory beside the executable, so the
/// shared folder may contain the application itself.
#[derive(Clone)]
pub struct WebBundle(Arc<HashMap<String, Bytes>>);

impl std::fmt::Debug for WebBundle {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("WebBundle")
            .field("files", &self.files())
            .field("bytes", &self.bytes())
            .finish()
    }
}

impl WebBundle {
    /// Validate a compiled-in table with the same path rules and bounds as a
    /// directory snapshot. The bytes are borrowed, never copied.
    pub fn from_static(files: &'static [(&'static str, &'static [u8])]) -> Result<Self, Error> {
        if files.len() > MAX_ASSET_FILES {
            return Err(Error::TooLarge);
        }
        let mut total = 0_u64;
        let mut map = HashMap::with_capacity(files.len());
        for (path, bytes) in files {
            RelativePath::parse(path, false)?;
            total = total.saturating_add(bytes.len() as u64);
            if total > MAX_ASSET_BYTES {
                return Err(Error::TooLarge);
            }
            if map
                .insert((*path).to_owned(), Bytes::from_static(bytes))
                .is_some()
            {
                return Err(Error::Conflict);
            }
        }
        if !map.contains_key("index.html") {
            return Err(Error::NotFound);
        }
        Ok(Self(Arc::new(map)))
    }

    pub(crate) fn from_snapshot(files: Vec<WebAsset>) -> Self {
        Self(Arc::new(
            files
                .into_iter()
                .map(|file| (file.path, Bytes::from(file.bytes)))
                .collect(),
        ))
    }

    pub fn files(&self) -> usize {
        self.0.len()
    }

    pub fn bytes(&self) -> u64 {
        self.0.values().map(|bytes| bytes.len() as u64).sum()
    }

    pub(crate) fn serve(&self, request: Request) -> Response {
        if request.method() != Method::GET && request.method() != Method::HEAD {
            let mut response = ApiError::from(Error::Unsupported).into_response();
            *response.status_mut() = StatusCode::METHOD_NOT_ALLOWED;
            return response;
        }
        let Ok(decoded) = percent_encoding::percent_decode_str(request.uri().path()).decode_utf8()
        else {
            return ApiError::from(Error::InvalidPath).into_response();
        };
        let relative = decoded
            .strip_prefix('/')
            .unwrap_or(&decoded)
            .trim_end_matches('/');
        if RelativePath::parse(relative, true).is_err() {
            return ApiError::from(Error::InvalidPath).into_response();
        }
        let candidates = if relative.is_empty() {
            vec!["index.html".to_owned()]
        } else {
            vec![
                relative.to_owned(),
                format!("{relative}/index.html"),
                format!("{relative}.html"),
            ]
        };
        let Some((path, bytes)) = candidates
            .iter()
            .find_map(|path| self.0.get(path).map(|bytes| (path, bytes)))
        else {
            return ApiError::from(Error::NotFound).into_response();
        };
        let body = if request.method() == Method::HEAD {
            Body::empty()
        } else {
            Body::from(bytes.clone())
        };
        let mut response = Response::new(body);
        let headers = response.headers_mut();
        headers.insert(
            http::header::CONTENT_LENGTH,
            bytes.len().to_string().parse().expect("numeric header"),
        );
        headers.insert(
            http::header::CONTENT_TYPE,
            mime_guess::from_path(path)
                .first_or_octet_stream()
                .as_ref()
                .parse()
                .expect("MIME header"),
        );
        headers.insert(
            "x-content-type-options",
            "nosniff".parse().expect("static header"),
        );
        response
    }
}

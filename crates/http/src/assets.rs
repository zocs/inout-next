use crate::routes::ApiError;
use axum::{
    body::Body,
    extract::Request,
    response::{IntoResponse, Response},
};
use bytes::Bytes;
use http::{Method, StatusCode};
use share_domain::{Error, RelativePath};
use share_storage::WebAsset;
use std::{collections::HashMap, sync::Arc};

#[derive(Clone)]
pub(crate) struct Assets(Arc<HashMap<String, Bytes>>);
impl Assets {
    pub fn new(files: Vec<WebAsset>) -> Self {
        Self(Arc::new(
            files
                .into_iter()
                .map(|file| (file.path, Bytes::from(file.bytes)))
                .collect(),
        ))
    }
    pub fn serve(&self, request: Request) -> Response {
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

use crate::AppState;
use axum::{
    body::Body,
    extract::{
        rejection::{JsonRejection, QueryRejection},
        Query, State,
    },
    response::{IntoResponse, Response},
    routing::{get, post, MethodRouter},
    Json, Router,
};
use futures_util::{StreamExt, TryStreamExt};
use http::{HeaderMap, Method, StatusCode};
use serde::Deserialize;
use serde_json::json;
use share_domain::{
    Error, FeatureModule, FilesModule, ManageModule, ModuleManifest, RelativePath, UploadsModule,
};
use tokio::io::AsyncReadExt;
use tokio_util::io::{ReaderStream, StreamReader};

pub(crate) struct ApiError {
    error: Error,
    size: Option<u64>,
}
impl From<Error> for ApiError {
    fn from(error: Error) -> Self {
        Self { error, size: None }
    }
}
impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status = match self.error {
            Error::Unauthorized => StatusCode::UNAUTHORIZED,
            Error::Forbidden => StatusCode::FORBIDDEN,
            Error::InvalidPath | Error::Conflict => StatusCode::BAD_REQUEST,
            Error::NotFound => StatusCode::NOT_FOUND,
            Error::TargetExists | Error::NotEmpty => StatusCode::CONFLICT,
            Error::TooLarge => StatusCode::PAYLOAD_TOO_LARGE,
            Error::Unsupported => StatusCode::NOT_IMPLEMENTED,
            Error::InvalidRange => StatusCode::RANGE_NOT_SATISFIABLE,
            Error::Io => StatusCode::INTERNAL_SERVER_ERROR,
        };
        let mut response = (
            status,
            Json(json!({"error":{"code":self.error.code(),"message":self.error.to_string()}})),
        )
            .into_response();
        if let Some(size) = self.size {
            response.headers_mut().insert(
                http::header::CONTENT_RANGE,
                format!("bytes */{size}").parse().expect("numeric header"),
            );
        }
        response
    }
}

/// HTTP feature modules supply their manifests and route registrations together.
trait HttpModule: FeatureModule {
    fn mount(&self, router: Router<AppState>) -> Router<AppState>;
}
struct FilesApi;
struct UploadsApi;
struct ManageApi;
impl FeatureModule for FilesApi {
    fn manifest(&self) -> ModuleManifest {
        FilesModule.manifest()
    }
}
impl FeatureModule for UploadsApi {
    fn manifest(&self) -> ModuleManifest {
        UploadsModule.manifest()
    }
}
impl FeatureModule for ManageApi {
    fn manifest(&self) -> ModuleManifest {
        ManageModule.manifest()
    }
}
impl HttpModule for FilesApi {
    fn mount(&self, router: Router<AppState>) -> Router<AppState> {
        router
            .route("/entries", get(entries).fallback(method_not_allowed))
            .route(
                "/content",
                get(content).head(content).fallback(method_not_allowed),
            )
    }
}
impl HttpModule for UploadsApi {
    fn mount(&self, router: Router<AppState>) -> Router<AppState> {
        router.route("/content", MethodRouter::new().put(upload))
    }
}
impl HttpModule for ManageApi {
    fn mount(&self, router: Router<AppState>) -> Router<AppState> {
        router
            .route("/directories", post(mkdir).fallback(method_not_allowed))
            .route("/operations", post(operation).fallback(method_not_allowed))
            .route("/entries", MethodRouter::new().delete(remove))
    }
}
pub(crate) fn mount(state: &AppState) -> Router<AppState> {
    let mut router = Router::new()
        .route("/status", get(status).fallback(method_not_allowed))
        .route("/context", get(context).fallback(method_not_allowed))
        .route(
            "/session",
            post(login).delete(logout).fallback(method_not_allowed),
        )
        .fallback(not_found);
    let modules: Vec<Box<dyn HttpModule>> = vec![
        Box::new(FilesApi),
        Box::new(UploadsApi),
        Box::new(ManageApi),
    ];
    for module in modules {
        if state.registry.has(module.manifest().id) {
            router = module.mount(router);
        }
    }
    router
}
pub(crate) async fn not_found() -> Response {
    ApiError::from(Error::NotFound).into_response()
}
async fn method_not_allowed() -> Response {
    let mut response = ApiError::from(Error::Unsupported).into_response();
    *response.status_mut() = StatusCode::METHOD_NOT_ALLOWED;
    response
}
async fn status(State(state): State<AppState>) -> Json<serde_json::Value> {
    Json(json!({"product":state.product,"auth_required":state.auth.required()}))
}
async fn context(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, ApiError> {
    state.auth.authorize(&headers, false)?;
    let permissions = state.registry.permissions(state.writable);
    Ok(Json(
        json!({"product":state.product,"root_label":"Shared files","modules":state.registry.enabled(),
        "capabilities":{"file_move":permissions.manage && state.store.file_move_supported(), "directory_move":permissions.manage && state.store.directory_move_supported(), "directory_copy":false},
        "permissions":permissions,"limits":{"max_upload_bytes":state.max_upload_bytes}}),
    ))
}
#[derive(Deserialize)]
struct Login {
    key: String,
}
async fn login(
    State(state): State<AppState>,
    headers: HeaderMap,
    request: Result<Json<Login>, JsonRejection>,
) -> Result<Response, ApiError> {
    let Json(request) = request.map_err(|_| Error::Conflict)?;
    let cookie = state.auth.login(&headers, &request.key)?;
    Ok((StatusCode::NO_CONTENT, [(http::header::SET_COOKIE, cookie)]).into_response())
}
async fn logout(State(state): State<AppState>, headers: HeaderMap) -> Result<Response, ApiError> {
    let cookie = state.auth.logout(&headers)?;
    Ok((StatusCode::NO_CONTENT, [(http::header::SET_COOKIE, cookie)]).into_response())
}
#[derive(Deserialize)]
struct PathQuery {
    path: String,
}
fn query_path(
    query: Result<Query<PathQuery>, QueryRejection>,
    root: bool,
) -> Result<RelativePath, ApiError> {
    let Query(query) = query.map_err(|_| Error::InvalidPath)?;
    Ok(RelativePath::parse(&query.path, root)?)
}
async fn entries(
    State(state): State<AppState>,
    headers: HeaderMap,
    query: Result<Query<PathQuery>, QueryRejection>,
) -> Result<Json<serde_json::Value>, ApiError> {
    state.auth.authorize(&headers, false)?;
    if !state.registry.permissions(state.writable).list {
        return Err(Error::Forbidden.into());
    }
    let path = query_path(query, true)?;
    let entries = state.store.list(&path).await?;
    Ok(Json(json!({"path":path.as_str(),"entries":entries})))
}
async fn content(
    State(state): State<AppState>,
    headers: HeaderMap,
    method: Method,
    query: Result<Query<PathQuery>, QueryRejection>,
) -> Result<Response, ApiError> {
    state.auth.authorize(&headers, false)?;
    if !state.registry.permissions(state.writable).download {
        return Err(Error::Forbidden.into());
    }
    let path = query_path(query, false)?;
    let download_buffer = state.download_control.buffer_bytes();
    let mut opened = state.store.open(&path, 0).await?;
    let size = opened.entry.size.ok_or(Error::Unsupported)?;
    let range = if method == Method::HEAD {
        None
    } else {
        parse_range(
            headers
                .get(http::header::RANGE)
                .and_then(|value| value.to_str().ok()),
            size,
        )
        .map_err(|_| ApiError {
            error: Error::InvalidRange,
            size: Some(size),
        })?
    };
    let (start, length) = range
        .map(|(start, end)| (start, end - start + 1))
        .unwrap_or((0, size));
    if start != 0 {
        // Position the original handle so metadata and content remain coherent
        // when its pathname is replaced. Storage retains a stream-only fallback.
        opened.reader.advance(start).await?;
    }
    let body = if method == Method::HEAD {
        Body::empty()
    } else {
        let stream = ReaderStream::with_capacity(opened.reader.take(length), download_buffer)
            .take_until(state.cancellation.cancelled_owned());
        Body::from_stream(stream)
    };
    let mut response = Response::new(body);
    *response.status_mut() = if range.is_some() {
        StatusCode::PARTIAL_CONTENT
    } else {
        StatusCode::OK
    };
    let headers = response.headers_mut();
    headers.insert(
        http::header::CONTENT_LENGTH,
        length.to_string().parse().expect("numeric header"),
    );
    headers.insert(
        http::header::CONTENT_TYPE,
        "application/octet-stream".parse().expect("static header"),
    );
    headers.insert(
        http::header::ACCEPT_RANGES,
        "bytes".parse().expect("static header"),
    );
    headers.insert(
        "x-content-type-options",
        "nosniff".parse().expect("static header"),
    );
    // RFC 5987 filename encoding prevents header injection and preserves Unicode.
    let encoded: String = opened
        .entry
        .name
        .as_bytes()
        .iter()
        .map(|byte| format!("%{byte:02X}"))
        .collect();
    headers.insert(
        http::header::CONTENT_DISPOSITION,
        format!("attachment; filename=\"download\"; filename*=UTF-8''{encoded}")
            .parse()
            .map_err(|_| Error::Io)?,
    );
    if let Some((start, end)) = range {
        headers.insert(
            http::header::CONTENT_RANGE,
            format!("bytes {start}-{end}/{size}")
                .parse()
                .expect("numeric header"),
        );
    }
    Ok(response)
}
fn parse_range(header: Option<&str>, size: u64) -> Result<Option<(u64, u64)>, Error> {
    let Some(value) = header else {
        return Ok(None);
    };
    let Some(value) = value.strip_prefix("bytes=") else {
        return Ok(None);
    };
    if value.contains(',') {
        return Ok(None);
    }
    let (start, end) = value.split_once('-').ok_or(Error::InvalidRange)?;
    if size == 0 {
        return Err(Error::InvalidRange);
    }
    if start.is_empty() {
        let suffix = end.parse::<u64>().map_err(|_| Error::InvalidRange)?;
        if suffix == 0 {
            return Err(Error::InvalidRange);
        }
        return Ok(Some((size.saturating_sub(suffix), size - 1)));
    }
    let start = start.parse::<u64>().map_err(|_| Error::InvalidRange)?;
    let end = if end.is_empty() {
        size - 1
    } else {
        end.parse::<u64>()
            .map_err(|_| Error::InvalidRange)?
            .min(size - 1)
    };
    if start >= size || start > end {
        return Err(Error::InvalidRange);
    }
    Ok(Some((start, end)))
}
async fn upload(
    State(state): State<AppState>,
    headers: HeaderMap,
    query: Result<Query<PathQuery>, QueryRejection>,
    body: Body,
) -> Result<Response, ApiError> {
    state.auth.authorize(&headers, true)?;
    if !state.registry.permissions(state.writable).upload {
        return Err(Error::Forbidden.into());
    }
    let path = query_path(query, false)?;
    if headers
        .get(http::header::CONTENT_LENGTH)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse::<u64>().ok())
        .is_some_and(|length| length > state.max_upload_bytes)
    {
        return Err(Error::TooLarge.into());
    }
    let stream = body.into_data_stream().map_err(std::io::Error::other);
    let reader = Box::pin(StreamReader::new(stream));
    let entry = tokio::select! {
        result = state.store.upload(&path, reader, state.max_upload_bytes) => result?,
        _ = state.cancellation.cancelled() => return Err(Error::Conflict.into()),
    };
    Ok((StatusCode::CREATED, Json(json!({"entry":entry}))).into_response())
}
#[derive(Deserialize)]
struct PathBody {
    path: String,
}
fn manage(state: &AppState, headers: &HeaderMap) -> Result<(), ApiError> {
    state.auth.authorize(headers, true)?;
    if !state.registry.permissions(state.writable).manage {
        return Err(Error::Forbidden.into());
    }
    Ok(())
}
async fn mkdir(
    State(state): State<AppState>,
    headers: HeaderMap,
    request: Result<Json<PathBody>, JsonRejection>,
) -> Result<Response, ApiError> {
    manage(&state, &headers)?;
    let Json(request) = request.map_err(|_| Error::Conflict)?;
    let path = RelativePath::parse(&request.path, false)?;
    let entry = state.store.mkdir(&path).await?;
    Ok((StatusCode::CREATED, Json(json!({"entry":entry}))).into_response())
}
#[derive(Deserialize)]
struct Operation {
    action: String,
    source: String,
    destination: String,
}
async fn operation(
    State(state): State<AppState>,
    headers: HeaderMap,
    request: Result<Json<Operation>, JsonRejection>,
) -> Result<Json<serde_json::Value>, ApiError> {
    manage(&state, &headers)?;
    let Json(request) = request.map_err(|_| Error::Conflict)?;
    let source = RelativePath::parse(&request.source, false)?;
    let destination = RelativePath::parse(&request.destination, false)?;
    let entry = match request.action.as_str() {
        "rename" | "move" => {
            if !state.store.file_move_supported() && !state.store.directory_move_supported() {
                return Err(Error::Unsupported.into());
            }
            state.store.move_entry(&source, &destination).await?
        }
        "copy" => state.store.copy(&source, &destination).await?,
        _ => return Err(Error::Unsupported.into()),
    };
    Ok(Json(json!({"entry":entry})))
}
#[derive(Deserialize)]
struct Remove {
    path: String,
    #[serde(default)]
    recursive: bool,
}
async fn remove(
    State(state): State<AppState>,
    headers: HeaderMap,
    query: Result<Query<Remove>, QueryRejection>,
) -> Result<StatusCode, ApiError> {
    manage(&state, &headers)?;
    let Query(query) = query.map_err(|_| Error::InvalidPath)?;
    let path = RelativePath::parse(&query.path, false)?;
    state.store.remove(&path, query.recursive).await?;
    Ok(StatusCode::NO_CONTENT)
}

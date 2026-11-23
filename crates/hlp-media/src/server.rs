use anyhow::{ensure, Result};
use axum::{
    body::Body,
    extract::{Path as RoutePath, State},
    http::{header, Method, Request, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
    Router,
};
use rand::{rngs::OsRng, RngCore};
use std::{
    collections::HashMap,
    net::SocketAddr,
    path::{Path, PathBuf},
    sync::{Arc, RwLock},
};
use tokio::net::TcpListener;
use tokio_util::sync::{CancellationToken, DropGuard};
use tower::ServiceExt;
use tower_http::services::ServeFile;

type Files = Arc<RwLock<HashMap<String, PathBuf>>>;
#[derive(Clone)]
pub struct MediaServer {
    files: Files,
    address: SocketAddr,
    _shutdown: Arc<DropGuard>,
}
impl MediaServer {
    pub async fn start() -> Result<Self> {
        let listener = TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0)).await?;
        let address = listener.local_addr()?;
        let files = Files::default();
        let app = Router::new()
            .route("/m/{token}", get(serve))
            .with_state(files.clone());
        let cancel = CancellationToken::new();
        let shutdown = Arc::new(cancel.clone().drop_guard());
        tokio::spawn(async move {
            if let Err(error) = axum::serve(listener, app)
                .with_graceful_shutdown(cancel.cancelled_owned())
                .await
            {
                tracing::warn!(%error,"Media listener stopped");
            }
        });
        Ok(Self {
            files,
            address,
            _shutdown: shutdown,
        })
    }
    pub fn register(&self, path: &Path) -> Result<String> {
        let path = path.canonicalize()?;
        ensure!(path.is_file(), "Media path must be a regular file");
        let mut files = self.files.write().unwrap_or_else(|e| e.into_inner());
        loop {
            let mut bytes = [0u8; 16];
            OsRng.fill_bytes(&mut bytes);
            let token = format!("{:032x}", u128::from_be_bytes(bytes));
            if !files.contains_key(&token) {
                files.insert(token.clone(), path);
                return Ok(format!("http://{}/m/{token}", self.address));
            }
        }
    }
    pub fn unregister(&self, url: &str) {
        if let Some(token) = url.strip_prefix(&format!("http://{}/m/", self.address)) {
            self.files
                .write()
                .unwrap_or_else(|e| e.into_inner())
                .remove(token);
        }
    }
}

async fn serve(
    State(files): State<Files>,
    RoutePath(token): RoutePath<String>,
    mut req: Request<Body>,
) -> Response {
    let path = files
        .read()
        .unwrap_or_else(|e| e.into_inner())
        .get(&token)
        .cloned();
    let Some(path) = path else {
        return StatusCode::NOT_FOUND.into_response();
    };
    let ext = path
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    let kind = match ext.as_str() {
        "mp4" | "m4v" => "video/mp4",
        "mov" => "video/quicktime",
        "webm" => "video/webm",
        "m4a" => "audio/mp4",
        "mp3" => "audio/mpeg",
        _ => mime_guess::from_path(&path)
            .first_raw()
            .unwrap_or("application/octet-stream"),
    };
    let service = ServeFile::new_with_mime(path, &kind.parse().expect("MIME type"));
    // ServeFile handles Range and conditional dates, but does not apply If-Range.
    // Only a Last-Modified validator can match because ServeFile supplies no ETag.
    if let Some(validator) = req.headers().get(header::IF_RANGE) {
        let head = Request::builder()
            .method(Method::HEAD)
            .body(Body::empty())
            .unwrap();
        let response = service.clone().oneshot(head).await.unwrap();
        if response.headers().get(header::LAST_MODIFIED) != Some(validator) {
            req.headers_mut().remove(header::RANGE);
        }
    }
    service.oneshot(req).await.unwrap().into_response()
}

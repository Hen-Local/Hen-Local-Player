//! User-initiated OpenSubtitles.com searches and downloads. Signed file requests
//! use their own headers so the consumer key only goes to the API origin.
use crate::subtitle_tracks::{decode_subtitle, SubtitleFile, MAX_BYTES};
use reqwest::{
    header::{HeaderValue, ACCEPT},
    redirect::Policy,
    Client, RequestBuilder, Url,
};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::{collections::HashSet, time::Duration};
use tauri_plugin_opener::OpenerExt;

const API: &str = "https://api.opensubtitles.com/api/v1";
const USER_AGENT: &str = concat!("Hen Local Player v", env!("CARGO_PKG_VERSION"));
const JSON_LIMIT: u64 = 2 * 1024 * 1024;
type Reply<T> = Result<T, &'static str>;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    has_api_key: bool,
}

fn key_from_environment() -> Option<String> {
    std::env::var("OPENSUBTITLES_API_KEY")
        .ok()
        .filter(|key| !key.trim().is_empty())
}

#[tauri::command]
pub fn get_opensubtitles_config() -> Config {
    Config {
        has_api_key: key_from_environment().is_some(),
    }
}

#[tauri::command]
pub fn open_opensubtitles_registration(app: tauri::AppHandle) -> Reply<()> {
    app.opener()
        .open_url("https://www.opensubtitles.com/en/consumers", None::<&str>)
        .map_err(|_| "network")
}

fn consumer_key(explicit: Option<String>) -> Reply<HeaderValue> {
    let key = explicit
        .filter(|key| !key.trim().is_empty())
        .or_else(key_from_environment)
        .ok_or("missing_key")?;
    let mut header = HeaderValue::from_str(key.trim()).map_err(|_| "invalid_key")?;
    header.set_sensitive(true);
    Ok(header)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchFile {
    file_id: u32,
    file_name: String,
    language: String,
    release: String,
}
#[derive(Deserialize)]
struct SearchResponse {
    data: Vec<Entry>,
}
#[derive(Deserialize)]
struct Entry {
    attributes: Attributes,
}
#[derive(Deserialize)]
struct Attributes {
    language: String,
    release: Option<String>,
    files: Vec<FileInfo>,
}
#[derive(Deserialize)]
struct FileInfo {
    file_id: u32,
    file_name: Option<String>,
}
#[derive(Deserialize)]
struct Download {
    link: String,
    file_name: Option<String>,
}

fn file_url_allowed(url: &Url) -> bool {
    url.scheme() == "https"
        && url.port().is_none()
        && url.username().is_empty()
        && url.password().is_none()
        && url.host_str().is_some_and(|host| {
            matches!(host, "opensubtitles.com" | "opensubtitles.org")
                || host.ends_with(".opensubtitles.com")
                || host.ends_with(".opensubtitles.org")
        })
}

fn client() -> Reply<Client> {
    Client::builder()
        .user_agent(USER_AGENT)
        .referer(false)
        .connect_timeout(Duration::from_secs(8))
        .timeout(Duration::from_secs(25))
        .redirect(Policy::custom(|attempt| {
            let next = attempt.url();
            let from_api = attempt
                .previous()
                .first()
                .is_some_and(|url| url.host_str() == Some("api.opensubtitles.com"));
            if attempt.previous().len() > 4
                || !file_url_allowed(next)
                || (from_api && next.host_str() != Some("api.opensubtitles.com"))
            {
                return attempt.stop();
            }
            attempt.follow()
        }))
        .build()
        .map_err(|_| "network")
}

async fn response_bytes(request: RequestBuilder, max: u64, api: bool) -> Reply<Vec<u8>> {
    let mut response = request.send().await.map_err(|_| "network")?;
    if !response.status().is_success() {
        return Err(match response.status().as_u16() {
            401 | 403 if api => "invalid_key",
            402 | 406 if api => "quota",
            429 => "rate_limited",
            _ => "unavailable",
        });
    }
    let invalid = if api {
        "unavailable"
    } else {
        "invalid_subtitle"
    };
    if response.content_length().is_some_and(|n| n > max) {
        return Err(invalid);
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(|_| "network")? {
        if bytes.len() as u64 + chunk.len() as u64 > max {
            return Err(invalid);
        }
        bytes.extend_from_slice(&chunk);
    }
    Ok(bytes)
}

async fn api_json<T: DeserializeOwned>(request: RequestBuilder, key: HeaderValue) -> Reply<T> {
    let request = request
        .header(ACCEPT, "application/json")
        .header("Api-Key", key);
    let bytes = response_bytes(request, JSON_LIMIT, true).await?;
    serde_json::from_slice(&bytes).map_err(|_| "unavailable")
}

async fn search(
    client: &Client,
    base: &str,
    key: HeaderValue,
    query: &str,
    language: &str,
) -> Reply<Vec<SearchFile>> {
    let query = query.trim();
    if query.is_empty() || query.chars().count() > 250 || query.chars().any(char::is_control) {
        return Err("invalid_query");
    }
    let mut request = client.get(format!("{base}/subtitles"));
    let language = language.trim().to_ascii_lowercase();
    if !language.is_empty() {
        if language.len() > 12
            || !language
                .split('-')
                .all(|s| (2..=3).contains(&s.len()) && s.bytes().all(|c| c.is_ascii_lowercase()))
        {
            return Err("invalid_language");
        }
        request = request.query(&[("languages", language)]);
    }
    // The provider asks for alphabetized parameters and lowercase queries.
    let result: SearchResponse =
        api_json(request.query(&[("query", query.to_lowercase())]), key).await?;
    let mut files = Vec::new();
    let mut ids = HashSet::new();
    for entry in result.data {
        let a = entry.attributes;
        for f in a.files {
            if f.file_id == 0 || !ids.insert(f.file_id) {
                continue;
            }
            files.push(SearchFile {
                file_id: f.file_id,
                file_name: f.file_name.unwrap_or_default(),
                language: a.language.clone(),
                release: a.release.clone().unwrap_or_default(),
            });
        }
    }
    Ok(files)
}

#[tauri::command]
pub async fn search_opensubtitles(
    query: String,
    language: String,
    api_key: Option<String>,
) -> Reply<Vec<SearchFile>> {
    let key = consumer_key(api_key)?;
    search(&client()?, API, key, &query, &language).await
}

async fn download(
    client: &Client,
    base: &str,
    key: HeaderValue,
    file_id: u32,
    allow_url: impl Fn(&Url) -> bool,
) -> Reply<SubtitleFile> {
    if file_id == 0 {
        return Err("invalid_subtitle");
    }
    let request = client
        .post(format!("{base}/download"))
        .json(&serde_json::json!({"file_id": file_id, "sub_format": "srt"}));
    let meta: Download = api_json(request, key).await?;
    let url = Url::parse(&meta.link).map_err(|_| "unavailable")?;
    if !allow_url(&url) {
        return Err("unavailable");
    }
    let bytes = response_bytes(client.get(url), MAX_BYTES, false).await?;
    // The importer performs complete cue parsing before adding this to a movie.
    let text = decode_subtitle(bytes).map_err(|_| "invalid_subtitle")?;
    if text.contains('\0') || !text.contains("-->") {
        return Err("invalid_subtitle");
    }
    let name = meta
        .file_name
        .as_deref()
        .and_then(|s| s.rsplit(['/', '\\']).next())
        .filter(|s| !s.trim().is_empty() && !s.chars().any(char::is_control))
        .map_or_else(|| format!("OpenSubtitles-{file_id}.srt"), str::to_owned);
    Ok(SubtitleFile { name, text })
}

#[tauri::command]
pub async fn download_opensubtitle(file_id: u32, api_key: Option<String>) -> Reply<SubtitleFile> {
    let key = consumer_key(api_key)?;
    download(&client()?, API, key, file_id, file_url_allowed).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        extract::OriginalUri,
        http::{HeaderMap, StatusCode},
        response::Redirect,
        routing::{get, post},
        Json, Router,
    };
    use serde_json::{json, Value};
    use tokio::net::TcpListener;

    const KEY: &str = "fixture-only";
    const SRT: &str = "1\r\n00:00:00,000 --> 00:00:02,000\r\nHello, 世界\r\n";

    fn key() -> HeaderValue {
        consumer_key(Some(KEY.into())).unwrap()
    }
    fn check_api_headers(headers: &HeaderMap) {
        assert_eq!(headers["Api-Key"], KEY);
        assert_eq!(headers["Accept"], "application/json");
        assert_eq!(headers["User-Agent"], USER_AGENT);
    }

    #[tokio::test]
    async fn search_then_download_uses_selected_file_and_omits_key_on_signed_request() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        let signed_url = format!("{base}/signed/file.srt");
        let routes = Router::new()
            .route("/subtitles", get(|uri: OriginalUri, headers: HeaderMap| async move {
                check_api_headers(&headers);
                assert_eq!(uri.0.query(), Some("languages=zh-cn&query=public+film+%26+part+2"));
                Json(json!({"data":[{"attributes":{
                    "language":"zh-cn", "release":"Public Film",
                    "files":[{"file_id":10,"file_name":"Part one"},{"file_id":11,"file_name":"Part two"}]
                }}]}))
            }))
            .route("/download", post(move |headers: HeaderMap, Json(body): Json<Value>| async move {
                check_api_headers(&headers);
                assert_eq!(body, json!({"file_id":11,"sub_format":"srt"}));
                Json(json!({"file_name":"movie.srt","link":signed_url}))
            }))
            .route("/signed/file.srt", get(|headers: HeaderMap| async move {
                assert!(!headers.contains_key("Api-Key"));
                assert!(!headers.contains_key("Authorization"));
                SRT
            }));
        let server = tokio::spawn(async move { axum::serve(listener, routes).await.unwrap() });
        let client = client().unwrap();
        let rows = search(&client, &base, key(), " Public Film & Part 2 ", "ZH-CN")
            .await
            .unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[1].file_name, "Part two");
        assert_eq!(rows[1].release, "Public Film");
        let file = download(&client, &base, key(), rows[1].file_id, |url| {
            url.as_str() == format!("{base}/signed/file.srt")
        })
        .await
        .unwrap();
        assert_eq!(file.name, "movie.srt");
        assert_eq!(file.text, SRT);
        server.abort();
    }

    #[test]
    fn file_locations_are_https_provider_origins() {
        for address in [
            "https://www.opensubtitles.com/download/ticket",
            "https://dl.opensubtitles.org/en/download/123",
        ] {
            assert!(file_url_allowed(&Url::parse(address).unwrap()));
        }
        for address in [
            "https://notopensubtitles.com/download",
            "https://opensubtitles.com.test/",
            "http://www.opensubtitles.com/x",
            "https://name@www.opensubtitles.com/",
            "https://127.0.0.1:8000/",
            "https://s2.opensubtitles.com:8888/",
        ] {
            assert!(!file_url_allowed(&Url::parse(address).unwrap()));
        }
        assert!(consumer_key(Some("bad\r\nheader".into())).is_err());
    }

    #[tokio::test]
    async fn errors_and_size_limits_are_actionable() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        let routes = Router::new()
            .route("/key", get(|| async { StatusCode::FORBIDDEN }))
            .route("/quota", get(|| async { StatusCode::NOT_ACCEPTABLE }))
            .route("/rate", get(|| async { StatusCode::TOO_MANY_REQUESTS }))
            .route("/large", get(|| async { "0123456789" }))
            .route(
                "/redirect",
                get(|| async { Redirect::temporary("http://127.0.0.1/private") }),
            );
        let server = tokio::spawn(async move { axum::serve(listener, routes).await.unwrap() });
        let client = client().unwrap();
        for (path, limit, api, error) in [
            ("key", 10, true, "invalid_key"),
            ("key", 10, false, "unavailable"),
            ("quota", 10, true, "quota"),
            ("rate", 10, true, "rate_limited"),
            ("large", 5, false, "invalid_subtitle"),
            ("redirect", 10, false, "unavailable"),
        ] {
            assert_eq!(
                response_bytes(client.get(format!("{base}/{path}")), limit, api)
                    .await
                    .unwrap_err(),
                error
            );
        }
        assert!(matches!(
            search(&client, &base, key(), "", "en").await,
            Err("invalid_query")
        ));
        assert!(matches!(
            search(&client, &base, key(), "movie", "../etc").await,
            Err("invalid_language")
        ));
        server.abort();
    }
}

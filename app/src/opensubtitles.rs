//! User-initiated OpenSubtitles.com searches and downloads. Signed file requests
//! use their own headers so the consumer key only goes to the API origin.
use crate::{
    online_subtitles::{response_bytes, subtitle_file, Reply, SearchFile, JSON_LIMIT, USER_AGENT},
    subtitle_tracks::{SubtitleFile, MAX_BYTES},
};
use reqwest::{
    header::{HeaderValue, ACCEPT},
    redirect::Policy,
    Client, RequestBuilder, Url,
};
use serde::{de::DeserializeOwned, Deserialize};
use std::{collections::HashSet, time::Duration};

const API: &str = "https://api.opensubtitles.com/api/v1";
fn consumer_key(key: &str) -> Reply<HeaderValue> {
    let mut header = HeaderValue::from_str(key.trim()).map_err(|_| "invalid_key")?;
    header.set_sensitive(true);
    Ok(header)
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
    let mut request = client.get(format!("{base}/subtitles"));
    if !language.is_empty() {
        let language = match language {
            "zh" => "zh-cn,zh-tw",
            "yue" => "zh-ca",
            "pt" => "pt-br,pt-pt",
            other => other,
        };
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
                file_id: format!("opensubtitles:{}", f.file_id),
                file_name: f.file_name.unwrap_or_default(),
                language: a.language.clone(),
                release: a.release.clone().unwrap_or_default(),
            });
        }
    }
    Ok(files)
}

pub(super) async fn search_titles(
    query: &str,
    language: &str,
    key: &str,
) -> Reply<Vec<SearchFile>> {
    search(&client()?, API, consumer_key(key)?, query, language).await
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
    let name = meta
        .file_name
        .as_deref()
        .and_then(|s| s.rsplit(['/', '\\']).next())
        .filter(|s| !s.trim().is_empty() && !s.chars().any(char::is_control))
        .map_or_else(|| format!("OpenSubtitles-{file_id}.srt"), str::to_owned);
    subtitle_file(name, bytes)
}

pub(super) async fn download_file(file_id: u32, key: &str) -> Reply<SubtitleFile> {
    download(
        &client()?,
        API,
        consumer_key(key)?,
        file_id,
        file_url_allowed,
    )
    .await
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
        consumer_key(KEY).unwrap()
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
        let rows = search(&client, &base, key(), "Public Film & Part 2", "zh-cn")
            .await
            .unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[1].file_name, "Part two");
        assert_eq!(rows[1].file_id, "opensubtitles:11");
        assert_eq!(rows[1].release, "Public Film");
        let file = download(&client, &base, key(), 11, |url| {
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
        assert!(consumer_key("bad\r\nheader").is_err());
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
        server.abort();
    }
}

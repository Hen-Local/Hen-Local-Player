//! Guest title search at subtitles.website. Each selected track goes through
//! the provider's download endpoint so it can enforce its daily allowance.
use super::online_subtitles::{
    response_bytes, subtitle_file, Reply, SearchFile, JSON_LIMIT, USER_AGENT,
};
use super::subtitle_tracks::{SubtitleFile, MAX_BYTES};
use reqwest::{header::ACCEPT, redirect::Policy, Client, RequestBuilder, Url};
use serde::{de::DeserializeOwned, Deserialize};
use std::{collections::HashSet, time::Duration};

const ORIGIN: &str = "https://subtitles.website";

#[derive(Deserialize)]
struct SearchResponse {
    data: Vec<Track>,
}

#[derive(Deserialize)]
struct Track {
    id: String,
    release: Option<String>,
    film_title: Option<String>,
    language: Option<Language>,
    format: Option<String>,
}

#[derive(Deserialize)]
struct Language {
    code: String,
}

#[derive(Deserialize)]
struct Ticket {
    download_url: String,
}

fn client() -> Reply<Client> {
    Client::builder()
        .user_agent(USER_AGENT)
        .referer(false)
        .redirect(Policy::none())
        .connect_timeout(Duration::from_secs(8))
        .timeout(Duration::from_secs(25))
        .build()
        .map_err(|_| "network")
}

fn guest_error(error: &'static str) -> &'static str {
    match error {
        "invalid_key" => "unavailable",
        "rate_limited" => "quota",
        other => other,
    }
}

async fn get_json<T: DeserializeOwned>(request: RequestBuilder) -> Reply<T> {
    let bytes = response_bytes(request.header(ACCEPT, "application/json"), JSON_LIMIT, true)
        .await
        .map_err(guest_error)?;
    serde_json::from_slice(&bytes).map_err(|_| "unavailable")
}

fn valid_id(id: &str) -> bool {
    !id.is_empty() && id.len() <= 64 && id.bytes().all(|b| b.is_ascii_alphanumeric())
}

fn provider_language(language: &str) -> &str {
    match language {
        "zh-cn" | "zh-tw" => "zh",
        "zh-ca" => "yue",
        "pt-pt" => "pt",
        code => code,
    }
}

pub(super) async fn search(query: &str, language: &str) -> Reply<Vec<SearchFile>> {
    search_at(&client()?, ORIGIN, query, language).await
}

async fn search_at(
    client: &Client,
    origin: &str,
    query: &str,
    language: &str,
) -> Reply<Vec<SearchFile>> {
    let imdb = query
        .strip_prefix("tt")
        .or_else(|| query.strip_prefix("TT"))
        .is_some_and(|s| (7..=12).contains(&s.len()) && s.bytes().all(|b| b.is_ascii_digit()));
    let mut request = client
        .get(format!("{origin}/strapi/api/search/subtitles"))
        .query(&[("format", "srt")]);
    request = if imdb {
        request.query(&[("imdb", &query.to_ascii_lowercase())])
    } else {
        request.query(&[("title", query)])
    };
    if !language.is_empty() {
        request = request.query(&[("language", provider_language(language))]);
    }

    let response: SearchResponse = get_json(request).await?;
    let mut ids = HashSet::new();
    let mut results = Vec::new();
    for track in response.data {
        if !valid_id(&track.id)
            || !track
                .format
                .as_deref()
                .is_some_and(|s| s.eq_ignore_ascii_case("srt"))
            || !ids.insert(track.id.clone())
        {
            continue;
        }
        let language = track.language.map_or_else(String::new, |l| l.code);
        let title = track.film_title.unwrap_or_default();
        let release = track.release.unwrap_or_default();
        let name = if !release.trim().is_empty() {
            &release
        } else if !title.trim().is_empty() {
            &title
        } else {
            query
        };
        let mut file_name: String = name
            .trim()
            .chars()
            .filter(|c| !c.is_control())
            .map(|c| if matches!(c, '/' | '\\') { '_' } else { c })
            .collect();
        if !file_name.to_ascii_lowercase().ends_with(".srt") {
            file_name.push_str(".srt");
        }
        results.push(SearchFile {
            file_id: format!("guest:{}", track.id),
            file_name,
            release: title,
            language,
        });
    }
    Ok(results)
}

fn upload_url(origin: &str, download_path: &str) -> Reply<Url> {
    let file = download_path
        .strip_prefix("/uploads/")
        .ok_or("unavailable")?;
    let lower = file.to_ascii_lowercase();
    if !lower.ends_with(".srt")
        || file.contains(['/', '\\', '?', '#'])
        || file.chars().any(char::is_control)
        || ["%2f", "%5c", "%00", "%25"]
            .iter()
            .any(|encoded| lower.contains(encoded))
    {
        return Err("unavailable");
    }
    Url::parse(&format!("{origin}/strapi/uploads/{file}")).map_err(|_| "unavailable")
}

pub(super) async fn download(id: &str) -> Reply<SubtitleFile> {
    download_at(&client()?, ORIGIN, id).await
}

async fn download_at(client: &Client, origin: &str, id: &str) -> Reply<SubtitleFile> {
    if !valid_id(id) {
        return Err("invalid_subtitle");
    }
    let request = client.get(format!("{origin}/strapi/api/search/download/{id}"));
    let ticket: Ticket = get_json(request).await?;
    let url = upload_url(origin, &ticket.download_url)?;
    let bytes = response_bytes(client.get(url), MAX_BYTES, false)
        .await
        .map_err(guest_error)?;
    // The track selected in search supplies its release name to the player.
    subtitle_file(format!("Subtitle-{id}.srt"), bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        extract::Query,
        http::{HeaderMap, StatusCode},
        routing::get,
        Json, Router,
    };
    use serde_json::json;
    use std::collections::HashMap;
    use tokio::net::TcpListener;

    #[tokio::test]
    async fn guest_search_then_selected_download() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let origin = format!("http://{}", listener.local_addr().unwrap());
        let routes = Router::new()
            .route("/strapi/api/search/subtitles", get(|Query(params): Query<HashMap<String, String>>, headers: HeaderMap| async move {
                assert!(!headers.contains_key("Authorization"));
                assert!(!headers.contains_key("Api-Key"));
                assert_eq!(headers["User-Agent"], USER_AGENT);
                assert_eq!(params.get("format").map(String::as_str), Some("srt"));
                assert_eq!(params.get("language").map(String::as_str), Some("zh"));
                if let Some(title) = params.get("title") {
                    assert_eq!(title, "彩色 & Film");
                } else {
                    assert_eq!(params.get("imdb").map(String::as_str), Some("tt0017136"));
                }
                Json(json!({"data":[
                    {"id":"id1","release":"Release 1","film_title":"Film","format":"srt","language":{"code":"zh"}},
                    {"id":"id2","release":"Release 2","film_title":"Film","format":"srt","language":{"code":"zh"},"download_url":"/not-the-ticket"},
                    {"id":"id2","release":"Duplicate","format":"srt"},
                    {"id":"id3","release":"Unsupported","format":"ass"},
                    {"id":"some/path","release":"Invalid id","format":"srt"}
                ]}))
            }))
            .route("/strapi/api/search/download/id2", get(|| async {Json(json!({"download_url":"/uploads/demo.srt","remaining":9}))}))
            .route("/strapi/uploads/demo.srt", get(|| async {"1\n00:00:00,000 --> 00:00:03,500\n你好\n"}))
            .route("/strapi/api/search/download/exhausted", get(|| async {StatusCode::TOO_MANY_REQUESTS}));
        let server = tokio::spawn(async move { axum::serve(listener, routes).await.unwrap() });
        let client = client().unwrap();
        for (query, language) in [("彩色 & Film", "zh-cn"), ("TT0017136", "zh-tw")] {
            let hits = search_at(&client, &origin, query, language).await.unwrap();
            assert_eq!(hits.len(), 2);
            assert_eq!(hits[1].file_id, "guest:id2");
            assert_eq!(hits[1].file_name, "Release 2.srt");
            assert_eq!(hits[1].release, "Film");
            assert_eq!(hits[1].language, "zh");
        }
        assert!(download_at(&client, &origin, "id2")
            .await
            .unwrap()
            .text
            .contains("你好"));
        assert!(matches!(
            download_at(&client, &origin, "exhausted").await,
            Err("quota")
        ));
        server.abort();
    }

    #[test]
    fn ticket_only_accepts_files_in_provider_uploads() {
        for invalid in [
            "https://elsewhere.test/a.srt",
            "//elsewhere.test/a.srt",
            "/uploads/../private.srt",
            "/uploads/%2e%2e%2Fprivate.srt",
            "/uploads/a.srt?go=/.srt",
            "/uploads/bad%255cname.srt",
            "/uploads/not-a-subtitle.zip",
        ] {
            assert!(upload_url(ORIGIN, invalid).is_err(), "{invalid}");
        }
        assert_eq!(
            upload_url(ORIGIN, "/uploads/My%20movie.srt")
                .unwrap()
                .as_str(),
            "https://subtitles.website/strapi/uploads/My%20movie.srt"
        );
    }

    #[tokio::test]
    #[ignore = "contacts the guest subtitle provider and uses one download"]
    async fn live_metropolis_guest_download() {
        let hits = search("Metropolis", "en").await.unwrap();
        let track = hits
            .iter()
            .find(|hit| hit.file_name.contains("1927"))
            .expect("English 1927 Metropolis subtitle");
        let srt = download(track.file_id.strip_prefix("guest:").unwrap())
            .await
            .unwrap();
        assert!(srt.text.matches("-->").count() > 10);
    }
}

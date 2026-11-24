//! On-demand subtitle search. Persist provider-prefixed IDs for cached tracks.
use crate::{
    opensubtitles, subtitle_search_engine,
    subtitle_tracks::{decode_subtitle, SubtitleFile},
};
use reqwest::RequestBuilder;
use serde::Serialize;

pub(super) const USER_AGENT: &str = concat!("Hen Local Player v", env!("CARGO_PKG_VERSION"));
pub(super) const JSON_LIMIT: u64 = 2 * 1024 * 1024;
pub(super) type Reply<T> = Result<T, &'static str>;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchFile {
    pub(super) file_id: String,
    pub(super) file_name: String,
    pub(super) language: String,
    pub(super) release: String,
}

fn configured_key() -> Option<String> {
    std::env::var("OPENSUBTITLES_API_KEY")
        .ok()
        .filter(|key| !key.trim().is_empty())
}

fn validate_query(query: &str, language: &str) -> Reply<()> {
    if query.is_empty() || query.chars().count() > 250 || query.chars().any(char::is_control) {
        return Err("invalid_query");
    }
    if !language.is_empty()
        && (language.len() > 12
            || !language
                .split('-')
                .all(|s| (2..=3).contains(&s.len()) && s.bytes().all(|c| c.is_ascii_lowercase())))
    {
        return Err("invalid_language");
    }
    Ok(())
}

#[tauri::command]
pub async fn search_online_subtitles(query: String, language: String) -> Reply<Vec<SearchFile>> {
    let query = query.trim();
    let language = language.trim().to_ascii_lowercase();
    validate_query(query, &language)?;
    match configured_key() {
        Some(key) => opensubtitles::search_titles(query, &language, &key).await,
        None => subtitle_search_engine::search(query, &language).await,
    }
}

#[tauri::command]
pub async fn download_online_subtitle(file_id: String) -> Reply<SubtitleFile> {
    match file_id.split_once(':') {
        Some(("guest", id)) => subtitle_search_engine::download(id).await,
        Some(("opensubtitles", id)) => {
            let id = id.parse().map_err(|_| "invalid_subtitle")?;
            let key = configured_key().ok_or("missing_key")?;
            opensubtitles::download_file(id, &key).await
        }
        _ => Err("invalid_subtitle"),
    }
}

pub(super) fn subtitle_file(name: String, bytes: Vec<u8>) -> Reply<SubtitleFile> {
    let text = decode_subtitle(bytes).map_err(|_| "invalid_subtitle")?;
    // The UI parses the cues before saving or selecting the track.
    if text.contains('\0') || !text.contains("-->") {
        return Err("invalid_subtitle");
    }
    Ok(SubtitleFile { name, text })
}

pub(super) async fn response_bytes(request: RequestBuilder, max: u64, api: bool) -> Reply<Vec<u8>> {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_user_query_before_contacting_a_provider() {
        assert!(validate_query("大都会", "zh-tw").is_ok());
        assert!(validate_query("Metropolis", "").is_ok());
        assert_eq!(validate_query("", "en"), Err("invalid_query"));
        assert_eq!(validate_query("movie", "../en"), Err("invalid_language"));
    }
}

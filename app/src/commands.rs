use crate::{events::emit, state::Desktop};
use hlp_core::{settings::Settings, types::*};
use hlp_media::{Downloader, Tools};
use hlp_pipeline::store;
use serde_json::{json, Value};
use std::path::PathBuf;
use tauri::{AppHandle, State};

type Reply<T> = Result<T, String>;
fn err(e: impl std::fmt::Display) -> String {
    format!("{e:#}")
}

#[tauri::command]
pub async fn set_playback_capabilities(
    app: AppHandle,
    state: State<'_, Desktop>,
    caps: Caps,
) -> Reply<()> {
    let mut p = state.player.lock().await;
    p.caps = caps;
    p.ready = true;
    if !p.pending.is_empty() {
        emit(&app, "app://open-files", std::mem::take(&mut p.pending));
    }
    #[cfg(target_os = "macos")]
    if let Some(action) = p.pending_menu.take() {
        emit(&app, "app://menu", action);
    }
    Ok(())
}
#[tauri::command]
pub async fn open_media(app: AppHandle, state: State<'_, Desktop>, path: PathBuf) -> Reply<()> {
    state
        .player
        .lock()
        .await
        .open(&app, path, None)
        .await
        .map_err(err)
}
#[tauri::command]
pub async fn close_media(state: State<'_, Desktop>) -> Reply<()> {
    let mut p = state.player.lock().await;
    if let Some(file) = p.active.take() {
        file.session.shutdown().await.map_err(err)?;
    }
    Ok(())
}

#[tauri::command]
pub async fn set_playhead(
    state: State<'_, Desktop>,
    session_id: String,
    ms: u64,
    playing: bool,
) -> Reply<()> {
    let p = state.player.lock().await;
    if let Some(file) = p.active.as_ref().filter(|f| f.session.id == session_id) {
        file.session
            .send(SessionCommand::Playhead { ms, playing })
            .await
            .map_err(err)?;
    }
    Ok(())
}
#[tauri::command]
pub async fn report_playback_error(
    state: State<'_, Desktop>,
    session_id: String,
    code: u16,
) -> Reply<()> {
    let p = state.player.lock().await;
    if let Some(file) = p.active.as_ref().filter(|f| f.session.id == session_id) {
        file.session
            .send(SessionCommand::PlaybackError(code))
            .await
            .map_err(err)?;
    }
    Ok(())
}
#[tauri::command]
pub async fn select_audio_track(state: State<'_, Desktop>, order: usize) -> Reply<()> {
    let p = state.player.lock().await;
    p.file()
        .map_err(err)?
        .session
        .send(SessionCommand::SelectAudioTrack(order))
        .await
        .map_err(err)
}
#[derive(serde::Serialize)]
pub struct TranscriptionStart {
    missing: Vec<String>,
}

#[tauri::command]
pub async fn start_transcription(
    state: State<'_, Desktop>,
    session_id: String,
    regenerate: Option<bool>,
    audio_track_order: Option<usize>,
) -> Reply<TranscriptionStart> {
    let (model, target, source) = {
        let p = state.player.lock().await;
        let f = p.file().map_err(err)?;
        if f.session.id != session_id {
            return Err("The open movie has changed.".into());
        }
        (
            p.settings.asr_model.clone(),
            p.settings.target_language.clone(),
            p.settings.source_language.clone(),
        )
    };
    let translate = target != "none" && source != target;
    let selected_model = model.clone();
    let missing = tokio::task::spawn_blocking(move || {
        hlp_models::missing_for_transcription(&selected_model, translate)
    })
    .await
    .map_err(err)?;
    let p = state.player.lock().await;
    let f = p.file().map_err(err)?;
    if f.session.id != session_id
        || p.settings.asr_model != model
        || p.settings.target_language != target
        || p.settings.source_language != source
    {
        return Err("The movie or its subtitle settings changed. Try again.".into());
    }
    f.session
        .send(SessionCommand::StartTranscription {
            components_ready: missing.is_empty(),
            regenerate: regenerate.unwrap_or(false),
            audio_track_order,
        })
        .await
        .map_err(err)?;
    Ok(TranscriptionStart { missing })
}

#[tauri::command]
pub async fn delete_generated_subtitles(
    state: State<'_, Desktop>,
    session_id: String,
    audio_track_order: usize,
) -> Reply<()> {
    let player = state.player.lock().await;
    let file = player.file().map_err(err)?;
    if file.session.id != session_id {
        return Err("The open movie has changed.".into());
    }
    file.session
        .send(SessionCommand::DeleteGeneratedSubtitles { audio_track_order })
        .await
        .map_err(err)
}

#[tauri::command]
pub async fn retry_failed(state: State<'_, Desktop>) -> Reply<()> {
    let p = state.player.lock().await;
    p.file()
        .map_err(err)?
        .session
        .send(SessionCommand::RetryFailed)
        .await
        .map_err(err)
}
#[tauri::command]
pub async fn get_settings(state: State<'_, Desktop>) -> Reply<Settings> {
    Ok(state.player.lock().await.settings.clone())
}
#[tauri::command]
pub async fn update_settings(state: State<'_, Desktop>, patch: Value) -> Reply<Settings> {
    let mut p = state.player.lock().await;
    let mut current = serde_json::to_value(&p.settings).map_err(err)?;
    for (k, v) in patch.as_object().ok_or("Expected a settings object")? {
        *current
            .get_mut(k)
            .ok_or_else(|| format!("Unknown setting {k}"))? = v.clone();
    }
    p.apply(serde_json::from_value(current).map_err(err)?)
        .await
        .map_err(err)?;
    Ok(p.settings.clone())
}
#[tauri::command]
pub async fn set_target_language(state: State<'_, Desktop>, code: String) -> Reply<()> {
    update_settings(state, json!({"targetLanguage":code}))
        .await
        .map(|_| ())
}
#[tauri::command]
pub async fn set_source_language(state: State<'_, Desktop>, mode_or_code: String) -> Reply<()> {
    update_settings(state, json!({"sourceLanguage":mode_or_code}))
        .await
        .map(|_| ())
}
#[tauri::command]
pub async fn export_subtitles(
    state: State<'_, Desktop>,
    variant: SubtitleVariant,
    format: SubtitleFormat,
    path: PathBuf,
) -> Reply<()> {
    let p = state.player.lock().await;
    p.file()
        .map_err(err)?
        .session
        .send(SessionCommand::Export {
            variant,
            format,
            path,
        })
        .await
        .map_err(err)
}
#[tauri::command]
pub async fn suggest_export_paths(
    state: State<'_, Desktop>,
    variant: SubtitleVariant,
    format: SubtitleFormat,
) -> Reply<PathBuf> {
    let p = state.player.lock().await;
    let f = p.file().map_err(err)?;
    let src = f.events.language.read();
    let src = src.as_deref().unwrap_or("und");
    let target = &p.settings.target_language;
    let lang = match variant {
        SubtitleVariant::Original => src.to_string(),
        SubtitleVariant::Translation => target.clone(),
        SubtitleVariant::Bilingual(_) => format!("{src}-{target}"),
    };
    let ext = match format {
        SubtitleFormat::Srt => "srt",
        SubtitleFormat::Vtt => "vtt",
        SubtitleFormat::Ass => "ass",
    };
    Ok(f.path.with_extension(format!("{lang}.{ext}")))
}
#[tauri::command]
pub async fn list_recent() -> Reply<Vec<RecentItem>> {
    Ok(store::list_recent(&store::data_root()))
}
#[tauri::command]
pub async fn get_setup_status() -> Reply<Vec<ComponentStatus>> {
    tokio::task::spawn_blocking(hlp_models::status)
        .await
        .map_err(err)
}
#[tauri::command]
pub async fn start_setup(
    app: AppHandle,
    state: State<'_, Desktop>,
    include_url_tools: bool,
    ids: Option<Vec<String>>,
) -> Reply<()> {
    let job = state.setup.begin().map_err(err)?;
    let ids = ids.unwrap_or_else(|| hlp_models::required_ids(include_url_tools));
    let source = state.player.lock().await.settings.model_download_source;
    hlp_models::download_from(
        &ids,
        source,
        |e| emit(&app, "setup://progress", e),
        job.cancel.clone(),
    )
    .await
    .map_err(err)?;
    let p = state.player.lock().await;
    if let Some(file) = &p.active {
        file.session
            .send(SessionCommand::RetryFailed)
            .await
            .map_err(err)?;
    }
    Ok(())
}
#[tauri::command]
pub async fn cancel_setup(state: State<'_, Desktop>) -> Reply<()> {
    state.setup.cancel();
    Ok(())
}
#[tauri::command]
pub async fn clear_cache(state: State<'_, Desktop>) -> Reply<()> {
    let active = state
        .player
        .lock()
        .await
        .active
        .as_ref()
        .map(|f| f.session.fingerprint.clone())
        .unwrap_or_default();
    tokio::task::spawn_blocking(move || store::evict(&store::data_root(), &active, 0))
        .await
        .map_err(err)?
        .map_err(err)
}
#[tauri::command]
pub async fn app_info() -> Reply<Value> {
    let root = store::data_root();
    let log = root.join("logs");
    let bytes = tokio::task::spawn_blocking(move || store::cache_size(&root.join("media")))
        .await
        .map_err(err)?;
    Ok(json!({"version":env!("CARGO_PKG_VERSION"),"logDir":log,"cacheBytes":bytes}))
}
#[tauri::command]
pub async fn reveal_in_folder(path: PathBuf) -> Reply<()> {
    tauri_plugin_opener::reveal_item_in_dir(path).map_err(err)
}
#[tauri::command]
pub async fn probe_url(app: AppHandle, state: State<'_, Desktop>, url: String) -> Reply<UrlInfo> {
    let parsed = tauri::Url::parse(&url).map_err(err)?;
    if !matches!(parsed.scheme(), "http" | "https") || parsed.host().is_none() {
        return Err("Enter an HTTP or HTTPS video URL".into());
    }
    let job = state.url.begin().map_err(err)?;
    state.preview.lock().take();
    let tools = Tools::resolve();
    let missing: Vec<String> = [("yt-dlp", tools.yt_dlp), ("deno", tools.deno)]
        .into_iter()
        .filter(|(_, p)| !p.is_file())
        .map(|(id, _)| id.to_string())
        .collect();
    if !missing.is_empty() {
        hlp_models::download(
            &missing,
            |e| emit(&app, "setup://progress", e),
            job.cancel.clone(),
        )
        .await
        .map_err(err)?;
    }
    let info = Downloader::new(Tools::resolve())
        .probe(&url, job.cancel.clone())
        .await
        .map_err(err)?;
    *state.preview.lock() = Some((url, info.clone()));
    Ok(info)
}
#[tauri::command]
pub async fn download_url(app: AppHandle, state: State<'_, Desktop>, url: String) -> Reply<()> {
    let job = state.url.begin().map_err(err)?;
    let info = state
        .preview
        .lock()
        .as_ref()
        .filter(|(u, _)| u == &url)
        .map(|(_, info)| info.clone())
        .ok_or("Preview this link before downloading")?;
    let dir = state.player.lock().await.settings.download_dir.clone();
    let path = Downloader::new(Tools::resolve())
        .download(
            &url,
            &info,
            &dir,
            |e| emit(&app, "download://progress", e),
            job.cancel.clone(),
        )
        .await
        .map_err(err)?;
    let mut player = state.player.lock().await;
    if job.cancel.is_cancelled() {
        return Err("Download cancelled".into());
    }
    player.open(&app, path, Some(url)).await.map_err(err)
}
#[tauri::command]
pub async fn cancel_download(state: State<'_, Desktop>) -> Reply<()> {
    state.url.cancel();
    Ok(())
}
#[tauri::command]
pub async fn update_ytdlp(app: AppHandle, state: State<'_, Desktop>) -> Reply<()> {
    let job = state.setup.begin().map_err(err)?;
    hlp_models::update_ytdlp(|e| emit(&app, "setup://progress", e), job.cancel.clone())
        .await
        .map_err(err)
}

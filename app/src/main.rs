#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]
mod app_updates;
mod commands;
mod events;
mod online_subtitles;
mod opensubtitles;
mod playback;
mod player_menu;
mod state;
mod subtitle_search_engine;
mod subtitle_tracks;
use commands::*;
use state::Desktop;
use tauri::{Manager, RunEvent};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

fn main() {
    hlp_models::use_bundled_playback_tools();
    let log_dir = hlp_pipeline::store::data_root().join("logs");
    std::fs::create_dir_all(&log_dir).expect("create player log directory");
    let file = tracing_appender::rolling::Builder::new()
        .rotation(tracing_appender::rolling::Rotation::DAILY)
        .filename_prefix("henlocal-player.log")
        .max_log_files(7)
        .build(log_dir)
        .expect("create player log");
    let (writer, _writer_guard) = tracing_appender::non_blocking(file);
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .with(tracing_subscriber::fmt::layer().with_writer(std::io::stderr))
        .with(
            tracing_subscriber::fmt::layer()
                .with_writer(writer)
                .with_ansi(false),
        )
        .init();
    let old_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |e| {
        tracing::error!(panic=%e,"Player panic");
        old_hook(e)
    }));
    // Finder can deliver the initial file before Tauri runs its setup hook.
    let desktop = tauri::async_runtime::block_on(Desktop::new()).expect("initialize player");
    #[cfg(target_os = "macos")]
    let mut launched = false;
    tauri::Builder::default()
        .manage(desktop)
        .plugin(
            tauri_plugin_updater::Builder::new()
                .pubkey(include_str!("../updater.pub").trim())
                .build(),
        )
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .menu(player_menu::build)
        .on_menu_event(|app, event| player_menu::handle(app, event.id().as_ref()))
        .invoke_handler(tauri::generate_handler![
            app_updates::list_app_releases,
            app_updates::download_app_release,
            app_updates::prepared_app_release,
            app_updates::install_app_release,
            set_playback_capabilities,
            open_media,
            close_media,
            playback::choose_media_folder,
            playback::list_media_siblings,
            playback::get_sample_media,
            playback::list_chapters,
            subtitle_tracks::list_subtitle_tracks,
            subtitle_tracks::read_sidecar_subtitle,
            subtitle_tracks::read_embedded_subtitle,
            subtitle_tracks::choose_subtitle_file,
            online_subtitles::search_online_subtitles,
            online_subtitles::download_online_subtitle,
            probe_url,
            download_url,
            cancel_download,
            set_playhead,
            set_target_language,
            set_source_language,
            select_audio_track,
            report_playback_error,
            retry_failed,
            start_transcription,
            delete_generated_subtitles,
            export_subtitles,
            suggest_export_paths,
            get_settings,
            update_settings,
            get_setup_status,
            start_setup,
            cancel_setup,
            list_recent,
            clear_cache,
            reveal_in_folder,
            app_info,
            update_ytdlp
        ])
        .build(tauri::generate_context!())
        .expect("build Hen Local Player")
        .run(move |app, event| match event {
            RunEvent::Ready => {
                #[cfg(target_os = "macos")]
                {
                    launched = true;
                }
                show_window(app);
            }
            #[cfg(target_os = "macos")]
            RunEvent::WindowEvent {
                label,
                event: tauri::WindowEvent::Destroyed,
                ..
            } if label == "main" => app.state::<Desktop>().window_closed(),
            #[cfg(target_os = "macos")]
            RunEvent::ExitRequested {
                code: None, api, ..
            } => {
                // The last window closed. AppKit's Quit command is independent.
                api.prevent_exit();
            }
            #[cfg(target_os = "macos")]
            RunEvent::Reopen {
                has_visible_windows: false,
                ..
            } if launched => show_window(app),
            #[cfg(target_os = "macos")]
            RunEvent::Opened { urls } => {
                let paths: Vec<_> = urls
                    .into_iter()
                    .filter_map(|url| url.to_file_path().ok())
                    .collect();
                if !paths.is_empty() {
                    if launched {
                        show_window(app);
                    }
                    let app = app.clone();
                    tauri::async_runtime::spawn(async move {
                        let state = app.state::<Desktop>();
                        let mut player = state.player.lock().await;
                        if player.ready {
                            events::emit(&app, "app://open-files", paths);
                        } else {
                            for path in paths {
                                if !player.pending.contains(&path) {
                                    player.pending.push(path);
                                }
                            }
                        }
                    });
                }
            }
            RunEvent::Exit => {
                let state = app.state::<Desktop>();
                tauri::async_runtime::block_on(state.shutdown());
            }
            _ => (),
        });
}

fn show_window(app: &tauri::AppHandle) {
    let window = app.get_webview_window("main");
    #[cfg(target_os = "macos")]
    let window = window.or_else(|| {
        let config = app
            .config()
            .app
            .windows
            .iter()
            .find(|w| w.label == "main")?;
        match tauri::WebviewWindowBuilder::from_config(app, config).and_then(|b| b.build()) {
            Ok(window) => Some(window),
            Err(error) => {
                tracing::error!(%error, "Could not reopen the player window");
                None
            }
        }
    });
    if let Some(window) = window {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
    }
}

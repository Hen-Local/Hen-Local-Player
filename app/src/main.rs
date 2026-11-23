#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]
mod commands;
mod events;
mod prewarm;
mod state;
use commands::*;
use state::Desktop;
use tauri::{
    menu::{Menu, MenuItem, MenuItemKind},
    Manager, RunEvent,
};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

fn main() {
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
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let desktop = tauri::async_runtime::block_on(Desktop::new())?;
            let p = desktop.player.try_lock()?;
            tauri::async_runtime::spawn(prewarm::run(p.settings.clone(), p.engines.clone()));
            drop(p);
            app.manage(desktop);
            Ok(())
        })
        .menu(|app| {
            let menu = Menu::default(app)?;
            for item in menu.items()? {
                if let MenuItemKind::Submenu(file) = item {
                    if file.text()? == "File" {
                        for (id, label, key) in [
                            ("export", "Export Subtitles…", "CmdOrCtrl+E"),
                            ("url", "Open Link…", "CmdOrCtrl+L"),
                            ("open", "Open File…", "CmdOrCtrl+O"),
                        ] {
                            file.prepend(&MenuItem::with_id(app, id, label, true, Some(key))?)?;
                        }
                        break;
                    }
                }
            }
            Ok(menu)
        })
        .on_menu_event(|app, e| match e.id().as_ref() {
            "open" | "url" | "export" => {
                events::emit(app, "app://menu", e.id().as_ref().to_string())
            }
            _ => (),
        })
        .invoke_handler(tauri::generate_handler![
            set_playback_capabilities,
            open_media,
            probe_url,
            download_url,
            cancel_download,
            set_playhead,
            set_target_language,
            set_source_language,
            select_audio_track,
            report_playback_error,
            retry_failed,
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
        .run(|app, event| match event {
            RunEvent::Ready => show_window(app),
            #[cfg(target_os = "macos")]
            RunEvent::Reopen {
                has_visible_windows: false,
                ..
            } => show_window(app),
            #[cfg(target_os = "macos")]
            RunEvent::Opened { urls } => {
                if let Some(path) = urls.into_iter().find_map(|url| url.to_file_path().ok()) {
                    let app = app.clone();
                    tauri::async_runtime::spawn(async move {
                        let state = app.state::<Desktop>();
                        let mut p = state.player.lock().await;
                        if !p.ready {
                            p.pending = Some(path);
                            return;
                        }
                        if let Err(e) = p.open(&app, path, None).await {
                            events::emit(
                                &app,
                                "app://error",
                                hlp_core::events::ErrorEvent {
                                    session_id: None,
                                    code: "OpenMedia".into(),
                                    message: format!("{e:#}"),
                                    recoverable: true,
                                },
                            );
                        }
                    });
                }
            }
            RunEvent::Exit => {
                let state = app.state::<Desktop>();
                state.setup.cancel();
                state.url.cancel();
                tauri::async_runtime::block_on(async {
                    let mut p = state.player.lock().await;
                    if let Some(file) = p.active.take() {
                        let _ = file.session.shutdown().await;
                    }
                    p.engines.shutdown().await;
                });
            }
            _ => (),
        });
}

fn show_window(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
    }
}

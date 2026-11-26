use serde::Deserialize;
use tauri::{
    menu::{
        AboutMetadata, Menu, MenuItem, PredefinedMenuItem as Native, Submenu, HELP_SUBMENU_ID,
        WINDOW_SUBMENU_ID,
    },
    AppHandle,
};

// Only the currently displayed labels cross IPC. Actions and keyboard shortcuts stay native
// so the application menu can reopen a closed window on macOS.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MenuLabels {
    app: String,
    has_media: bool,
    file: String,
    open: String,
    open_folder: String,
    recent: String,
    close_file: String,
    url: String,
    settings: String,
    playback: String,
    play_pause: String,
    previous_file: String,
    next_file: String,
    playlist: String,
    subtitles: String,
    panel: String,
    open_subtitle_file: String,
    find_online_subtitles: String,
    export_subs: String,
    edit: String,
    #[cfg(target_os = "macos")]
    view: String,
    window: String,
    help: String,
    about: String,
    #[cfg(target_os = "macos")]
    services: String,
    #[cfg(target_os = "macos")]
    hide: String,
    #[cfg(target_os = "macos")]
    hide_others: String,
    quit: String,
    close_window: String,
    undo: String,
    redo: String,
    cut: String,
    copy: String,
    paste: String,
    select_all: String,
    #[cfg(target_os = "macos")]
    fullscreen: String,
    minimize: String,
    maximize: String,
}

#[tauri::command]
pub fn set_menu_labels(app: AppHandle, labels: MenuLabels) -> tauri::Result<()> {
    let menu = Menu::new(&app)?;
    let item = |id: &str, label: &str, shortcut: Option<&str>| {
        MenuItem::with_id(&app, id, label, true, shortcut)
    };
    let close_file = MenuItem::with_id(
        &app,
        "close-file",
        &labels.close_file,
        labels.has_media,
        None::<&str>,
    )?;
    let settings = item("settings", &labels.settings, Some("CmdOrCtrl+,"))?;
    let about = Native::about(
        &app,
        Some(&labels.about),
        Some(AboutMetadata {
            name: Some(labels.app.clone()),
            version: Some(app.package_info().version.to_string()),
            copyright: app.config().bundle.copyright.clone(),
            authors: app.config().bundle.publisher.clone().map(|p| vec![p]),
            ..Default::default()
        }),
    )?;
    #[cfg(target_os = "macos")]
    menu.append(&Submenu::with_items(
        &app,
        &labels.app,
        true,
        &[
            &about,
            &settings,
            &Native::separator(&app)?,
            &Native::services(&app, Some(&labels.services))?,
            &Native::separator(&app)?,
            &Native::hide(&app, Some(&labels.hide))?,
            &Native::hide_others(&app, Some(&labels.hide_others))?,
            &Native::separator(&app)?,
            &Native::quit(&app, Some(&labels.quit))?,
        ],
    )?)?;
    let file = Submenu::with_items(
        &app,
        &labels.file,
        true,
        &[
            &item("open", &labels.open, Some("CmdOrCtrl+O"))?,
            &item(
                "open-folder",
                &labels.open_folder,
                Some("CmdOrCtrl+Shift+O"),
            )?,
            &item("url", &labels.url, Some("CmdOrCtrl+L"))?,
            &item("library", &labels.recent, None)?,
            &Native::separator(&app)?,
            &close_file,
            &Native::close_window(&app, Some(&labels.close_window))?,
        ],
    )?;
    #[cfg(not(target_os = "macos"))]
    {
        file.append(&settings)?;
        file.append(&Native::quit(&app, Some(&labels.quit))?)?;
    }
    menu.append(&file)?;
    menu.append(&Submenu::with_items(
        &app,
        &labels.playback,
        true,
        &[
            &item("play-pause", &labels.play_pause, None)?,
            &item("previous-file", &labels.previous_file, None)?,
            &item("next-file", &labels.next_file, None)?,
            &item("playlist", &labels.playlist, None)?,
        ],
    )?)?;
    menu.append(&Submenu::with_items(
        &app,
        &labels.subtitles,
        true,
        &[
            &item("transcript", &labels.panel, None)?,
            &item("subtitle-file", &labels.open_subtitle_file, None)?,
            &item(
                "find-subtitles",
                &labels.find_online_subtitles,
                Some("CmdOrCtrl+Shift+D"),
            )?,
            &item("export", &labels.export_subs, Some("CmdOrCtrl+E"))?,
        ],
    )?)?;
    menu.append(&Submenu::with_items(
        &app,
        &labels.edit,
        true,
        &[
            &Native::undo(&app, Some(&labels.undo))?,
            &Native::redo(&app, Some(&labels.redo))?,
            &Native::separator(&app)?,
            &Native::cut(&app, Some(&labels.cut))?,
            &Native::copy(&app, Some(&labels.copy))?,
            &Native::paste(&app, Some(&labels.paste))?,
            &Native::select_all(&app, Some(&labels.select_all))?,
        ],
    )?)?;
    #[cfg(target_os = "macos")]
    menu.append(&Submenu::with_items(
        &app,
        &labels.view,
        true,
        &[&Native::fullscreen(&app, Some(&labels.fullscreen))?],
    )?)?;
    menu.append(&Submenu::with_id_and_items(
        &app,
        WINDOW_SUBMENU_ID,
        &labels.window,
        true,
        &[
            &Native::minimize(&app, Some(&labels.minimize))?,
            &Native::maximize(&app, Some(&labels.maximize))?,
            #[cfg(target_os = "macos")]
            &Native::separator(&app)?,
            &Native::close_window(&app, Some(&labels.close_window))?,
        ],
    )?)?;
    menu.append(&Submenu::with_id_and_items(
        &app,
        HELP_SUBMENU_ID,
        &labels.help,
        true,
        &[
            #[cfg(not(target_os = "macos"))]
            &about,
        ],
    )?)?;
    app.set_menu(menu)?;
    Ok(())
}

pub fn handle(app: &AppHandle, id: &str) {
    #[cfg(target_os = "macos")]
    if matches!(id, "open" | "open-folder" | "url" | "settings" | "library") {
        use tauri::Manager;
        crate::show_window(app);
        let app = app.clone();
        let action = id.to_owned();
        tauri::async_runtime::spawn(async move {
            let state = app.state::<crate::state::Desktop>();
            let mut player = state.player.lock().await;
            if player.ready {
                crate::events::emit(&app, "app://menu", action);
            } else {
                player.pending_menu = Some(action);
            }
        });
        return;
    }
    if matches!(
        id,
        "open"
            | "open-folder"
            | "url"
            | "play-pause"
            | "library"
            | "close-file"
            | "previous-file"
            | "next-file"
            | "playlist"
            | "subtitle-file"
            | "find-subtitles"
            | "transcript"
            | "export"
            | "settings"
    ) {
        crate::events::emit(app, "app://menu", id);
    }
}

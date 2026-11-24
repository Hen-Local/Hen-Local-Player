use tauri::{
    menu::{Menu, MenuItem, MenuItemKind, Submenu},
    AppHandle,
};

fn item(
    app: &AppHandle,
    id: &str,
    label: &str,
    shortcut: Option<&str>,
) -> tauri::Result<MenuItem<tauri::Wry>> {
    MenuItem::with_id(app, id, label, true, shortcut)
}

pub fn build(app: &AppHandle) -> tauri::Result<Menu<tauri::Wry>> {
    let menu = Menu::default(app)?;
    let mut file = None;
    for entry in menu.items()? {
        if let MenuItemKind::Submenu(submenu) = entry {
            if submenu.text()? == "File" {
                file = Some(submenu);
                break;
            }
        }
    }
    let file = match file {
        Some(file) => file,
        None => {
            let file = Submenu::new(app, "File", true)?;
            menu.prepend(&file)?;
            file
        }
    };
    for (index, (id, label, key)) in [
        ("open", "Open File…", Some("CmdOrCtrl+O")),
        ("open-folder", "Open Folder…", Some("CmdOrCtrl+Shift+O")),
        ("url", "Open Link…", Some("CmdOrCtrl+L")),
        ("close-media", "Close File", None),
    ]
    .into_iter()
    .enumerate()
    {
        file.insert(&item(app, id, label, key)?, index)?;
    }
    let settings = item(app, "settings", "Settings…", Some("CmdOrCtrl+,"))?;
    #[cfg(target_os = "macos")]
    if let Some(MenuItemKind::Submenu(application)) = menu.items()?.first() {
        application.insert(&settings, 1)?;
    }
    #[cfg(not(target_os = "macos"))]
    file.append(&settings)?;
    let playback = Submenu::new(app, "Playback", true)?;
    for (id, label) in [
        ("play-pause", "Play / Pause"),
        ("seek-backward", "Back 10 Seconds"),
        ("seek-forward", "Forward 10 Seconds"),
        ("previous-file", "Previous File"),
        ("next-file", "Next File"),
        ("repeat", "Cycle Repeat Mode"),
        ("playlist", "Show Playlist"),
        ("playback-options", "Playback Options…"),
    ] {
        playback.append(&item(app, id, label, None)?)?;
    }
    let audio = Submenu::new(app, "Audio", true)?;
    audio.append(&item(app, "mute", "Mute / Unmute", None)?)?;
    audio.append(&item(app, "volume-up", "Volume Up", None)?)?;
    audio.append(&item(app, "volume-down", "Volume Down", None)?)?;
    audio.append(&item(app, "audio-tracks", "Audio Tracks…", None)?)?;
    let subtitles = Submenu::new(app, "Subtitles", true)?;
    for (id, label, key) in [
        ("subtitle-file", "Open Subtitle File…", None),
        ("subtitles", "Cycle Generated Subtitles", None),
        ("transcript", "Show Subtitles Panel", None),
        ("export", "Export Generated Subtitles…", Some("CmdOrCtrl+E")),
    ] {
        subtitles.append(&item(app, id, label, key)?)?;
    }
    let slot = menu
        .items()?
        .iter()
        .position(|i| i.id() == file.id())
        .unwrap_or(0)
        + 1;
    menu.insert(&playback, slot)?;
    menu.insert(&audio, slot + 1)?;
    menu.insert(&subtitles, slot + 2)?;
    Ok(menu)
}

pub fn handle(app: &AppHandle, id: &str) {
    #[cfg(target_os = "macos")]
    if matches!(id, "open" | "open-folder" | "url" | "settings") {
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
            | "close-media"
            | "play-pause"
            | "seek-backward"
            | "seek-forward"
            | "previous-file"
            | "next-file"
            | "repeat"
            | "playlist"
            | "mute"
            | "volume-up"
            | "volume-down"
            | "audio-tracks"
            | "playback-options"
            | "subtitle-file"
            | "subtitles"
            | "transcript"
            | "export"
            | "settings"
    ) {
        crate::events::emit(app, "app://menu", id);
    }
}

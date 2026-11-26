import { invoke } from '@tauri-apps/api/core'
import type { Messages } from './index'

let lastUpdate = Promise.resolve()

export function applyNativeMenu(s: Messages, fullscreen: boolean, hasMedia: boolean) {
  const withApp = (label: string) => label.replace('{app}', s.app)
  const labels = {
    app: s.app,
    hasMedia,
    file: s.menuFile,
    open: (/Mac/.test(navigator.platform) ? s.open + ' / ' + s.openFolder : s.open) + '…',
    openFolder: s.openFolder + '…',
    recent: s.recent,
    closeFile: s.closeFile,
    url: s.openUrl + '…',
    settings: s.settings + '…',
    playback: s.playMedia,
    playPause: s.play + ' / ' + s.pause,
    previousFile: s.previousFile,
    nextFile: s.nextFile,
    playlist: s.playlist,
    subtitles: s.subtitles,
    panel: s.subtitleList,
    openSubtitleFile: s.openSubtitleFile,
    findOnlineSubtitles: s.findOnlineSubtitles + '…',
    exportSubs: s.exportSubs,
    edit: s.menuEdit,
    view: s.menuView,
    window: s.menuWindow,
    help: s.menuHelp,
    about: withApp(s.menuAbout),
    services: s.menuServices,
    hide: withApp(s.menuHide),
    hideOthers: s.menuHideOthers,
    quit: withApp(s.menuQuit),
    closeWindow: s.menuCloseWindow,
    undo: s.menuUndo,
    redo: s.menuRedo,
    cut: s.menuCut,
    copy: s.menuCopy,
    paste: s.menuPaste,
    selectAll: s.menuSelectAll,
    fullscreen: fullscreen ? s.exitFullscreen : s.fullscreen,
    minimize: s.menuMinimize,
    maximize: s.menuMaximize,
  }
  // A previous locale may still be crossing IPC when another one is selected.
  const update = lastUpdate.then(() => invoke<void>('set_menu_labels', { labels }))
  lastUpdate = update.catch(() => {})
  return update
}

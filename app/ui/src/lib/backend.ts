import mediaExtensions from '../../../media-formats.json'
import type { Caps, MediaOpened, PlaybackPrep, CueBatch, PipelineStatus, LanguageEvent, ComponentStatus, ErrorEvent, DownloadProgress, Settings, UrlInfo, RecentItem, SubtitleVariant, SubtitleFormat } from './types'
import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { getCurrentWebview } from '@tauri-apps/api/webview'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { open, save } from '@tauri-apps/plugin-dialog'
export interface EmbeddedTrack {
  order: number
  default: boolean
  forced: boolean
  language: string | null
  title: string | null
}
export interface SubtitleInventory {
  embedded: EmbeddedTrack[]
  sidecars: string[]
  exactSidecar: string | null
  probeFailed: boolean
}
export interface Commands {
  set_playback_capabilities: [ { caps: Caps }, void ]
  close_media: [ undefined, void ]
  choose_media_folder: [ undefined, string[] ]
  save_video_frame: [ { sessionId: string; ms: number }, boolean ]
  get_sample_media: [ undefined, string | null ]
  list_chapters: [ { sessionId: string }, { startMs: number; title: string }[] ]
  open_media: [ { path: string }, void ]
  probe_url: [ { url: string }, UrlInfo ]
  download_url: [ { url: string }, void ]
  cancel_download: [ undefined, void ]
  set_playhead: [ { sessionId: string; ms: number; playing: boolean }, void ]
  set_target_language: [ { code: string }, void ]
  set_source_language: [ { modeOrCode: string }, void ]
  select_audio_track: [ { order: number }, void ]
  report_playback_error: [ { sessionId: string; code: number }, void ]
  retry_failed: [ undefined, void ]
  start_transcription: [{ sessionId: string; regenerate?: boolean; audioTrackOrder?: number }, { missing: string[] }]
  list_subtitle_tracks: [{ sessionId: string }, SubtitleInventory]
  read_embedded_subtitle: [{ sessionId: string; order: number }, string]
  read_sidecar_subtitle: [{ sessionId: string; name: string }, string]
  export_subtitles: [ { variant: SubtitleVariant; format: SubtitleFormat; path: string }, void ]
  suggest_export_paths: [ { variant: SubtitleVariant; format: SubtitleFormat }, string ]
  get_settings: [ undefined, Settings ]
  update_settings: [ { patch: Partial<Settings> }, Settings ]
  get_setup_status: [ undefined, ComponentStatus[] ]
  start_setup: [ { includeUrlTools: boolean; ids?: string[] }, void ]
  cancel_setup: [ undefined, void ]
  list_recent: [ undefined, RecentItem[] ]
  clear_cache: [ undefined, void ]
  reveal_in_folder: [ { path: string }, void ]
  app_info: [ undefined, {version:string; logDir:string; cacheBytes:number} ]
  update_ytdlp: [ undefined, void ]
}
export interface Events {
  'media://opened': MediaOpened
  'media://prep': PlaybackPrep
  'pipeline://cues': CueBatch
  'pipeline://status': PipelineStatus
  'pipeline://language': LanguageEvent
  'setup://progress': ComponentStatus
  'download://progress': DownloadProgress
  'app://open-files': string[]
  'app://menu': 'open' | 'open-folder' | 'close-media' | 'url' | 'export' | 'play-pause' | 'playlist' | 'previous-file' | 'next-file' | 'seek-backward' | 'seek-forward' | 'mute' | 'audio-tracks' | 'volume-up' | 'volume-down' | 'repeat' | 'fullscreen' | 'subtitles' | 'transcript' | 'subtitle-file' | 'playback-options' | 'settings'
  'app://error': ErrorEvent
}
export interface Backend {
  call<K extends keyof Commands>(command: K, args?: Commands[K][0]): Promise<Commands[K][1]>
  listen<K extends keyof Events>(event: K, receive: (payload: Events[K]) => void): Promise<() => void>
  openFiles(): Promise<string[]>
  openFolder(): Promise<string[]>
  saveFile(defaultPath: string): Promise<string | null>
  fullscreen(enabled: boolean): Promise<void>
  onDrop(receive: (paths: string[]) => void): Promise<() => void>
}
export class TauriBackend implements Backend {
  call<K extends keyof Commands>(command: K, args?: Commands[K][0]): Promise<Commands[K][1]> {
    return invoke(command, args)
  }
  listen<K extends keyof Events>(event: K, receive: (payload: Events[K]) => void): Promise<UnlistenFn> {
    return listen<Events[K]>(event, ({ payload }) => receive(payload))
  }
  async openFiles() {
    return (await open({ multiple: true, directory: false, filters: [{ name: 'Media', extensions: mediaExtensions }, { name: 'All files', extensions: ['*'] }] })) ?? []
  }
  async openFolder() {
    const paths = await this.call('choose_media_folder')
    const order = new Intl.Collator(undefined, { numeric: true, sensitivity: 'base' })
    return paths.sort(order.compare)
  }
  saveFile(defaultPath: string) { return save({ defaultPath }) }
  fullscreen(enabled: boolean) { return getCurrentWindow().setFullscreen(enabled) }
  onDrop(receive: (paths:string[]) => void) {
    return getCurrentWebview().onDragDropEvent(({ payload }) => {
      if (payload.type === 'drop' && payload.paths.length) receive(payload.paths)
    })
  }
}

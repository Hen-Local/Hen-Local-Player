import type { Caps, MediaOpened, PlaybackPrep, CueBatch, PipelineStatus, LanguageEvent, ComponentStatus, ErrorEvent, DownloadProgress, Settings, UrlInfo, RecentItem, SubtitleVariant, SubtitleFormat } from './types'
export interface Commands {
  set_playback_capabilities: [ { caps: Caps }, void ]
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
  'app://error': ErrorEvent
}
export interface Backend {
  call<K extends keyof Commands>(command: K, args?: Commands[K][0]): Promise<Commands[K][1]>
  listen<K extends keyof Events>(event: K, receive: (payload: Events[K]) => void): Promise<() => void>
  openFile(): Promise<string | null>
  saveFile(defaultPath: string): Promise<string | null>
  fullscreen(enabled: boolean): Promise<void>
  onDrop(receive: (path: string) => void): Promise<() => void>
}
export class TauriBackend implements Backend {
  async call<K extends keyof Commands>(command: K, args?: Commands[K][0]): Promise<Commands[K][1]> {throw Error('Desktop commands are under construction')}
  async listen<K extends keyof Events>(event: K, receive: (payload: Events[K]) => void) {return () => {}}
  async openFile() {return null}
  async saveFile(defaultPath: string) {return null}
  async fullscreen(enabled: boolean) {}
  async onDrop(receive: (path:string) => void) {return () => {}}
}

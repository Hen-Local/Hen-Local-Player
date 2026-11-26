export type UttStatus = 'pending' | 'done' | 'noSpeech' | {failed: {attempts:number;error:string}}
export type PlaybackPlan = 'direct' | 'remux' | 'transcode'
export type ComponentState = 'missing' | 'downloading' | 'verifying' | 'ready' | {failed:string}
export type Caps = Record<string,string>
export type SubtitleFormat = 'srt' | 'vtt' | 'ass'
export type BilingualOrder = 'translation-first' | 'original-first'
export type SubtitleVariant = 'original' | 'translation' | { bilingual: BilingualOrder }
export interface SpeechRegion {
  startMs: number
  endMs: number
}
export interface Utterance {
  id: string
  startMs: number
  endMs: number
  regions: SpeechRegion[]
  status: UttStatus
  text: string | null
  language: string | null
  forcedLanguage: string | null
  asrModel: string | null
}
export interface Cue {
  id: string
  utteranceId: string
  startMs: number
  endMs: number
  original: string
  translation: string | null
  lang: string | null
  stale: boolean
}
export interface VideoStream {
  codecName: string
  codecTagString: string
  profile: string
  pixFmt: string
  width: number
  height: number
  fieldOrder: string
}
export interface AudioStream {
  index: number
  order: number
  codecName: string
  channels: number
  channelLayout: string
  sampleRate: number
  languageTag: string | null
  default: boolean
}
export interface MediaInfo {
  container: string
  durationMs: number
  video: VideoStream | null
  audio: AudioStream[]
  hasSubtitleStreams: boolean
}
export interface UrlInfo {
  id: string
  title: string
  duration: number | null
  extractor: string
  thumbnail: string | null
  isLive: boolean
}
export interface ComponentStatus {
  id: string
  ready: boolean
  bytesTotal: number
  bytesDone: number
  state: ComponentState
}
export interface DownloadProgress {
  part: number
  parts: number
  downloaded: number
  total: number | null
  eta: number | null
}
export interface RecentItem {
  fingerprint: string
  path: string
  title: string
  durationMs: number
  lastOpened: number
  progress: number
}
export interface LanguageEvent {
  sessionId: string
  mode: string
  detected: string | null
  confidence: number
  locked: boolean
  unverifiedTranslation: boolean
}
export interface MediaOpened {
  sourcePath?: string
  sessionId: string
  title: string
  durationMs: number
  mediaUrl: string | null
  plan: PlaybackPlan
  audioTracks: AudioStream[]
  language: LanguageEvent
  target: string
  cues: Cue[]
}
export interface PlaybackPrep {
  sessionId: string
  plan: PlaybackPlan
  progress: number
  mediaUrl: string | null
  error: string | null
}
export interface CueBatch {
  sessionId: string
  upserts: Cue[]
  removedIds: string[]
}
export interface PipelineStatus {
  sessionId: string
  extractedUntilMs: number
  vadComplete: boolean
  uttTotal: number
  uttDone: number
  uttNoSpeech: number
  uttFailed: number
  translatedCount: number
  translationFailed: number
  asrPendingSpans: [number, number][]
  trPendingSpans: [number, number][]
  asrRtf: number
  msPerTranslation: number
  etaSeconds: number | null
  stage: string
}
export interface ErrorEvent {
  sessionId: string | null
  code: string
  message: string
  recoverable: boolean
}
export interface Settings {
  autoDownloadUpdates: boolean
  accentTheme: 'neon-blue' | 'neon-orange' | 'neon-pink' | 'neon-green' | 'system'
  uiLanguage: string
  targetLanguage: string
  sourceLanguage: string
  subtitleMode: string
  bilingualOrder: BilingualOrder
  subtitleScale: number
  subtitleBackground: number
  subtitleBottomOffset: number
  showTranscriptPanel: boolean
  lookaheadSeconds: number
  processWholeFile: boolean
  dialogueChannelFocus: boolean
  asrModel: string
  modelDownloadSource: 'auto' | 'china' | 'global'
  translationContext: boolean
  downloadDir: string
  cacheLimitGb: number
}

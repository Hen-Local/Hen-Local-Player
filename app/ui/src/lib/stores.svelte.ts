import { TauriBackend, type Backend, type Events } from './backend'
import { MockBackend } from './mock'
import { defaults } from './defaults'
import { capMimes } from './format'
import en from './i18n/en'
import zh from './i18n/zh'
import type {
  ComponentStatus,
  Cue,
  DownloadProgress,
  LanguageEvent,
  MediaOpened,
  PipelineStatus,
  PlaybackPrep,
  RecentItem,
  Settings,
} from './types'

export const mockMode = !('__TAURI_INTERNALS__' in window)
export const backend: Backend = mockMode ? new MockBackend() : new TauriBackend()
export type Dialog = 'settings' | 'setup' | 'url' | 'export' | null
export const modes = ['original', 'translation', 'both', 'off'] as const
export function cueAt(cues: Cue[], ms: number) {
  let lo = 0,
    hi = cues.length
  while (lo < hi) {
    const mid = (lo + hi) >>> 1
    if (cues[mid].startMs <= ms) lo = mid + 1
    else hi = mid
  }
  return lo > 0 && ms < cues[lo - 1].endMs ? lo - 1 : -1
}
class PlayerState {
  settings = $state<Settings>(defaults())
  media = $state<MediaOpened | null>(null)
  prep = $state<PlaybackPrep | null>(null)
  status = $state<PipelineStatus | null>(null)
  language = $state<LanguageEvent | null>(null)
  cues = $state.raw<Cue[]>([])
  cueIndex = new Map<string, number>()
  components = $state<ComponentStatus[]>([])
  recent = $state<RecentItem[]>([])
  download = $state<DownloadProgress | null>(null)
  dialog = $state<Dialog>(null)
  ms = $state(0)
  duration = $state(0)
  playing = $state(false)
  volume = $state(0.85)
  rate = $state(1)
  fullscreen = $state(false)
  track = $state(0)
  error = $state('')
  notice = $state('')
  video: HTMLVideoElement | null = null
  info = $state({ version: '', logDir: '', cacheBytes: 0 })
  locale = $derived<'zh' | 'en'>(
    (this.settings.uiLanguage === 'system'
      ? navigator.language
      : this.settings.uiLanguage
    ).startsWith('zh')
      ? 'zh'
      : 'en',
  )
  strings = $derived(this.locale === 'zh' ? zh : en)
  active = $derived(cueAt(this.cues, this.ms))
  progress = $derived(
    this.status?.uttTotal
      ? Math.min(1, (this.status.uttDone + this.status.uttNoSpeech) / this.status.uttTotal)
      : this.status?.stage === 'complete'
        ? 1
        : 0,
  )
  private timer = 0
  private cleanups: (() => void)[] = []
  private disposed = false
  private queued: { sessionId: string; action: () => Promise<void> } | null = null
  t = (key: keyof typeof en) => this.strings[key]
  message(e: unknown) {
    this.error = e instanceof Error ? e.message : String(e)
  }
  toast(s: string) {
    this.notice = s
    clearTimeout(this.timer)
    this.timer = window.setTimeout(() => (this.notice = ''), 3200)
  }
  async act(action: () => Promise<unknown>) {
    try {
      await action()
    } catch (e) {
      this.message(e)
    }
  }
  async update(patch: Partial<Settings>) {
    this.settings = await backend.call('update_settings', { patch })
  }
  async open(path: string) {
    await this.act(() => backend.call('open_media', { path }))
  }
  async choose() {
    await this.act(async () => {
      const path = await backend.openFile()
      if (path) await this.open(path)
    })
  }
  async setSource(code: string) {
    await backend.call('set_source_language', { modeOrCode: code })
    this.settings = { ...this.settings, sourceLanguage: code }
  }
  async setTarget(code: string) {
    await backend.call('set_target_language', { code })
    this.settings = { ...this.settings, targetLanguage: code }
  }
  async setTrack(order: number) {
    await backend.call('select_audio_track', { order })
    this.track = order
  }
  seek(ms: number) {
    if (this.video) {
      this.video.currentTime = Math.min(Math.max(ms, 0), this.duration) / 1000
      this.ms = this.video.currentTime * 1000
    }
  }
  jump(dir: number) {
    let i =
      dir < 0
        ? this.cues.map((c) => c.startMs < this.ms - 150).lastIndexOf(true)
        : this.cues.findIndex((c) => c.startMs > this.ms + 150)
    if (i >= 0) this.seek(this.cues[i].startMs)
  }
  toggle() {
    if (this.video) {
      if (this.video.paused) void this.video.play().catch((e) => this.message(e))
      else this.video.pause()
    }
  }
  async full(value: boolean) {
    await backend.fullscreen(value)
    this.fullscreen = value
  }
  cycle() {
    return this.update({
      subtitleMode:
        modes[(modes.indexOf(this.settings.subtitleMode as (typeof modes)[number]) + 1) % 4],
    })
  }
  scale(delta: number) {
    return this.update({
      subtitleScale: Math.max(
        0.6,
        Math.min(2, Math.round((this.settings.subtitleScale + delta) * 10) / 10),
      ),
    })
  }
  scheduleExport(action: () => Promise<void>) {
    if (this.media) this.queued = { sessionId: this.media.sessionId, action }
  }
  private replace(cues: Cue[]) {
    this.cues = [...cues].sort((a, b) => a.startMs - b.startMs || a.id.localeCompare(b.id))
    this.cueIndex = new Map(this.cues.map((c, i) => [c.id, i]))
  }
  private own(id: string) {
    return id === this.media?.sessionId
  }
  private async listen<K extends keyof Events>(key: K, handler: (e: Events[K]) => void) {
    const fn = await backend.listen(key, handler)
    if (this.disposed) fn()
    else this.cleanups.push(fn)
  }
  async init() {
    this.disposed = false
    await Promise.all([
      this.listen('media://opened', (e) => {
        this.media = e
        this.prep = null
        this.status = null
        this.language = e.language
        this.replace(e.cues)
        this.ms = 0
        this.duration = e.durationMs
        this.error = ''
        this.queued = null
        this.track = e.audioTracks.find((a) => a.default)?.order ?? 0
        this.settings = { ...this.settings, targetLanguage: e.target }
        void this.act(async () => {
          this.recent = await backend.call('list_recent')
        })
      }),
      this.listen('media://prep', (e) => {
        if (this.own(e.sessionId)) {
          this.prep = e
          if (e.error) this.message(e.error)
        }
      }),
      this.listen('pipeline://cues', (e) => {
        if (!this.own(e.sessionId)) return
        const rows = [...this.cues]
        for (const c of e.upserts) {
          const index = this.cueIndex.get(c.id)
          if (index === undefined) rows.push(c)
          else rows[index] = c
        }
        const removed = new Set(e.removedIds)
        this.replace(rows.filter((c) => !removed.has(c.id)))
      }),
      this.listen('pipeline://status', (s) => {
        if (!this.own(s.sessionId)) return
        this.status = s
        if (s.stage === 'complete' && this.queued?.sessionId === s.sessionId) {
          const q = this.queued
          this.queued = null
          void this.act(q.action)
        }
      }),
      this.listen('pipeline://language', (e) => {
        if (this.own(e.sessionId)) this.language = e
      }),
      this.listen('setup://progress', (e) => {
        this.components = [...this.components.filter((c) => c.id !== e.id), e].sort((a, b) =>
          a.id.localeCompare(b.id),
        )
      }),
      this.listen('download://progress', (e) => (this.download = e)),
      this.listen('app://error', (e) => {
        if (!e.sessionId || this.own(e.sessionId)) this.message(e.message)
      }),
    ])
    const unDrop = await backend.onDrop((path) => void this.open(path))
    if (this.disposed) {
      unDrop()
      return
    }
    this.cleanups.push(unDrop)
    const video = document.createElement('video')
    await backend.call('set_playback_capabilities', {
      caps: Object.fromEntries(capMimes.map((m) => [m, video.canPlayType(m)])),
    })
    const [settings, components, recent, info] = await Promise.all([
      backend.call('get_settings'),
      backend.call('get_setup_status'),
      backend.call('list_recent'),
      backend.call('app_info'),
    ])
    this.settings = settings
    this.components = components
    this.recent = recent
    this.info = info
    if (
      components.some(
        (c) =>
          !c.ready &&
          ![
            'yt-dlp',
            'deno',
            settings.asrModel === 'qwen3-asr-0.6b' ? 'asr-qwen3-1.7b-mlx' : 'asr-qwen3-0.6b-mlx',
          ].includes(c.id),
      )
    )
      this.dialog = 'setup'
  }
  dispose() {
    this.disposed = true
    for (const fn of this.cleanups) fn()
    this.cleanups = []
    clearTimeout(this.timer)
  }
}
export const player = new PlayerState()

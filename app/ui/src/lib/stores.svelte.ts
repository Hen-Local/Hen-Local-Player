import { TauriBackend, type Backend, type Events } from './backend'
import { MockBackend } from './mock'
import { ResumeHistory } from './resume'
import { subtitleTracks } from './subtitle-tracks.svelte'
import { defaults } from './defaults'
import { getCurrentWindow } from '@tauri-apps/api/window'
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
  video = $state.raw<HTMLVideoElement | null>(null)
  videoFit = $state<'contain' | 'cover'>('contain')
  queue = $state<{ path: string; title: string }[]>([])
  currentPath = $state('')
  queueIndex = $derived(this.queue.findIndex((item) => item.path === this.currentPath))
  repeat = $state<'off' | 'all' | 'one'>('off')
  private positions = new ResumeHistory()
  private fileOperations = Promise.resolve()
  private switchPosition: { path: string; ms: number; playing: boolean } | null = null
  private lastSave = 0
  private lastVolume = 0.85
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
  private queueItems(paths: string[]) {
    return [...new Set(paths.filter(Boolean))].map((path) => ({
      path,
      title: path.split(/[\\/]/).pop() || path,
    }))
  }
  private fileOperation(operation: () => Promise<void>) {
    // File opens replace a native session. Keep picker, Finder and playlist
    // operations in arrival order even when probing a slow file.
    this.fileOperations = this.fileOperations.then(() => this.act(operation))
    return this.fileOperations
  }
  async openPaths(paths: string[]) {
    const items = this.queueItems(paths)
    if (!items.length) return
    await this.fileOperation(async () => {
      this.remember(true)
      const previous = this.queue
      this.queue = items
      try {
        await backend.call('open_media', { path: items[0].path })
      } catch (error) {
        this.queue = previous
        throw error
      }
    })
  }
  open(path: string) {
    const index = this.queue.findIndex((item) => item.path === path)
    return index < 0 ? this.openPaths([path]) : this.openQueueIndex(index)
  }
  async choose() {
    await this.act(async () => this.openPaths(await backend.openFiles()))
  }
  async chooseFolder() {
    await this.act(async () => this.openPaths(await backend.openFolder()))
  }
  async addToQueue() {
    await this.act(async () => {
      const paths = await backend.openFiles()
      if (!this.media) return this.openPaths(paths)
      this.queue.push(
        ...this.queueItems(paths).filter((item) => !this.queue.some((q) => q.path === item.path)),
      )
    })
  }
  async openQueueIndex(index: number) {
    const item = this.queue[index]
    if (!item) return
    await this.fileOperation(async () => {
      this.remember(true)
      await backend.call('open_media', { path: item.path })
    })
  }
  async removeQueueItem(index: number) {
    const item = this.queue[index]
    if (!item) return
    if (item.path === this.currentPath) {
      if (this.queue.length === 1) return this.close()
      const replacement = this.queue[index + 1] ?? this.queue[index - 1]
      await this.fileOperation(async () => {
        this.remember(true)
        await backend.call('open_media', { path: replacement.path })
        this.queue = this.queue.filter((entry) => entry.path !== item.path)
      })
    } else this.queue = this.queue.filter((entry) => entry.path !== item.path)
  }
  next() {
    const index = this.queueIndex + 1
    if (index < this.queue.length) return this.openQueueIndex(index)
    if (this.repeat === 'all' && this.queue.length) return this.openQueueIndex(0)
  }
  previous() {
    if (this.queueIndex > 0) return this.openQueueIndex(this.queueIndex - 1)
    if (this.repeat === 'all' && this.queue.length > 1)
      return this.openQueueIndex(this.queue.length - 1)
    this.seek(0)
  }
  toggleRepeat() {
    this.repeat = this.repeat === 'off' ? 'all' : this.repeat === 'all' ? 'one' : 'off'
  }
  ended() {
    this.remember(true)
    if (this.repeat === 'one' || (this.repeat === 'all' && this.queue.length === 1)) {
      this.seek(0)
      void this.video?.play().catch((error) => this.message(error))
    } else void this.next()
  }
  close() {
    return this.fileOperation(async () => {
      this.remember(true)
      await backend.call('close_media')
      this.video?.pause()
      this.media = null
      this.prep = this.status = this.language = null
      this.queue = []
      this.currentPath = ''
      this.switchPosition = null
      this.queued = null
      if (this.fullscreen) await this.full(false)
      this.ms = this.duration = 0
      this.playing = false
      this.replace([])
    })
  }
  resumePosition(path: string) {
    return this.positions.get(path)
  }
  remember(force = false) {
    const video = this.video
    if (!video || video.readyState < 1 || video.dataset.session !== this.media?.sessionId) return
    const now = Date.now()
    if (!force && now - this.lastSave < 4000) return
    this.lastSave = now
    this.positions.save(this.currentPath, video.currentTime * 1000, video.duration * 1000)
  }
  restoreVideo(video: HTMLVideoElement) {
    const pending = this.switchPosition?.path === this.currentPath ? this.switchPosition : null
    this.switchPosition = null
    const ms = pending?.ms ?? this.resumePosition(this.currentPath)
    if (ms > 0 && ms < video.duration * 1000) video.currentTime = ms / 1000
    if (pending?.playing !== false)
      void video.play().catch(() => {
        this.playing = false
      })
  }
  skip(deltaMs: number) {
    this.seek((this.video ? this.video.currentTime * 1000 : this.ms) + deltaMs)
  }
  mute() {
    if (this.volume) {
      this.lastVolume = this.volume
      this.volume = 0
    } else this.volume = this.lastVolume
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
    if (order === this.track) return
    this.switchPosition = { path: this.currentPath, ms: this.ms, playing: this.playing }
    try {
      await backend.call('select_audio_track', { order })
      this.track = order
    } catch (error) {
      this.switchPosition = null
      throw error
    }
  }
  seek(ms: number) {
    if (this.video) {
      this.video.currentTime = Math.min(Math.max(ms, 0), this.duration) / 1000
      this.ms = this.video.currentTime * 1000
    }
  }
  jump(dir: number) {
    const cues = subtitleTracks.authored?.cues ?? this.cues
    const index =
      dir < 0
        ? cues.map((cue) => cue.startMs < this.ms - 150).lastIndexOf(true)
        : cues.findIndex((cue) => cue.startMs > this.ms + 150)
    if (index >= 0) this.seek(cues[index].startMs)
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
    return subtitleTracks.cycle()
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
      this.listen('app://open-files', (paths) => {
        void this.openPaths(paths)
      }),
      this.listen('media://opened', (e) => {
        this.remember(true)
        const path = e.sourcePath
        if (path && !this.queue.some((item) => item.path === path))
          this.queue.push({ path, title: e.title })
        this.currentPath = path || ''
        this.playing = false
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
      this.listen('app://menu', (action) => {
        if (this.dialog) return
        switch (action) {
          case 'open':
            void this.choose()
            break
          case 'open-folder':
            void this.chooseFolder()
            break
          case 'close-media':
            void this.close()
            break
          case 'url':
            this.dialog = 'url'
            break
          case 'settings':
            this.dialog = 'settings'
            break
          case 'export':
            if (this.media) this.dialog = 'export'
            break
          case 'play-pause':
            this.toggle()
            break
          case 'previous-file':
            void this.previous()
            break
          case 'next-file':
            void this.next()
            break
          case 'seek-backward':
            this.skip(-10000)
            break
          case 'seek-forward':
            this.skip(10000)
            break
          case 'volume-up':
            this.volume = Math.min(1, this.volume + 0.05)
            break
          case 'volume-down':
            this.volume = Math.max(0, this.volume - 0.05)
            break
          case 'audio-tracks':
            if (this.media) window.dispatchEvent(new Event('hlp:playback-options'))
            break
          case 'mute':
            this.mute()
            break
          case 'repeat':
            this.toggleRepeat()
            break
          case 'fullscreen':
            void this.act(() => this.full(!this.fullscreen))
            break
          case 'playback-options':
          case 'subtitle-file':
            if (this.media) window.dispatchEvent(new Event('hlp:' + action))
            break
          case 'transcript':
          case 'playlist':
            window.dispatchEvent(new CustomEvent('player:panel', { detail: action }))
            break
          case 'subtitles':
            void this.act(() => this.cycle())
            break
        }
      }),
      this.listen('app://error', (e) => {
        if (!e.sessionId || this.own(e.sessionId)) this.message(e.message)
      }),
    ])
    const unDrop = await backend.onDrop((paths) => void this.openPaths(paths))
    if (this.disposed) {
      unDrop()
      return
    }
    this.cleanups.push(unDrop)
    const save = () => this.remember(true)
    window.addEventListener('pagehide', save)
    document.addEventListener('visibilitychange', save)
    this.cleanups.push(() => {
      window.removeEventListener('pagehide', save)
      document.removeEventListener('visibilitychange', save)
    })
    if (mockMode) {
      const reflectFullscreen = () => {
        this.fullscreen = Boolean(document.fullscreenElement)
      }
      document.addEventListener('fullscreenchange', reflectFullscreen)
      this.cleanups.push(() => document.removeEventListener('fullscreenchange', reflectFullscreen))
    } else {
      const win = getCurrentWindow()
      const reflectFullscreen = async () => {
        this.fullscreen = await win.isFullscreen()
      }
      this.cleanups.push(await win.onResized(() => void this.act(reflectFullscreen)))
      await this.act(reflectFullscreen)
    }
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
  }
  dispose() {
    this.remember(true)
    this.disposed = true
    for (const fn of this.cleanups) fn()
    this.cleanups = []
    clearTimeout(this.timer)
  }
}
export const player = new PlayerState()

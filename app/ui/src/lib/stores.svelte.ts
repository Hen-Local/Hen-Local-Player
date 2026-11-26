import { TauriBackend, sortMediaPaths, type Backend, type Events } from './backend'
import { MockBackend } from './mock'
import { AppUpdates } from './app-updates.svelte'
import { sortPlaylist, type PlaylistSort, type QueueItem } from './playlist'
import { ResumeHistory } from './resume'
import { subtitleTracks } from './subtitle-tracks.svelte'
import { defaults } from './defaults'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { capMimes } from './format'
import { lastStartedCue } from './subtitle-tracks'
import en from './i18n/en'
import type { Messages } from './i18n'
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
export const backend: Backend = mockMode ? new MockBackend() : new TauriBackend(() => player.strings)
export const appUpdates = new AppUpdates(backend)
type DeleteSubtitleRequest = { kind: 'delete-subtitles'; sessionId: string; audioTrackOrder: number }
export type Dialog = 'settings' | 'setup' | 'url' | 'export' | 'open-media' | 'online-subtitles' | DeleteSubtitleRequest | null
export const playbackSpeedPresets = [0.5, 0.75, 1, 1.25, 1.5, 2, 4]
export const modes = ['original', 'translation', 'both', 'off'] as const
export function cueAt(cues: Cue[], ms: number) {
  const index = lastStartedCue(cues, ms)
  return index >= 0 && ms < cues[index].endMs ? index : -1
}
function compareCues(a: Cue, b: Cue) {
  return a.startMs - b.startMs || a.id.localeCompare(b.id)
}
class PlayerState {
  settings = $state<Settings>(defaults())
  media = $state<MediaOpened | null>(null)
  prep = $state<PlaybackPrep | null>(null)
  status = $state.raw<PipelineStatus | null>(null)
  transcriptionRequested = $state(false)
  startingTranscription = $state(false)
  deletingSubtitles = $state(false)
  missingSubtitleComponents = $state<string[]>([])
  subtitleError = $state('')
  installingSubtitleComponent = $state('')
  private regenerationTrack: number | null = null
  canRegenerate = $derived(!!this.media?.audioTracks.length && !this.startingTranscription &&
    !this.missingSubtitleComponents.length && !this.subtitleError &&
    (this.status?.stage === 'complete' ||
      (this.status?.stage === 'idle' && this.status.vadComplete &&
        this.status.uttDone + this.status.uttNoSpeech === this.status.uttTotal &&
        !this.status.trPendingSpans.length && !this.status.translationFailed)))
  language = $state<LanguageEvent | null>(null)
  cues = $state.raw<Cue[]>([])
  canDeleteSubtitles = $derived(this.canRegenerate || (!!this.media?.audioTracks.length &&
    !this.startingTranscription && !!this.cues.length && !!this.status &&
    ['partial', 'paused', 'error', 'idle'].includes(this.status.stage)))
  private cueIndex = new Map<string, number>()
  components = $state<ComponentStatus[]>([])
  recent = $state<RecentItem[]>([])
  private recentRevision = 0
  download = $state<DownloadProgress | null>(null)
  dialog = $state<Dialog>(null)
  ms = $state(0)
  seekRevision = $state(0)
  duration = $state(0)
  playing = $state(false)
  volume = $state(0.85)
  rate = $state(1)
  fullscreen = $state(false)
  private requestedFullscreen: boolean | null = null
  private fullscreenRevision = 0
  private fullscreenOperation = Promise.resolve()
  private windowSizeRequest?: { sessionId: string; width: number; height: number }
  private windowSizeOperation = Promise.resolve()
  track = $state(0)
  error = $state('')
  notice = $state('')
  video = $state.raw<HTMLVideoElement | null>(null)
  interfaceCopy = $state.raw<{ locale: string; messages: Messages }>({ locale: 'en', messages: en })
  locale = $derived(this.interfaceCopy.locale)
  private originalQueue = $state.raw<QueueItem[]>([])
  private modifiedTimes = $state.raw(new Map<QueueItem, number | null>())
  private datesRequestedFor: QueueItem[] | null = null
  queueSort = $state<PlaylistSort>('original')
  queue = $derived(sortPlaylist(this.originalQueue, this.queueSort, this.modifiedTimes, this.locale))
  queueFolder = $state('')
  currentPath = $state('')
  queueIndex = $derived(this.queue.findIndex((item) => item.path === this.currentPath))
  repeat = $state<'off' | 'all' | 'one'>('off')
  private positions = new ResumeHistory()
  private restoredSources = new WeakMap<HTMLVideoElement, string>()
  private fileOperations = Promise.resolve()
  private switchPosition: { path: string; ms: number; playing: boolean } | null = null
  private lastSave = 0
  private lastVolume = 0.85
  info = $state({ version: '', logDir: '', cacheBytes: 0 })
  strings = $derived(this.interfaceCopy.messages)
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
    const text = e instanceof Error ? e.message : String(e)
    this.error = Object.hasOwn(this.strings, text) ? this.t(text as keyof Messages) : text
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
    if (['subtitleMode', 'translationContext', 'sourceLanguage', 'targetLanguage', 'asrModel', 'dialogueChannelFocus'].some(key => key in patch))
      await this.reportPlayhead()
    this.settings = await backend.call('update_settings', { patch })
  }
  findOnlineSubtitles() {
    if (this.dialog) return
    if (this.media) this.dialog = 'online-subtitles'
    else this.toast(this.t('openMediaToFindSubtitles'))
  }
  confirmSubtitleDeletion() {
    if (this.dialog || !this.media || !this.canDeleteSubtitles) return
    this.error = ''
    this.dialog = { kind: 'delete-subtitles', sessionId: this.media.sessionId, audioTrackOrder: this.track }
  }
  async deleteGeneratedSubtitles() {
    const target = this.dialog
    if (!target || typeof target !== 'object' || this.deletingSubtitles) return
    if (!this.own(target.sessionId) || this.track !== target.audioTrackOrder || !this.canDeleteSubtitles) {
      this.dialog = null
      return
    }
    const media = this.media
    const previousStatus = this.status
    this.error = ''
    this.deletingSubtitles = true
    try {
      await backend.call('delete_generated_subtitles', { sessionId: target.sessionId, audioTrackOrder: target.audioTrackOrder })
      if (this.media !== media || this.track !== target.audioTrackOrder) return
      this.replace([])
      this.transcriptionRequested = false
      this.regenerationTrack = null
      this.subtitleError = ''
      this.missingSubtitleComponents = []
      this.queued = null
      if (this.status === previousStatus) this.status = null
      if (this.dialog === target) {
        this.dialog = null
        this.toast(this.t('subtitlesDeleted'))
        return true
      }
    } catch (error) {
      if (this.media === media && this.track === target.audioTrackOrder) this.message(error)
    } finally {
      if (this.media === media) this.deletingSubtitles = false
    }
  }
  async startTranscription(options: { originalOnly?: boolean; regenerate?: boolean } = {}) {
    const media = this.media
    if (!media || this.startingTranscription || this.deletingSubtitles) return
    if (options.regenerate && !this.canRegenerate) return
    if (options.regenerate) this.regenerationTrack = this.track
    this.startingTranscription = true
    this.subtitleError = ''
    try {
      if (options.originalOnly) await this.setTarget('none')
      if (this.media !== media) return
      if (subtitleTracks.selected !== 'generated' || this.settings.subtitleMode === 'off')
        await subtitleTracks.select('generated')
      if (this.media !== media || subtitleTracks.selected !== 'generated') return
      this.transcriptionRequested = true
      const track = this.regenerationTrack
      const previousStatus = this.status
      await this.reportPlayhead()
      if (this.media !== media) return
      const { missing } = await backend.call('start_transcription', track === null
        ? { sessionId: media.sessionId }
        : { sessionId: media.sessionId, regenerate: true, audioTrackOrder: track })
      if (this.media === media) {
        this.missingSubtitleComponents = missing
        if (missing.some(id => !this.components.some(c => c.id === id))) {
          const previous = this.components
          void this.act(async () => {
            const catalog = await backend.call('get_setup_status')
            if (!this.disposed && this.components === previous) this.components = catalog
          })
        }
        if (!missing.length) {
          this.regenerationTrack = null
          // Silence can finish immediately. For queued speech, wait for the new
          // run's first status unless it already arrived before the command reply.
          if ((track !== null || previousStatus?.stage === 'partial') && previousStatus?.uttTotal && this.status === previousStatus)
            this.status = null
        }
      }
    } catch (error) {
      if (this.media === media) this.subtitleError = String(error instanceof Error ? error.message : error)
    } finally {
      if (this.media === media) this.startingTranscription = false
    }
  }
  private componentReady(id: string) {
    if (!this.missingSubtitleComponents.includes(id)) return
    this.missingSubtitleComponents = this.missingSubtitleComponents.filter((key) => key !== id)
    if (this.media && !this.missingSubtitleComponents.length && this.transcriptionRequested && subtitleTracks.selected === 'generated')
      return this.startTranscription()
  }
  async installSubtitleComponent(component: string) {
    const id = this.media?.sessionId
    if (!id || this.installingSubtitleComponent || !this.missingSubtitleComponents.includes(component)) return
    this.installingSubtitleComponent = component
    this.subtitleError = ''
    try {
      await backend.call('start_setup', { includeUrlTools: false, ids: [component] })
      this.components = await backend.call('get_setup_status')
      if (this.components.some((c) => c.id === component && c.ready))
        await this.componentReady(component)
    } catch (error) {
      if (this.own(id)) this.subtitleError = String(error instanceof Error ? error.message : error)
    } finally {
      this.installingSubtitleComponent = ''
    }
  }
  setQueueSort(order: PlaylistSort) {
    this.queueSort = order
    void this.loadQueueDates()
  }
  private setQueue(items: QueueItem[], folder: string) {
    this.originalQueue = items
    this.queueFolder = folder
    this.datesRequestedFor = null
    // Keep dates for retained entries. Edits to a long, time-sorted playlist
    // should neither reshuffle existing files nor stat the entire folder again.
    const retained = new Map<QueueItem, number | null>()
    for (const item of items) {
      const date = this.modifiedTimes.get(item)
      if (date !== undefined) retained.set(item, date)
    }
    this.modifiedTimes = retained
    void this.loadQueueDates()
  }
  private async loadQueueDates() {
    if (this.queueSort !== 'newest' && this.queueSort !== 'oldest') return
    const items = this.originalQueue
    if (!items.length || this.datesRequestedFor === items) return
    this.datesRequestedFor = items
    const missing = items.filter(item => !this.modifiedTimes.has(item))
    if (!missing.length) return
    try {
      const dates = await backend.call('playlist_file_times', { paths: missing.map(item => item.path) })
      if (this.originalQueue !== items || this.disposed) return
      const times = new Map(this.modifiedTimes)
      dates.forEach((ms, index) => times.set(missing[index], ms))
      this.modifiedTimes = times
    } catch (error) {
      if (this.originalQueue !== items || this.disposed) return
      this.datesRequestedFor = null
      if (this.queueSort === 'newest' || this.queueSort === 'oldest') this.message(error)
    }
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
  private async loadSiblings(path: string) {
    const pending = this.originalQueue
    try {
      const folder = await backend.call('list_media_siblings', { path })
      if (!folder) return
      // Apply after any in-flight file open has committed or rolled back.
      // Explicit opens and edits own the newer list if a slow drive finishes late.
      await this.fileOperation(async () => {
        if (this.originalQueue !== pending || this.disposed) return
        this.setQueue(this.queueItems(sortMediaPaths(folder.files)), folder.folder)
      })
    } catch {
      // The file itself can be readable without permission to list its parent.
    }
  }
  async openPaths(paths: string[], browseFolder = true, folder = '') {
    const items = this.queueItems(paths)
    if (!items.length) return
    await this.fileOperation(async () => {
      this.remember(true)
      const previous = this.originalQueue
      const previousFolder = this.queueFolder
      this.setQueue(items, folder)
      try {
        await backend.call('open_media', { path: items[0].path })
      } catch (error) {
        this.setQueue(previous, previousFolder)
        throw error
      }
      if (browseFolder && items.length === 1 && this.originalQueue === items) {
        void this.loadSiblings(items[0].path)
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
  async chooseMedia() {
    if (mockMode || !/Mac/.test(navigator.platform)) {
      this.dialog = 'open-media'
      return
    }
    await this.act(async () => {
      const sources = await backend.call('choose_media_sources', { title: this.t('open') + ' / ' + this.t('openFolder') })
      const only = sources.length === 1 ? sources[0] : null
      const paths = sources.flatMap(source => typeof source === 'string' ? [source] : sortMediaPaths(source.files))
      await this.openPaths(paths, typeof only === 'string', only && typeof only !== 'string' ? only.folder : '')
    })
  }
  async chooseFolder() {
    await this.act(async () => this.openPaths(await backend.openFolder(), false))
  }
  async addToQueue() {
    await this.act(async () => {
      const paths = await backend.openFiles()
      if (!this.media) return this.openPaths(paths, false)
      const additions = this.queueItems(paths).filter((item) => !this.queue.some((q) => q.path === item.path))
      if (additions.length) {
        this.setQueue([...this.originalQueue, ...additions], '')
      }
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
        this.setQueue(this.originalQueue.filter((entry) => entry.path !== item.path), '')
      })
    } else {
      this.setQueue(this.originalQueue.filter((entry) => entry.path !== item.path), '')
    }
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
      if (typeof this.dialog === 'object') this.dialog = null
      this.video?.pause()
      this.media = null
      this.prep = this.status = this.language = null
      this.setQueue([], '')
      this.currentPath = ''
      subtitleTracks.attach('')
      this.switchPosition = null
      this.queued = null
      if (this.fullscreen) await this.full(false)
      this.ms = this.duration = 0
      this.playing = false
      this.replace([])
      await this.refreshRecent()
    })
  }
  resumePosition(path: string) {
    return this.positions.get(path)
  }
  async reportPlayhead() {
    const video = this.video, sessionId = this.media?.sessionId
    if (sessionId && video?.dataset.session === sessionId && video.readyState >= 1 && this.restoredSources.get(video) === video.currentSrc)
      await backend.call('set_playhead', { sessionId, ms: Math.round(video.currentTime * 1000), playing: !video.paused })
  }
  remember(force = false) {
    const video = this.video
    if (!video || video.readyState < 1 || video.dataset.session !== this.media?.sessionId) return
    // WebKit can deliver durationchange at time zero, before loadedmetadata restores the saved position.
    if (this.restoredSources.get(video) !== video.currentSrc) return
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
    this.restoredSources.set(video, video.currentSrc)
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
    const id = this.media?.sessionId
    await this.reportPlayhead()
    await backend.call('set_source_language', { modeOrCode: code })
    this.settings = { ...this.settings, sourceLanguage: code }
    if (id && this.own(id) && this.missingSubtitleComponents.length && subtitleTracks.selected === 'generated' && !this.startingTranscription)
      await this.startTranscription()
  }
  async setTarget(code: string) {
    const id = this.media?.sessionId
    await this.reportPlayhead()
    await backend.call('set_target_language', { code })
    this.settings = { ...this.settings, targetLanguage: code }
    if (id && this.own(id) && this.missingSubtitleComponents.length && subtitleTracks.selected === 'generated' && !this.startingTranscription)
      await this.startTranscription()
  }
  async setTrack(order: number) {
    if (order === this.track) return
    if (typeof this.dialog === 'object') this.dialog = null
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
      this.seekRevision++
    }
  }
  jump(dir: number) {
    const authored = subtitleTracks.authored
    const cues = authored?.cues ?? this.cues
    const delay = authored ? 0 : subtitleTracks.delayMs
    const now = this.ms - delay
    const index =
      dir < 0
        ? cues.map((cue) => cue.startMs < now - 150).lastIndexOf(true)
        : cues.findIndex((cue) => cue.startMs > now + 150)
    if (index >= 0) this.seek(cues[index].startMs + delay)
  }
  toggle() {
    if (this.video) {
      if (this.video.paused) void this.video.play().catch((e) => this.message(e))
      else this.video.pause()
    }
  }
  async full(value: boolean) {
    const revision = ++this.fullscreenRevision
    const previous = this.fullscreen
    this.fullscreen = value
    if (!mockMode) this.requestedFullscreen = value
    const operation = this.fullscreenOperation.then(() => {
      if (revision === this.fullscreenRevision) return backend.fullscreen(value)
    })
    this.fullscreenOperation = operation.catch(() => {})
    try {
      await operation
    } catch (error) {
      const actual = mockMode ? previous : await getCurrentWindow().isFullscreen().catch(() => previous)
      if (revision === this.fullscreenRevision) {
        this.requestedFullscreen = null
        this.fullscreen = actual
      }
      throw error
    }
  }
  syncVideoWindowAspect(element: HTMLVideoElement, force = false) {
    const sessionId = element.dataset.session
    if (mockMode || !sessionId || !this.own(sessionId) || element.readyState < 1) return
    const width = element.videoWidth, height = element.videoHeight
    const previous = this.windowSizeRequest
    if (!force && previous?.sessionId === sessionId && previous.width === width && previous.height === height) return
    const size = { sessionId, width, height }
    this.windowSizeRequest = size
    this.windowSizeOperation = this.windowSizeOperation.then(async () => {
      if (size === this.windowSizeRequest && this.own(sessionId)) await backend.call('set_video_window_aspect', size)
    }).catch(error => {
      if (size === this.windowSizeRequest && this.own(sessionId)) {
        this.windowSizeRequest = undefined
        this.message(error)
      }
    })
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
    this.cues = [...cues].sort(compareCues)
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
  private async refreshRecent() {
    const revision = ++this.recentRevision
    const recent = await backend.call('list_recent')
    if (!this.disposed && revision === this.recentRevision) this.recent = recent
  }
  init() {
    this.disposed = false
    // History can paint immediately. A click on it must wait for media events
    // and the native capability handshake before opening its file.
    return Promise.all([this.refreshRecent(), this.fileOperation(() => this.connectPlayback())])
  }
  private async connectPlayback() {
    const initialSettings = this.settings
    await Promise.all([
      this.act(async () => {
        const settings = await backend.call('get_settings')
        if (!this.disposed && this.settings === initialSettings) this.settings = settings
      }),
      this.listen('app://open-files', (paths) => {
        void this.openPaths(paths)
      }),
      this.listen('media://opened', (e) => {
        if (typeof this.dialog === 'object') this.dialog = null
        this.deletingSubtitles = false
        this.remember(true)
        const path = e.sourcePath
        if (path && !this.queue.some((item) => item.path === path)) {
          this.setQueue([...this.originalQueue, { path, title: e.title }], '')
        }
        this.currentPath = path || ''
        this.playing = false
        this.media = e
        subtitleTracks.attach(e.sessionId)
        this.transcriptionRequested = false
        this.regenerationTrack = null
        this.startingTranscription = false
        this.missingSubtitleComponents = []
        this.subtitleError = ''
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
        void this.act(() => this.refreshRecent())
      }),
      this.listen('media://prep', (e) => {
        if (this.own(e.sessionId)) {
          this.prep = e
          if (e.error) this.message(e.error)
        }
      }),
      this.listen('pipeline://cues', (e) => {
        if (!this.own(e.sessionId)) return
        const rows = this.cues.slice()
        let orderChanged = false
        for (const c of e.upserts) {
          const index = this.cueIndex.get(c.id)
          if (index === undefined) {
            const last = rows.at(-1)
            if (last && compareCues(last, c) > 0) orderChanged = true
            this.cueIndex.set(c.id, rows.length)
            rows.push(c)
          } else {
            if (rows[index].startMs !== c.startMs) orderChanged = true
            rows[index] = c
          }
        }
        if (e.removedIds.length) {
          const removed = new Set(e.removedIds)
          this.replace(rows.filter((c) => !removed.has(c.id)))
        } else if (orderChanged) this.replace(rows)
        else this.cues = rows
      }),
      this.listen('pipeline://status', (s) => {
        if (!this.own(s.sessionId)) return
        this.status = s
        // A previously requested track can resume before any cues have arrived.
        if (this.media?.audioTracks.length && ['processing', 'paused', 'partial'].includes(s.stage))
          this.transcriptionRequested = true
        if (s.stage === 'processing' || s.stage === 'complete' || s.stage === 'partial')
          this.subtitleError = ''
        if ((s.stage === 'complete' || s.stage === 'partial') && this.queued?.sessionId === s.sessionId) {
          const q = this.queued
          this.queued = null
          if (s.stage === 'complete') void this.act(q.action)
          else this.toast(this.t('partialExport'))
        }
      }),
      this.listen('pipeline://language', (e) => {
        if (this.own(e.sessionId)) this.language = e
      }),
      this.listen('setup://progress', (e) => {
        this.components = [...this.components.filter((c) => c.id !== e.id), e].sort((a, b) =>
          a.id.localeCompare(b.id),
        )
        if (e.ready && !this.installingSubtitleComponent) void this.componentReady(e.id)
      }),
      this.listen('download://progress', (e) => (this.download = e)),
      this.listen('app://menu', (action) => {
        if (this.dialog) return
        switch (action) {
          case 'open':
            if (/Mac/.test(navigator.platform)) void this.chooseMedia()
            else void this.choose()
            break
          case 'open-folder':
            void this.chooseFolder()
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
          case 'close-file':
            if (this.media) void this.close()
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
          case 'subtitle-file':
            if (this.media) window.dispatchEvent(new Event('hlp:' + action))
            break
          case 'find-subtitles':
            this.findOnlineSubtitles()
            break
          case 'transcript':
          case 'library':
          case 'playlist':
            window.dispatchEvent(new CustomEvent('player:panel', { detail: action }))
            break
        }
      }),
      this.listen('app://error', (e) => {
        if (e.sessionId && !this.own(e.sessionId)) return
        if (e.sessionId && this.transcriptionRequested && ['AsrUnavailable', 'Translation', 'Extraction'].includes(e.code)) {
          this.subtitleError = e.message
          return
        }
        this.message(e.message)
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
      if (/Mac/.test(navigator.platform)) {
        this.cleanups.push(await win.onCloseRequested(() => {
          this.video?.pause()
          this.remember(true)
        }))
      }
      let resizeTimer = 0
      let resizeRevision = 0
      let lastNativeFullscreen = false
      const reflectFullscreen = async () => {
        const resize = ++resizeRevision
        const request = this.fullscreenRevision
        await this.fullscreenOperation
        if (request !== this.fullscreenRevision || this.disposed) return
        const active = await win.isFullscreen()
        if (request !== this.fullscreenRevision || resize !== resizeRevision || this.disposed) return
        // A resize can belong to an earlier transition. Keep the latest
        // requested state until the native window reaches that destination.
        if (this.requestedFullscreen !== null && active !== this.requestedFullscreen) return
        const leftFullscreen = !active && (lastNativeFullscreen || this.requestedFullscreen === false)
        lastNativeFullscreen = active
        this.requestedFullscreen = null
        this.fullscreen = active
        if (leftFullscreen) {
          if (this.video && this.media) this.syncVideoWindowAspect(this.video, true)
          else if (!this.media) await backend.call('set_video_window_aspect', { sessionId: null, width: 0, height: 0 })
        }
      }
      const unlisten = await win.onResized(() => {
        window.clearTimeout(resizeTimer)
        resizeTimer = window.setTimeout(() => void this.act(reflectFullscreen), 80)
      })
      this.cleanups.push(() => {
        window.clearTimeout(resizeTimer)
        unlisten()
      })
      await this.act(reflectFullscreen)
    }
    const video = document.createElement('video')
    await backend.call('set_playback_capabilities', {
      caps: Object.fromEntries(capMimes.map((m) => [m, video.canPlayType(m)])),
    })
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

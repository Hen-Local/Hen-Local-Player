import type { Backend, Commands, Events } from './backend'
import type {
  ComponentStatus,
  Cue,
  MediaOpened,
  RecentItem,
  Settings,
  SubtitleVariant,
  SubtitleFormat,
} from './types'
import { defaults } from './defaults'
import { readSubtitleFile } from './subtitle-tracks'
import { languages } from './languages'
import mediaExtensions from '../../../media-formats.json'

const demoPath = 'fixture_ja.mp4'
const sample = [
  [500, 2750, '今日はとてもいい天気です', 'The weather is lovely today', '今天的天气很好'],
  [
    4056,
    6300,
    '駅の前で友達と会いました',
    'I met a friend outside the station',
    '我在车站前见到了朋友',
  ],
  [7767, 10300, '私たちは公園まで歩きました', 'We walked to the park', '我们步行去了公园'],
  [11866, 14100, '小さな子供が遊んでいます', 'A little child is playing', '一个小孩正在玩耍'],
  [15499, 17600, '夕方には家に帰ります', 'I will go home this evening', '傍晚我会回家'],
  [
    19072,
    21000,
    '明日もここに来たいです',
    'I would like to come back tomorrow',
    '明天我还想来这里',
  ],
] as const
const wait = (ms: number) => new Promise((resolve) => setTimeout(resolve, ms))
function stored<T>(key: string, fallback: T): T {
  try {
    return JSON.parse(localStorage.getItem('hlp.mock.' + key) ?? 'null') ?? fallback
  } catch {
    return fallback
  }
}
function persist(key: string, v: unknown) {
  try {
    localStorage.setItem('hlp.mock.' + key, JSON.stringify(v))
  } catch {
    /* private browsing */
  }
}
type Reply<K extends keyof Commands> = Commands[K][1]
const initialComponents: [string, number, boolean][] = [
  ['asr-qwen3-1.7b-mlx', 2473308160, true],
  ['asr-qwen3-0.6b-mlx', 1010000000, false],
  ['vad-silero', 643854, true],
  ['mt-hy-mt2-1.8b', 1133080448, true],
  ['llama-runtime-b11429', 11000000, true],
  ['ffmpeg', 54000000, true],
  ['yt-dlp', 31000000, false],
  ['deno', 40000000, false],
]
export class MockBackend implements Backend {
  private events = new EventTarget()
  private settings = { ...defaults(), ...stored<Partial<Settings>>('settings', {}) }
  private recent = stored<RecentItem[]>('recent', [])
  private sources = new Map<string, string>()
  private cueCache = new Map<string, Cue[]>(stored<[string, Cue[]][]>('samples', []))
  private companions = new Map<string, File[]>()
  private generating = false
  private playhead = 0
  private visibleCues = new Map<string, Cue>()
  private current: MediaOpened | null = null
  private timer = 0
  private generation = 0
  private setupRun = 0
  private downloadRun = 0
  private components: ComponentStatus[] = initialComponents.map(([id, bytesTotal, ready]) => ({
    id,
    bytesTotal,
    ready,
    bytesDone: ready ? bytesTotal : 0,
    state: ready ? 'ready' : 'missing',
  }))

  async listen<K extends keyof Events>(event: K, receive: (payload: Events[K]) => void) {
    const listener = (e: Event) => receive((e as CustomEvent<Events[K]>).detail)
    this.events.addEventListener(event, listener)
    return () => this.events.removeEventListener(event, listener)
  }
  private emit<K extends keyof Events>(name: K, payload: Events[K]) {
    this.events.dispatchEvent(new CustomEvent(name, { detail: structuredClone(payload) }))
  }
  private file(file: File) {
    const path =
      'browser:' +
      file.size +
      ':' +
      file.lastModified +
      '/' +
      (file.webkitRelativePath || file.name)
    const old = this.sources.get(path)
    if (old) URL.revokeObjectURL(old)
    this.sources.set(path, URL.createObjectURL(file))
    return path
  }
  private mediaPaths(selection: FileList | null, directory = false) {
    const files = Array.from(selection ?? [])
    const media = files.filter(
      (f) =>
        /^(audio|video)\//.test(f.type) ||
        mediaExtensions.includes(f.name.split('.').pop()?.toLowerCase() ?? ''),
    )
    if (directory)
      media.sort((a, b) =>
        a.webkitRelativePath.localeCompare(b.webkitRelativePath, undefined, { numeric: true }),
      )
    const subtitles = files.filter((f) => /\.(srt|vtt)$/i.test(f.name))
    return media.map((file) => {
      const path = this.file(file)
      const stem = (file.webkitRelativePath || file.name).replace(/\.[^.]+$/, '')
      this.companions.set(
        path,
        subtitles
          .filter((caption) => {
            const base = (caption.webkitRelativePath || caption.name).replace(/\.[^.]+$/, '')
            return (
              base === stem ||
              (base.startsWith(stem + '.') && !base.slice(stem.length).includes('/'))
            )
          })
          .sort((a, b) => a.name.localeCompare(b.name)),
      )
      return path
    })
  }
  openFiles(directory = false): Promise<string[]> {
    return new Promise((resolve) => {
      const input = document.createElement('input')
      input.type = 'file'
      input.multiple = true
      input.accept = ['video/*', 'audio/*', ...mediaExtensions.map((ext) => '.' + ext)].join(',')
      if (directory && 'webkitdirectory' in input) input.webkitdirectory = true
      input.onchange = () => {
        resolve(this.mediaPaths(input.files, directory))
      }
      input.oncancel = () => resolve([])
      input.click()
    })
  }
  openFolder() {
    return this.openFiles(true)
  }
  async onDrop(receive: (paths: string[]) => void) {
    const over = (e: DragEvent) => e.preventDefault()
    const drop = (e: DragEvent) => {
      e.preventDefault()
      const paths = this.mediaPaths(e.dataTransfer?.files ?? null)
      if (paths.length) receive(paths)
    }
    window.addEventListener('dragover', over)
    window.addEventListener('drop', drop)
    return () => {
      window.removeEventListener('dragover', over)
      window.removeEventListener('drop', drop)
    }
  }
  async saveFile(path: string) {
    return path
  }
  async fullscreen(enabled: boolean) {
    if (enabled && !document.fullscreenElement && document.documentElement.requestFullscreen) {
      // Embedded previews can deny the browser's fullscreen request. The
      // player still expands inside its host and Escape restores the layout.
      await document.documentElement.requestFullscreen().catch(() => {})
    } else if (!enabled && document.fullscreenElement) await document.exitFullscreen()
  }
  private cues(target: string, duration = this.current?.durationMs ?? 22500): Cue[] {
    return Array.from({ length: Math.ceil(duration / 22500) }, (_, loop) =>
      sample.flatMap(([begin, end, original, en, zh], i) => {
        const startMs = loop * 22500 + begin
        if (startMs >= duration) return []
        const id = 'u' + (loop * sample.length + i)
        return [
          {
            id: id + '-0',
            utteranceId: id,
            startMs,
            endMs: Math.min(duration, loop * 22500 + end),
            original,
            translation:
              target === 'none' || target === 'ja'
                ? null
                : target.startsWith('zh')
                  ? zh
                  : target === 'en'
                    ? en
                    : null,
            lang: this.settings.sourceLanguage.startsWith('auto')
              ? 'ja'
              : this.settings.sourceLanguage,
            stale: false,
          },
        ]
      }),
    ).flat()
  }
  // Only completed sample runs are cached. Rebuild their fixed display text
  // when reading an older preview cache, just as the desktop rebuilds cues.
  private cachedCues(path: string, duration = this.current?.durationMs) {
    const key = JSON.stringify([path, this.settings.sourceLanguage, this.settings.targetLanguage])
    if (this.cueCache.has(key)) return this.cues(this.settings.targetLanguage, duration)
  }
  private stream(path: string, restored: Cue[] = []) {
    const session = this.current
    if (!session) return
    const key = JSON.stringify([path, this.settings.sourceLanguage, this.settings.targetLanguage])
    const all = this.cues(this.settings.targetLanguage)
    this.visibleCues = new Map(restored.map((c) => [c.id, c]))
    const doneIds = new Set(restored.filter((c) => !c.stale).map((c) => c.id))
    const step = () => {
      if (session !== this.current) return
      const priority = (cue: Cue) =>
        cue.endMs >= this.playhead
          ? Math.max(0, cue.startMs - this.playhead)
          : 1e9 + this.playhead - cue.startMs
      const pending = all.filter((c) => !doneIds.has(c.id))
      const next = pending
        .filter(
          (c) =>
            this.settings.processWholeFile ||
            (c.endMs >= this.playhead - 2000 &&
              c.startMs <= this.playhead + this.settings.lookaheadSeconds * 1000),
        )
        .sort((a, b) => priority(a) - priority(b))[0]
      if (next) {
        doneIds.add(next.id)
        this.visibleCues.set(next.id, next)
        this.emit('pipeline://cues', {
          sessionId: session.sessionId,
          upserts: [next],
          removedIds: [],
        })
      }
      const done = doneIds.size === all.length,
        spans = pending
          .filter((c) => c !== next)
          .map((c) => [c.startMs, c.endMs] as [number, number])
      this.emit('pipeline://status', {
        sessionId: session.sessionId,
        stage: done ? 'complete' : next ? 'processing' : 'paused',
        extractedUntilMs: session.durationMs,
        vadComplete: true,
        uttTotal: all.length,
        uttDone: doneIds.size,
        uttNoSpeech: 0,
        uttFailed: 0,
        translatedCount: [...this.visibleCues.values()].filter((c) => c.translation).length,
        asrPendingSpans: spans,
        trPendingSpans: this.settings.targetLanguage === 'none' ? [] : spans,
        asrRtf: 24.2,
        msPerTranslation: 180,
        etaSeconds: done ? 0 : all.length - doneIds.size,
      })
      if (done) {
        this.cueCache.set(key, all)
        if (path === demoPath)
          persist(
            'samples',
            [...this.cueCache].filter(([k]) => JSON.parse(k)[0] === demoPath),
          )
        clearInterval(this.timer)
        const recent = this.recent.find((r) => r.path === path)
        if (recent) {
          recent.progress = 1
          persist(
            'recent',
            this.recent.filter((r) => r.path === demoPath),
          )
        }
      }
    }
    clearInterval(this.timer)
    this.timer = window.setInterval(step, 600)
    step()
  }
  private async duration(path: string) {
    const src = this.sources.get(path)
    if (!src) return 22500
    return new Promise<number>((resolve) => {
      const video = document.createElement('video')
      let timer = 0
      const finish = (value: number) => {
        clearTimeout(timer)
        video.onloadedmetadata = null
        video.onerror = null
        video.removeAttribute('src')
        video.load()
        resolve(value)
      }
      timer = window.setTimeout(() => finish(22500), 1500)
      video.preload = 'metadata'
      video.onloadedmetadata = () =>
        finish(Number.isFinite(video.duration) ? Math.round(video.duration * 1000) : 22500)
      video.onerror = () => finish(22500)
      video.src = src
    })
  }
  private currentPath = ''
  private async open(path: string) {
    const turn = ++this.generation
    this.generating = false
    clearInterval(this.timer)
    this.currentPath = path
    this.playhead = 0
    const durationMs = await this.duration(path)
    if (turn !== this.generation) return
    const title = path.split(/[\\/]/).at(-1) || demoPath
    const plan = this.sources.has(path)
      ? 'direct'
      : /\.mkv$/i.test(title)
        ? 'remux'
        : /\.avi$/i.test(title)
          ? 'transcode'
          : 'direct'
    const cached = this.cachedCues(path, durationMs) ?? []
    const sessionId = String(turn)
    const info: MediaOpened = {
      sessionId,
      sourcePath: path,
      title,
      mediaUrl: plan === 'direct' ? this.sources.get(path) || '/demo/demo.mp4' : null,
      durationMs,
      plan,
      language: {
        sessionId,
        mode: this.settings.sourceLanguage,
        detected: this.settings.sourceLanguage.startsWith('auto')
          ? 'ja'
          : this.settings.sourceLanguage,
        confidence: 0.96,
        locked: true,
        unverifiedTranslation: false,
      },
      target: this.settings.targetLanguage,
      audioTracks: [
        {
          index: 1,
          order: 0,
          codecName: 'aac',
          channels: 1,
          channelLayout: 'mono',
          sampleRate: 16000,
          languageTag: 'ja',
          default: true,
        },
      ],
      cues: cached,
    }
    if (this.sources.has(path)) {
      info.language = { ...info.language, detected: null, confidence: 0, locked: false }
      info.audioTracks = []
      this.visibleCues.clear()
    }
    this.current = info
    this.visibleCues = new Map(cached.map((c) => [c.id, c]))
    this.recent = [
      {
        path,
        title,
        fingerprint: title,
        durationMs: info.durationMs,
        lastOpened: Date.now(),
        progress: cached.length ? 1 : 0,
      },
      ...this.recent.filter((r) => r.path !== path),
    ].slice(0, 20)
    persist(
      'recent',
      this.recent.filter((r) => r.path === demoPath),
    )
    this.emit('media://opened', info)
    this.emit('pipeline://language', info.language)
    if (this.sources.has(path)) return
    if (plan !== 'direct')
      for (const progress of [0.2, 0.6, 1]) {
        await wait(400)
        if (turn !== this.generation) return
        this.emit('media://prep', {
          sessionId,
          plan,
          progress,
          mediaUrl: progress === 1 ? '/demo/demo.mp4' : null,
          error: null,
        })
      }
  }
  private async setup(ids: string[]) {
    const run = ++this.setupRun
    for (const c of this.components.filter((c) => ids.includes(c.id) && !c.ready)) {
      for (let i = 1; i <= 6; i++) {
        await wait(100)
        if (run !== this.setupRun) return
        c.bytesDone = Math.round((c.bytesTotal * i) / 6)
        c.state = i === 6 ? 'verifying' : 'downloading'
        this.emit('setup://progress', c)
      }
      c.ready = true
      c.state = 'ready'
      this.emit('setup://progress', c)
    }
  }
  async call<K extends keyof Commands>(command: K, args?: Commands[K][0]): Promise<Reply<K>> {
    // The switch implements the strongly typed desktop command map using browser fixtures.
    const a: Commands[keyof Commands][0] = args
    let reply: unknown
    switch (command) {
      case 'set_playhead':
        if (a && 'sessionId' in a && 'ms' in a && a.sessionId === this.current?.sessionId)
          this.playhead = a.ms
        break
      case 'get_settings':
        reply = { ...this.settings }
        break
      case 'update_settings':
        if (a && 'patch' in a) Object.assign(this.settings, a.patch)
        persist('settings', this.settings)
        reply = { ...this.settings }
        break
      case 'list_recent':
        reply = [...this.recent]
        break
      case 'get_sample_media':
        reply = demoPath
        break
      case 'list_chapters':
        reply = []
        break
      case 'close_media':
        this.generation++
        clearInterval(this.timer)
        this.current = null
        this.currentPath = ''
        this.visibleCues.clear()
        break
      case 'open_media':
        if (a && 'path' in a) await this.open(a.path)
        break
      case 'set_target_language':
      case 'set_source_language': {
        if (a && 'modeOrCode' in a) this.settings.sourceLanguage = a.modeOrCode
        if (a && 'code' in a && typeof a.code === 'string') this.settings.targetLanguage = a.code
        persist('settings', this.settings)
        if (this.current && !this.sources.has(this.currentPath)) {
          const s = this.current
          s.target = this.settings.targetLanguage
          s.language = {
            ...s.language,
            unverifiedTranslation: languages.some(
              (l) => l.code === this.settings.sourceLanguage && !l.target,
            ),
            mode: this.settings.sourceLanguage,
            detected: this.settings.sourceLanguage.startsWith('auto')
              ? 'ja'
              : this.settings.sourceLanguage,
          }
          this.emit('pipeline://language', s.language)
          const cached = this.cachedCues(this.currentPath)
          this.emit('pipeline://cues', {
            sessionId: s.sessionId,
            removedIds: [],
            upserts:
              cached ??
              [...this.visibleCues.values()].map((c) => ({
                ...c,
                translation: null,
                stale: command === 'set_source_language',
              })),
          })
          if (this.generating) this.stream(this.currentPath, cached)
        }
        break
      }
      case 'probe_url':
        if (a && 'url' in a) {
          const url = new URL(a.url)
          if (!['http:', 'https:'].includes(url.protocol)) throw Error('HTTP or HTTPS required')
          await wait(260)
          reply = {
            id: 'demo',
            title: 'Japanese sample',
            duration: 22.5,
            extractor: url.hostname,
            thumbnail: null,
            isLive: false,
          }
        }
        break
      case 'download_url': {
        const run = ++this.downloadRun
        for (let i = 1; i <= 12; i++) {
          await wait(160)
          if (run !== this.downloadRun) return undefined as Reply<K>
          this.emit('download://progress', {
            part: i > 6 ? 2 : 1,
            parts: 2,
            downloaded: ((i - 1) % 6) * 200000 + 200000,
            total: 1200000,
            eta: Math.ceil((12 - i) * 0.16),
          })
        }
        await this.open(demoPath)
        break
      }
      case 'cancel_download':
        this.downloadRun++
        break
      case 'get_setup_status':
        reply = structuredClone(this.components)
        break
      case 'start_setup':
        if (a && 'includeUrlTools' in a)
          await this.setup(
            a.ids ??
              this.components
                .filter(
                  (c) =>
                    !c.id.includes('0.6b') &&
                    (a.includeUrlTools || !['yt-dlp', 'deno'].includes(c.id)),
                )
                .map((c) => c.id),
          )
        break
      case 'cancel_setup':
        this.setupRun++
        for (const c of this.components.filter((c) => !c.ready)) {
          c.state = 'missing'
          this.emit('setup://progress', c)
        }
        break
      case 'update_ytdlp':
        await this.setup(['yt-dlp'])
        break
      case 'clear_cache':
        this.cueCache.clear()
        persist('samples', [])
        break
      case 'app_info':
        reply = { version: '0.1.0', logDir: 'logs', cacheBytes: 0 }
        break
      case 'suggest_export_paths':
        if (a && 'variant' in a) reply = this.filename(a.variant, a.format)
        break
      case 'export_subtitles':
        if (a && 'variant' in a && 'path' in a) this.exportSample(a.variant, a.format, a.path)
        break
      case 'list_subtitle_tracks': {
        if (!a || !('sessionId' in a) || a.sessionId !== this.current?.sessionId)
          throw Error('Media changed')
        if (this.currentPath === demoPath) {
          reply = {
            embedded: [],
            sidecars: ['demo.srt'],
            exactSidecar: 'demo.srt',
            probeFailed: false,
          }
          break
        }
        const sidecars = (this.companions.get(this.currentPath) ?? []).map((f) => f.name)
        const title = this.current.title.replace(/\.[^.]+$/, '')
        reply = {
          embedded: [],
          sidecars,
          exactSidecar: sidecars.find((n) => n.replace(/\.[^.]+$/, '') === title) ?? null,
          probeFailed: false,
        }
        break
      }
      case 'read_sidecar_subtitle': {
        if (!a || !('name' in a) || a.sessionId !== this.current?.sessionId)
          throw Error('Media changed')
        if (this.currentPath === demoPath && a.name === 'demo.srt') {
          const response = await fetch('/demo/demo.srt')
          if (!response.ok) throw Error('Sample subtitles unavailable')
          reply = await response.text()
          break
        }
        const file = this.companions.get(this.currentPath)?.find((f) => f.name === a.name)
        if (!file) throw Error('Subtitle file unavailable')
        reply = await readSubtitleFile(file, this.settings.uiLanguage)
        break
      }
      case 'read_embedded_subtitle':
        throw Error('Open this file in the desktop player for embedded tracks.')
      case 'start_transcription': {
        if (!a || !('sessionId' in a) || a.sessionId !== this.current?.sessionId)
          throw Error('Media changed')
        if (this.sources.has(this.currentPath))
          throw Error('Open this file in the desktop player to transcribe it.')
        const asr =
          this.settings.asrModel === 'qwen3-asr-0.6b' ? 'asr-qwen3-0.6b-mlx' : 'asr-qwen3-1.7b-mlx'
        const required = ['ffmpeg', asr, 'vad-silero']
        if (
          this.settings.targetLanguage !== 'none' &&
          this.settings.targetLanguage !== this.current.language.detected
        )
          required.push('mt-hy-mt2-1.8b', 'llama-runtime-b11429')
        const missing = required.filter(
          (key) => !this.components.some((c) => c.id === key && c.ready),
        )
        reply = { missing }
        if (!missing.length) {
          this.generating = true
          this.stream(this.currentPath, [...this.visibleCues.values()])
        }
        break
      }
      case 'retry_failed':
        if (this.current && this.generating && !this.sources.has(this.currentPath))
          this.stream(this.currentPath, [...this.visibleCues.values()])
        break
      case 'report_playback_error':
        if (this.current)
          this.emit('app://error', {
            sessionId: this.current.sessionId,
            code: 'media',
            message: 'This browser could not decode the media.',
            recoverable: true,
          })
        break
    }
    return reply as Reply<K>
  }
  private filename(variant: SubtitleVariant, format: SubtitleFormat) {
    const source = this.current?.language.detected || 'und',
      target = this.settings.targetLanguage
    return (
      (this.current?.title || 'sample').replace(/\.[^.]+$/, '') +
      '.' +
      (variant === 'original'
        ? source
        : variant === 'translation'
          ? target
          : source + '-' + target) +
      '.' +
      format
    )
  }
  private exportSample(variant: SubtitleVariant, format: SubtitleFormat, path: string) {
    const stamp = (ms: number, ass = false) =>
      new Date(ms)
        .toISOString()
        .slice(ass ? 12 : 11, ass ? 22 : 23)
        .replace('.', format === 'srt' ? ',' : '.')
    const blocks = [...this.visibleCues.values()]
      .sort((a, b) => a.startMs - b.startMs)
      .flatMap((c) => {
        const parts =
          variant === 'original'
            ? [c.original]
            : variant === 'translation'
              ? [c.translation]
              : variant.bilingual === 'translation-first'
                ? [c.translation, c.original]
                : [c.original, c.translation]
        const lines = parts.filter((t): t is string => !!t)
        return lines.length ? [{ cue: c, lines }] : []
      })
    const body =
      format === 'ass'
        ? '[Script Info]\nScriptType: v4.00+\nWrapStyle: 2\n[V4+ Styles]\nFormat: Name, Fontname, Fontsize, PrimaryColour, SecondaryColour, OutlineColour, BackColour, Bold, Italic, Underline, StrikeOut, ScaleX, ScaleY, Spacing, Angle, BorderStyle, Outline, Shadow, Alignment, MarginL, MarginR, MarginV, Encoding\nStyle: Default,Arial,32,&H00FFFFFF,&H00FFFFFF,&H00000000,&H00000000,0,0,0,0,100,100,0,0,1,2,0,2,20,20,20,1\n[Events]\nFormat: Layer, Start, End, Style, Name, MarginL, MarginR, MarginV, Effect, Text\n' +
          blocks
            .map(
              ({ cue: c, lines }) =>
                'Dialogue: 0,' +
                stamp(c.startMs, true) +
                ',' +
                stamp(c.endMs, true) +
                ',Default,,0,0,0,,' +
                lines.join('\\N'),
            )
            .join('\n')
        : (format === 'vtt' ? 'WEBVTT\n\n' : '') +
          blocks
            .map(
              ({ cue: c, lines }, i) =>
                (format === 'srt' ? i + 1 + '\n' : '') +
                stamp(c.startMs) +
                ' --> ' +
                stamp(c.endMs) +
                '\n' +
                lines.join('\n'),
            )
            .join('\n\n')
    const url = URL.createObjectURL(new Blob([body], { type: 'text/plain;charset=utf-8' })),
      a = document.createElement('a')
    a.href = url
    a.download = path.split(/[\\/]/).at(-1) || path
    a.click()
    setTimeout(() => URL.revokeObjectURL(url), 1000)
  }
}

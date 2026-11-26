import type { Backend, Commands, Events } from './backend'
import type {
  ComponentStatus,
  MediaOpened,
  RecentItem,
  Settings,
} from './types'
import { defaults } from './defaults'
import { loadMessages, resolveUiLanguage } from './i18n'
import { readSubtitleFile } from './subtitle-tracks'
import mediaExtensions from '../../../media-formats.json'
import { version } from '../../package.json'

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
  async version() { return version }
  private events = new EventTarget()
  private settings = { ...defaults(), ...stored<Partial<Settings>>('settings', {}) }
  // Browser file grants and their object URLs last for this page session only.
  private recent: RecentItem[] = []
  private sources = new Map<string, string>()
  private modifiedTimes = new Map<string, number>()
  private companions = new Map<string, File[]>()
  private current: MediaOpened | null = null
  private generation = 0
  private setupRun = 0
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
    this.modifiedTimes.set(path, file.lastModified)
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
    return new Promise((resolve, reject) => {
      const input = document.createElement('input')
      input.type = 'file'
      input.multiple = true
      input.accept = ['video/*', 'audio/*', ...mediaExtensions.map((ext) => '.' + ext)].join(',')
      if (directory && 'webkitdirectory' in input) input.webkitdirectory = true
      input.onchange = () => {
        const paths = this.mediaPaths(input.files, directory)
        if (directory && input.files?.length && !paths.length) reject(new Error('emptyMediaFolder'))
        else resolve(paths)
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
  private async duration(src: string) {
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
      timer = window.setTimeout(() => finish(0), 1500)
      video.preload = 'metadata'
      video.onloadedmetadata = () =>
        finish(Number.isFinite(video.duration) ? Math.round(video.duration * 1000) : 0)
      video.onerror = () => finish(0)
      video.src = src
    })
  }
  private async open(path: string) {
    const mediaUrl = this.sources.get(path)
    if (!mediaUrl) throw Error('browserUnsupported')
    const turn = ++this.generation
    const durationMs = await this.duration(mediaUrl)
    if (turn !== this.generation) return
    const title = path.split('/').at(-1) || path
    const sessionId = String(turn)
    const info: MediaOpened = {
      sessionId,
      sourcePath: path,
      title,
      mediaUrl,
      durationMs,
      plan: 'direct',
      language: {
        sessionId,
        mode: this.settings.sourceLanguage,
        detected: null,
        confidence: 0,
        locked: false,
        unverifiedTranslation: false,
      },
      target: this.settings.targetLanguage,
      audioTracks: [],
      cues: [],
    }
    this.current = info
    this.recent = [
      {
        path,
        title,
        fingerprint: title,
        durationMs: info.durationMs,
        lastOpened: Date.now(),
        progress: 0,
      },
      ...this.recent.filter((r) => r.path !== path),
    ].slice(0, 20)
    this.emit('media://opened', info)
    this.emit('pipeline://language', info.language)
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
    const a: Commands[keyof Commands][0] = args
    let reply: unknown
    switch (command) {
      case 'system_accent_color':
        reply = '#007aff'
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
      case 'search_online_subtitles':
      case 'download_online_subtitle':
        throw Error('desktop_only')
      case 'list_chapters':
        reply = []
        break
      case 'close_media':
        this.generation++
        this.current = null
        break
      case 'open_media':
        if (a && 'path' in a) await this.open(a.path)
        break
      case 'playlist_file_times':
        if (a && 'paths' in a) reply = a.paths.map(path => this.modifiedTimes.get(path) ?? null)
        break
      case 'list_media_siblings':
        // A browser file picker grants access to selected files only.
        reply = null
        break
      case 'set_target_language':
      case 'set_source_language': {
        if (a && 'modeOrCode' in a) this.settings.sourceLanguage = a.modeOrCode
        if (a && 'code' in a && typeof a.code === 'string') this.settings.targetLanguage = a.code
        persist('settings', this.settings)
        break
      }
      case 'probe_url':
      case 'download_url':
        throw Error('browserUnsupported')
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
      case 'app_info':
        reply = { version, logDir: 'logs', cacheBytes: 0 }
        break
      case 'list_app_releases':
        reply = []
        break
      case 'prepared_app_release':
        reply = null
        break
      case 'download_app_release':
      case 'install_app_release':
        throw Error('Application updates are available in the desktop app.')
      case 'suggest_export_paths':
      case 'export_subtitles':
      case 'start_transcription':
      case 'retry_failed':
      case 'delete_generated_subtitles': {
        const locale = resolveUiLanguage(this.settings.uiLanguage, navigator.languages)
        throw Error((await loadMessages(locale)).transcribeDesktop)
      }
      case 'list_subtitle_tracks': {
        if (!a || !('sessionId' in a) || a.sessionId !== this.current?.sessionId)
          throw Error('mediaChanged')
        const sidecars = (this.companions.get(this.current.sourcePath ?? '') ?? []).map((f) => f.name)
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
          throw Error('mediaChanged')
        const file = this.companions.get(this.current.sourcePath ?? '')?.find((f) => f.name === a.name)
        if (!file) throw Error('Subtitle file unavailable')
        reply = await readSubtitleFile(file)
        break
      }
      case 'read_embedded_subtitle':
        throw Error('Open this file in the desktop player for embedded tracks.')
      case 'report_playback_error':
        if (this.current)
          this.emit('app://error', {
            sessionId: this.current.sessionId,
            code: 'media',
            message: 'browserUnsupported',
            recoverable: true,
          })
        break
    }
    return reply as Reply<K>
  }
}

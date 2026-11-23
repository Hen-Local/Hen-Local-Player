import { invoke } from '@tauri-apps/api/core'
import { backend, cueAt, mockMode, player } from './stores.svelte'
import {
  parseSubtitleText,
  indexTimeline,
  visibleText,
  type SubtitleTimeline,
  type TimedText,
} from './subtitle-tracks'
import {
  getPreference,
  putPreference,
  importedText,
  type TrackPreference,
  type FileEntry,
} from './subtitle-tracks-history'
import type { Cue, Settings } from './types'

export interface EmbeddedTrack {
  order: number
  default: boolean
  forced: boolean
  language: string | null
  title: string | null
}
interface SubtitleInventory {
  embedded: EmbeddedTrack[]
  sidecars: string[]
  exactSidecar: string | null
  probeFailed: boolean
}
function generatedLines(cue: Cue | undefined, settings: Settings): string[] {
  if (!cue || settings.subtitleMode === 'off') return []
  if (settings.subtitleMode === 'original') return [cue.original]
  if (settings.subtitleMode === 'translation') return cue.translation ? [cue.translation] : []
  const pair =
    settings.bilingualOrder === 'translation-first'
      ? [cue.translation, cue.original]
      : [cue.original, cue.translation]
  return pair.filter((line): line is string => !!line)
}

class SubtitleTracks {
  session = ''
  selected = $state('generated')
  delayMs = $state(0)
  embedded = $state.raw<EmbeddedTrack[]>([])
  sidecars = $state.raw<string[]>([])
  files = $state.raw<FileEntry[]>([])
  timeline = $state.raw<SubtitleTimeline | null>(null)
  authored = $derived.by(() => {
    if (!this.timeline || !this.delayMs) return this.timeline
    return indexTimeline(
      this.timeline.cues
        .map((cue) => ({
          startMs: Math.max(0, cue.startMs + this.delayMs),
          endMs: cue.endMs + this.delayMs,
          text: cue.text,
        }))
        .filter((cue) => cue.endMs > cue.startMs),
    )
  })
  loading = $state(false)
  discoveryError = $state(false)
  private path = ''
  private pref: TrackPreference = {}
  private probed = false
  private initial = true
  private request = 0
  private extracted = new Map<string, SubtitleTimeline>()

  attach(id: string) {
    if (id === this.session) return
    this.session = id
    this.path = player.media?.sourcePath || player.currentPath
    this.pref = getPreference(this.path)
    this.selected = this.pref.choice === 'off' ? 'off' : 'generated'
    this.delayMs = this.pref.delayMs ?? 0
    this.files = this.pref.files ?? []
    this.timeline = null
    this.embedded = []
    this.sidecars = []
    this.extracted.clear()
    this.probed = false
    this.initial = true
    this.loading = false
    this.discoveryError = false
    ++this.request
  }

  private current(id: string) {
    return !!id && this.session === id && player.media?.sessionId === id
  }

  async discover(id: string) {
    this.attach(id)
    if (!id || this.probed) return
    const revision = this.request
    this.probed = true
    this.discoveryError = false
    try {
      const result = mockMode
        ? { embedded: [], sidecars: [], exactSidecar: null, probeFailed: false }
        : await invoke<SubtitleInventory>('list_subtitle_tracks', { sessionId: id })
      if (!this.current(id)) return
      this.embedded = result.embedded
      this.sidecars = result.sidecars
      this.probed = !result.probeFailed
      this.discoveryError = result.probeFailed
      if (!this.initial) return
      this.initial = false
      let key = this.pref.choice
      if (!key && player.settings.subtitleMode !== 'off') {
        const embedded = this.embedded.find((t) => t.forced) ?? this.embedded.find((t) => t.default)
        if (embedded) key = 'embedded:' + embedded.order
        else if (result.exactSidecar) {
          const components = player.components.length
            ? player.components
            : await backend.call('get_setup_status')
          const asr =
            player.settings.asrModel === 'sherpa-qwen3-asr-0.6b'
              ? 'asr-qwen3-0.6b-sherpa'
              : player.settings.asrModel === 'qwen3-asr-0.6b'
                ? 'asr-qwen3-0.6b-mlx'
                : 'asr-qwen3-1.7b-mlx'
          const needed = [asr, 'vad-silero']
          if (player.settings.targetLanguage !== 'none')
            needed.push('mt-hy-mt2-1.8b', 'llama-runtime-b11429')
          if (needed.some((id) => !components.some((c) => c.id === id && c.ready)))
            key = 'sidecar:' + result.exactSidecar
        }
      }
      if (key && revision === this.request && this.current(id)) await this.select(key, false)
    } catch {
      if (this.current(id)) {
        this.probed = false
        this.discoveryError = true
      }
    }
  }

  private save(patch: TrackPreference) {
    this.pref = { ...this.pref, ...patch }
    putPreference(this.path, this.pref)
  }
  setDelay(ms: number) {
    if (!Number.isSafeInteger(ms)) return
    this.delayMs = ms
    this.save({ delayMs: ms })
  }

  async select(key: string, explicit = true) {
    const id = this.session
    if (!this.current(id)) return
    const revision = ++this.request
    if (explicit) this.initial = false
    this.loading = false
    let timeline = this.extracted.get(key) ?? null
    const last = this.selected === 'off' ? this.pref.last : this.selected
    if (!timeline && key !== 'generated' && key !== 'off') {
      const item = key.slice(key.indexOf(':') + 1)
      const order = Number(item)
      let read: Promise<string | undefined>
      if (key.startsWith('embedded:') && this.embedded.some((t) => t.order === order)) {
        read = invoke<string>('read_embedded_subtitle', { sessionId: id, order })
      } else if (key.startsWith('sidecar:') && this.sidecars.includes(item)) {
        read = invoke<string>('read_sidecar_subtitle', { sessionId: id, name: item })
      } else if (this.files.some((file) => file.id === key)) {
        read = importedText(this.path, key)
      } else return
      this.loading = true
      try {
        const text = await read
        if (revision !== this.request || !this.current(id)) return
        if (!text)
          throw Error(
            player.locale === 'zh' ? '请重新打开字幕文件。' : 'Open this subtitle file again.',
          )
        timeline = parseSubtitleText(text)
        if (!timeline.cues.length)
          throw Error(
            player.locale === 'zh'
              ? '轨道中没有可读的字幕。'
              : 'This track contains no readable cues.',
          )
        this.extracted.set(key, timeline)
      } catch (e) {
        if (revision === this.request && this.current(id) && explicit) throw e
        return
      } finally {
        if (revision === this.request) this.loading = false
      }
    }
    this.selected = key
    this.timeline = timeline
    if (explicit) {
      this.save({ choice: key, last: key === 'off' ? last : key })
      if (key === 'generated' && player.settings.subtitleMode === 'off')
        await player.update({ subtitleMode: 'both' })
    }
  }

  async setMode(mode: string) {
    if (mode === 'off') return this.select('off')
    await this.select('generated')
    await player.update({ subtitleMode: mode })
  }

  async cycle() {
    if (!player.media) return
    if (this.selected === 'off') {
      if (this.pref.last && this.pref.last !== 'generated') return this.select(this.pref.last)
      return this.setMode('original')
    }
    if (this.selected !== 'generated') return this.select('off')
    const modes = ['original', 'translation', 'both', 'off']
    return this.setMode(modes[(modes.indexOf(player.settings.subtitleMode) + 1) % 4])
  }

  async importFile(browserFile?: File) {
    const id = this.session
    if (!this.current(id)) return
    this.initial = false
    const revision = ++this.request
    this.loading = false
    let file: { name: string; text: string } | null
    if (browserFile) {
      if (browserFile.size > 16 * 1024 * 1024)
        throw Error(player.locale === 'zh' ? '字幕文件超过 16 MiB' : 'Subtitle file exceeds 16 MiB')
      const bytes = new Uint8Array(await browserFile.arrayBuffer())
      const encoding =
        bytes[0] === 0xff && bytes[1] === 0xfe
          ? 'utf-16le'
          : bytes[0] === 0xfe && bytes[1] === 0xff
            ? 'utf-16be'
            : 'utf-8'
      file = {
        name: browserFile.name,
        text: new TextDecoder(encoding, { fatal: true }).decode(bytes),
      }
    } else {
      file = await invoke<{ name: string; text: string } | null>('choose_subtitle_file')
    }
    if (!file || !this.current(id)) return
    const timeline = parseSubtitleText(file.text)
    if (!timeline.cues.length)
      throw Error(
        player.locale === 'zh'
          ? '文件中没有有效的 SRT 或 WebVTT 字幕。'
          : 'This file contains no readable SRT or WebVTT cues.',
      )
    const track = { id: 'file:' + (this.files.length + 1), name: file.name }
    this.extracted.set(track.id, timeline)
    this.files = [...this.files, track]
    try {
      await importedText(this.path, track.id, file.text)
      if (!this.current(id)) return
      this.save({ files: this.files })
    } catch {
      /* Still usable in this session if device storage is exhausted. */
    }
    if (this.current(id) && revision === this.request) await this.select(track.id)
  }

  linesAt(ms: number, cues: Cue[], settings: Settings) {
    if (this.selected === 'off') return []
    const at = ms - this.delayMs
    if (this.authored) return visibleText(this.authored, ms)
    return generatedLines(cues[cueAt(cues, at)], settings)
  }

  pipCues(cues: Cue[], settings: Settings): TimedText[] {
    if (this.selected === 'off') return []
    if (this.authored) return this.authored.cues
    const rows = cues.map((cue) => ({
      startMs: cue.startMs,
      endMs: cue.endMs,
      text: generatedLines(cue, settings).join('\n'),
    }))
    return rows
      .map((cue) => ({
        startMs: Math.max(0, cue.startMs + this.delayMs),
        endMs: cue.endMs + this.delayMs,
        text: cue.text,
      }))
      .filter((cue) => cue.text && cue.endMs > cue.startMs)
  }
}
export const subtitleTracks = new SubtitleTracks()

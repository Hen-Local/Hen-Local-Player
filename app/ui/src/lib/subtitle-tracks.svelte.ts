import { invoke } from '@tauri-apps/api/core'
import { backend, player } from './stores.svelte'
import {
  readSubtitleFile,
  parseSubtitleText,
  indexTimeline,
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
import type { Cue, LanguageEvent, Settings } from './types'
import type { EmbeddedTrack } from './backend'

export function displaySourceLanguage(settings: Settings, language: LanguageEvent | null) {
  if (settings.sourceLanguage === 'auto-per-line') return null
  if (settings.sourceLanguage !== 'auto') return settings.sourceLanguage
  return language?.mode === 'mixed' ? null : language?.detected ?? null
}

export function generatedLines(
  cue: (Pick<Cue, 'original' | 'translation'> & { lang?: string | null }) | undefined,
  settings: Settings,
  reserveRows = false,
): string[] {
  if (!cue || settings.subtitleMode === 'off') return []
  if (
    settings.subtitleMode === 'original' ||
    settings.targetLanguage === 'none' ||
    settings.targetLanguage === (cue.lang ?? displaySourceLanguage(settings, player.language))
  )
    return [cue.original]
  if (settings.subtitleMode === 'translation') return cue.translation ? [cue.translation] : []
  const pair =
    settings.bilingualOrder === 'translation-first'
      ? [cue.translation, cue.original]
      : [cue.original, cue.translation]
  // Keep the original on its chosen row while its translation is arriving.
  return reserveRows ? pair.map((line) => line ?? '') : pair.filter((line): line is string => !!line)
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
  discovering = $state(false)
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
    this.discovering = false
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
    this.discovering = true
    this.discoveryError = false
    try {
      const result = await backend.call('list_subtitle_tracks', { sessionId: id })
      if (!this.current(id)) return
      this.embedded = result.embedded
      this.sidecars = result.sidecars
      this.probed = !result.probeFailed
      this.discoveryError = result.probeFailed
      if (!this.initial) return
      this.initial = false
      if (revision !== this.request) return
      if (this.pref.choice) {
        const key = this.pref.choice
        await this.select(key, false)
        if (!this.current(id) || this.request !== revision + 1 || this.selected === key) return
      }
      if (player.settings.subtitleMode === 'off') return
      const embeddedKey = (t: EmbeddedTrack) => 'embedded:' + t.order
      const options = [
        ...this.embedded.filter((t) => t.forced).map(embeddedKey),
        ...this.embedded.filter((t) => t.default).map(embeddedKey),
        ...(result.exactSidecar ? ['sidecar:' + result.exactSidecar] : []),
        ...this.embedded.map(embeddedKey),
        ...this.sidecars.map((name) => 'sidecar:' + name),
      ]
      for (const key of new Set(options)) {
        const expected = this.request + 1
        await this.select(key, false)
        if (!this.current(id) || this.request !== expected || this.timeline) return
      }
    } catch {
      if (this.current(id)) {
        this.probed = false
        this.discoveryError = true
      }
    } finally {
      if (this.current(id)) this.discovering = false
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
        read = backend.call('read_embedded_subtitle', { sessionId: id, order })
      } else if (key.startsWith('sidecar:') && this.sidecars.includes(item)) {
        read = backend.call('read_sidecar_subtitle', { sessionId: id, name: item })
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
    return this.importText(async () => browserFile
      ? { name: browserFile.name, text: await readSubtitleFile(browserFile, player.locale) }
      : invoke<{ name: string; text: string } | null>('choose_subtitle_file'))
  }

  async importOnline(fileId: string, read: () => Promise<{ name: string; text: string } | null>) {
    const key = 'file:' + fileId
    if (this.files.some((track) => track.id === key)) {
      const id = this.session
      const expected = this.request + 1
      try {
        await this.select(key)
        return
      } catch {
        // The browser can evict imported text independently of track preferences.
        if (!this.current(id) || expected !== this.request) return
      }
    }
    return this.importText(read, key)
  }

  private async importText(read: () => Promise<{ name: string; text: string } | null>, key?: string) {
    const id = this.session
    if (!this.current(id)) return
    this.initial = false
    const revision = ++this.request
    this.loading = false
    const file = await read()
    if (!file || !this.current(id)) return
    const timeline = parseSubtitleText(file.text)
    if (!timeline.cues.length)
      throw Error(
        key ? 'invalid_subtitle' : player.locale === 'zh'
          ? '文件中没有有效的 SRT 或 WebVTT 字幕。'
          : 'This file contains no readable SRT or WebVTT cues.',
      )
    const track = { id: key ?? 'file:' + (this.files.length + 1), name: file.name }
    this.extracted.set(track.id, timeline)
    this.files = [...this.files.filter((entry) => entry.id !== track.id), track]
    try {
      await importedText(this.path, track.id, file.text)
      if (!this.current(id)) return
      this.save({ files: this.files })
    } catch {
      if (this.current(id)) player.toast(player.locale === 'zh'
        ? '已载入字幕。本机存储空间不足或不可用，下次打开时需重新导入。'
        : 'Subtitles loaded. Local storage is unavailable, so you’ll need to import them again next time.')
    }
    if (this.current(id) && revision === this.request) await this.select(track.id)
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

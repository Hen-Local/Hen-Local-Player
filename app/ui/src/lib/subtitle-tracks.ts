// SubRip and WebVTT share the same timed cue structure. Render cue payloads as plain text.
export interface TimedText {
  startMs: number
  endMs: number
  text: string
}

export interface SubtitleTimeline {
  cues: TimedText[]
  latestEnd: number[]
}

const timestamp = /^(?:(\d+):)?(\d{1,2}):(\d{2})[.,](\d{1,3})$/
function millis(value: string) {
  const match = timestamp.exec(value)
  if (!match || +match[3] > 59 || (match[1] !== undefined && +match[2] > 59)) return NaN
  return ((+(match[1] || 0) * 60 + +match[2]) * 60 + +match[3]) * 1000 + +match[4].padEnd(3, '0')
}

const entities: Record<string, string> = {
  amp: '&',
  lt: '<',
  gt: '>',
  nbsp: '\u00a0',
  quot: '"',
  apos: "'",
  lrm: '\u200e',
  rlm: '\u200f',
}

function plainText(text: string): string {
  return text
    .replace(/<br\s*\/?\s*>/gi, '\n')
    .replace(/<\/?(?:c(?:\.[^\s>]+)*|i|b|u|ruby|rt|v|lang|font)(?:\s[^>]*)?>|<\d[\d:.]*>/gi, '')
    .replace(/\{\\[^{}]*\}/g, '')
    .replace(/&(amp|lt|gt|nbsp|quot|apos|lrm|rlm|#\d+|#x[\da-f]+);/gi, (entity, value: string) => {
      if (value[0] !== '#') return entities[value.toLowerCase()] ?? entity
      const code = value[1].toLowerCase() === 'x' ? parseInt(value.slice(2), 16) : +value.slice(1)
      return code > 0 && code <= 0x10ffff && (code < 0xd800 || code > 0xdfff)
        ? String.fromCodePoint(code)
        : entity
    })
    .trim()
}

export function parseSubtitleText(input: string): SubtitleTimeline {
  const blocks = input
    .replace(/^\uFEFF/, '')
    .replace(/\r\n?/g, '\n')
    .split(/\n[\t ]*\n/)
  const cues: TimedText[] = []
  for (const block of blocks) {
    const lines = block.trim().split('\n')
    if (/^(WEBVTT(?:\s|$)|NOTE(?:\s|$)|STYLE$|REGION$)/.test(lines[0])) continue
    const index = lines[0].includes('-->') ? 0 : 1
    const times = /^\s*(\S+)\s*-->\s*(\S+)(?:\s.*)?$/.exec(lines[index] || '')
    if (!times) continue
    const startMs = millis(times[1]),
      endMs = millis(times[2])
    if (!Number.isSafeInteger(startMs) || !Number.isSafeInteger(endMs) || endMs <= startMs) continue
    const text = plainText(lines.slice(index + 1).join('\n'))
    if (text) cues.push({ startMs, endMs, text })
  }
  cues.sort((a, b) => a.startMs - b.startMs)
  return indexTimeline(cues)
}

export function indexTimeline(cues: TimedText[]): SubtitleTimeline {
  let end = 0
  return { cues, latestEnd: cues.map((cue) => (end = Math.max(end, cue.endMs))) }
}

export function lastStartedCue(cues: readonly { startMs: number }[], ms: number): number {
  let low = 0,
    high = cues.length
  while (low < high) {
    const mid = (low + high) >>> 1
    if (cues[mid].startMs <= ms) low = mid + 1
    else high = mid
  }
  return low - 1
}

type AuthoredFrame = { from: number; until: number; indices: number[] }
const frames = new WeakMap<SubtitleTimeline, AuthoredFrame>()

export function activeAuthoredFrame(timeline: SubtitleTimeline, ms: number): AuthoredFrame {
  const previous = frames.get(timeline)
  if (previous && ms >= previous.from && ms < previous.until) return previous
  const { cues, latestEnd } = timeline
  const indices: number[] = []
  const last = lastStartedCue(cues, ms)
  let until = cues[last + 1]?.startMs ?? Infinity
  // A long caption can span several shorter ones. The prefix ends let us stop
  // at the first point before which no cue can still be on screen.
  for (let i = last; i >= 0 && latestEnd[i] > ms; i--) {
    if (cues[i].endMs > ms) {
      indices.push(i)
      until = Math.min(until, cues[i].endMs)
    }
  }
  const frame = { from: ms, until, indices: indices.reverse() }
  frames.set(timeline, frame)
  return frame
}

export function visibleIndices(timeline: SubtitleTimeline, ms: number): number[] {
  return activeAuthoredFrame(timeline, ms).indices
}

export async function readSubtitleFile(file: File): Promise<string> {
  if (file.size > 16 * 1024 * 1024)
    throw Error('subtitleTooLarge')
  const bytes = new Uint8Array(await file.arrayBuffer())
  const encoding = bytes[0] === 0xff && bytes[1] === 0xfe ? 'utf-16le'
    : bytes[0] === 0xfe && bytes[1] === 0xff ? 'utf-16be' : 'utf-8'
  try {
    return new TextDecoder(encoding, { fatal: true }).decode(bytes)
  } catch {
    throw Error('invalidSubtitleEncoding')
  }
}

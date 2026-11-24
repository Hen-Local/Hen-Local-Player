// These pages are only needed when a generated cue cannot physically fit the
// current picture at the selected text size. Stored subtitle text stays intact.
const words = new Intl.Segmenter(undefined, { granularity: 'word' })
const graphemes = new Intl.Segmenter(undefined, { granularity: 'grapheme' })

export type RowMetrics = { width: number; font: string; spacing: string }

let context: CanvasRenderingContext2D | null = null

export function captionPages(line: string, metrics: RowMetrics): string[] {
  context ??= document.createElement('canvas').getContext('2d')
  if (!context || !line || metrics.width <= 0) return [line]
  const canvas = context
  canvas.font = metrics.font
  if ('letterSpacing' in canvas) canvas.letterSpacing = metrics.spacing
  const width = (s: string) => canvas.measureText(s.trim()).width
  if (width(line) <= metrics.width) return [line]

  // Include spaces and punctuation in the source slices. Word segmentation
  // keeps unspaced Chinese and Japanese words intact. Grapheme boundaries are
  // only needed inside a single word wider than the entire row (e.g. a URL).
  const starts = [0]
  for (const part of words.segment(line)) {
    if (part.index > 0 && (part.isWordLike || !/^[\s\p{P}]+$/u.test(part.segment)))
      starts.push(part.index)
  }
  starts.push(line.length)
  const ends: number[] = []
  for (let i = 1; i < starts.length; i++) {
    const from = starts[i - 1], to = starts[i]
    const word = line.slice(from, to)
    if (width(word) > metrics.width) {
      for (const part of graphemes.segment(word)) ends.push(from + part.index + part.segment.length)
    } else ends.push(to)
  }

  // Find the minimum page count, then choose balanced widths so the last
  // page does not flash a lone word. All adjustments use the same hard limit.
  function wrap(limit: number) {
    const pages: string[] = []
    let start = 0, last = 0
    for (const end of ends) {
      if (last > start && width(line.slice(start, end)) > limit) {
        pages.push(line.slice(start, last))
        start = last
      }
      last = end
    }
    pages.push(line.slice(start))
    return pages
  }
  let pages = wrap(metrics.width)
  let low = Math.min(metrics.width, width(line) / pages.length), high = metrics.width
  for (let i = 0; i < 5 && high - low > 1; i++) {
    const mid = (low + high) / 2
    const candidate = wrap(mid)
    if (candidate.length > pages.length) low = mid
    else { high = mid; pages = candidate }
  }
  return pages.map(page => page.trim())
}

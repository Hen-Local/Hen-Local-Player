export interface QueueItem {
  path: string
  title: string
}
export type PlaylistSort = 'original' | 'nameAsc' | 'nameDesc' | 'newest' | 'oldest'

export function mediaKind(filename: string): 'audio' | 'video' {
  return /\.(aac|ac3|aif[fc]?|alac|flac|m4a|mka|mp3|oga|ogg|opus|wav|wma)$/i.test(filename)
    ? 'audio' : 'video'
}

export function sortPlaylist(
  items: QueueItem[], order: PlaylistSort, modified: ReadonlyMap<QueueItem, number | null>, locale: string,
): QueueItem[] {
  if (order === 'original') return items
  if (order === 'nameAsc' || order === 'nameDesc') {
    const byName = new Intl.Collator(locale, { numeric: true, sensitivity: 'base' })
    const direction = order === 'nameAsc' ? 1 : -1
    return [...items].sort((a, b) => direction * byName.compare(a.title, b.title))
  }
  return [...items].sort((a, b) => {
    const left = modified.get(a), right = modified.get(b)
    if (left == null) return right == null ? 0 : 1
    if (right == null) return -1
    return order === 'newest' ? right - left : left - right
  })
}

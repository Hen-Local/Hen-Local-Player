export function clock(ms: number) {
  const seconds = Math.max(0, Math.floor(ms / 1000))
  const hh = Math.floor(seconds / 3600),
    mm = Math.floor(seconds / 60) % 60,
    ss = seconds % 60
  return (
    (hh ? hh + ':' + String(mm).padStart(2, '0') : String(mm).padStart(2, '0')) +
    ':' +
    String(ss).padStart(2, '0')
  )
}
export function size(bytes: number) {
  return bytes >= 1e9
    ? (bytes / 1e9).toFixed(2) + ' GB'
    : bytes >= 1e6
      ? (bytes / 1e6).toFixed(1) + ' MB'
      : Math.ceil(bytes / 1e3) + ' KB'
}
export const capMimes = [
  'video/mp4; codecs="avc1.640028"',
  'video/mp4; codecs="hvc1.1.6.L120.90"',
  'video/mp4; codecs="av01.0.08M.08"',
  'video/mp4; codecs="vp09.00.40.08"',
  'video/webm; codecs="vp9"',
  'video/webm; codecs="vp8"',
  'video/webm; codecs="av01.0.08M.08"',
  'audio/mp4; codecs="mp4a.40.2"',
  'audio/mp4; codecs="opus"',
  'audio/mp4; codecs="flac"',
  'audio/mpeg',
  'audio/mp4; codecs="ac-3"',
  'audio/mp4; codecs="ec-3"',
  'audio/webm; codecs="opus"',
  'audio/webm; codecs="vorbis"',
]

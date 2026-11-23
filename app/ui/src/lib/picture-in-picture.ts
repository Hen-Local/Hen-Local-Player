type WebKitVideo = HTMLVideoElement & {
  webkitPresentationMode?: string
  webkitSetPresentationMode?: (mode: 'inline' | 'picture-in-picture') => void
  webkitSupportsPresentationMode?: (mode: 'picture-in-picture') => boolean
}

export function supportsPiP(video: WebKitVideo) {
  return (
    !video.disablePictureInPicture &&
    (Boolean(document.pictureInPictureEnabled && video.requestPictureInPicture) ||
      Boolean(video.webkitSupportsPresentationMode?.('picture-in-picture')))
  )
}

export function inPiP(video: WebKitVideo) {
  return (
    document.pictureInPictureElement === video ||
    video.webkitPresentationMode === 'picture-in-picture'
  )
}

export async function togglePiP(video: WebKitVideo) {
  if (document.pictureInPictureEnabled && video.requestPictureInPicture) {
    if (document.pictureInPictureElement === video) await document.exitPictureInPicture()
    else await video.requestPictureInPicture()
  } else video.webkitSetPresentationMode?.(inPiP(video) ? 'inline' : 'picture-in-picture')
}

// Use a native timed track for PiP, where the app window's HTML overlay is absent.
export function updatePiPCaptions(
  track: TextTrack,
  cues: { startMs: number; endMs: number; text: string }[],
) {
  const previous = new Map(Array.from(track.cues ?? [], (cue) => [cue.id, cue]))
  for (const { startMs, endMs, text } of cues) {
    if (!text || startMs >= endMs) continue
    const id = JSON.stringify([startMs, endMs, text])
    if (!previous.delete(id)) {
      const cue = new VTTCue(
        startMs / 1000,
        endMs / 1000,
        text.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;'),
      )
      cue.id = id
      track.addCue(cue)
    }
  }
  for (const cue of previous.values()) track.removeCue(cue)
}

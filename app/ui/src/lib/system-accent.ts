import type { Backend } from './backend'

function luminance(rgb: number[]) {
  const linear = rgb.map(channel => {
    const value = channel / 255
    return value <= 0.04045 ? value / 12.92 : ((value + 0.055) / 1.055) ** 2.4
  })
  return 0.2126 * linear[0] + 0.7152 * linear[1] + 0.0722 * linear[2]
}

// Lift dark accents just enough for labels and small glyphs on the gray glass.
const glassLuminance = luminance([81, 83, 90])
function glassInk(rgb: number[], minimumContrast: number) {
  let ink = rgb
  for (let step = 1; (luminance(ink) + 0.05) / (glassLuminance + 0.05) < minimumContrast; step++) {
    ink = rgb.map(channel => Math.round(channel + (255 - channel) * step / 20))
  }
  return `rgb(${ink.join(' ')})`
}

function applyAccent(hex: string | null) {
  if (!hex || !/^#[0-9a-f]{6}$/i.test(hex)) return
  const rgb = [1, 3, 5].map(offset => parseInt(hex.slice(offset, offset + 2), 16))
  const style = document.documentElement.style
  style.setProperty('--system-accent', hex)
  style.setProperty('--system-glass-accent', glassInk(rgb, 4.5))
  style.setProperty('--system-indicator-accent', glassInk(rgb, 3))
  style.setProperty('--system-on-solid', luminance(rgb) > 0.179 ? '#000' : '#fff')
}

export function observeSystemAccent(backend: Backend) {
  let stopped = false
  let receivedChange = false
  const subscription = backend.listen('system-accent-changed', color => {
    if (stopped) return
    receivedChange = true
    applyAccent(color)
  })
  // Subscribe first so a slow initial read cannot overwrite a live change.
  void subscription.catch(error => console.warn('System color notifications unavailable', error))
    .then(async () => {
      if (stopped || receivedChange) return
      const color = await backend.call('system_accent_color')
      if (!stopped && !receivedChange) applyAccent(color)
    }).catch(error => console.warn('System accent unavailable', error))
  return () => {
    stopped = true
    void subscription.then(unlisten => unlisten(), () => {})
  }
}

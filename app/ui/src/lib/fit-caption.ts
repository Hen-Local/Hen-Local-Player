// Scale the painted text when it exceeds its row. The transform leaves the
// measured layout widths intact, so observing it never triggers a resize loop.
export function fitCaption(row: HTMLElement) {
  const label = row.querySelector('span')!
  function fit() {
    const scale = Math.min(1, row.clientWidth / Math.max(1, label.offsetWidth))
    label.style.transform = scale < 1 ? `scale(${scale})` : ''
  }
  const observer = new ResizeObserver(fit)
  observer.observe(row)
  observer.observe(label)
  fit()
  return { destroy: () => observer.disconnect() }
}

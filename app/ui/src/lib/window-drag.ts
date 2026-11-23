// Native dragging takes over the mouse until release. Wait for deliberate motion
// so the movie can still distinguish a single click from a double-click.
export function enableWindowDragging(shell: HTMLElement, canDrag: () => boolean, startDrag: () => void) {
  const listeners = new AbortController()
  const options = { capture: true, signal: listeners.signal }
  let down: PointerEvent | null = null
  let suppressClick = false

  shell.addEventListener('pointerdown', (event) => {
    down = null
    suppressClick = false
    if (!canDrag() || !event.isPrimary || event.pointerType !== 'mouse' || event.button !== 0) return
    const target = event.target
    if (!(target instanceof Element) || target.closest(
      'button, a, input, textarea, select, label, summary, [contenteditable], [data-transport], ' +
      '[data-tauri-drag-region], [role="menu"], dialog, [draggable="true"]',
    )) return
    // Sidebar text may be selected; blank space and the panel heading may move the window.
    if (target.closest('.inspector') && !target.closest('.panel-title')) {
      if (Array.from(target.childNodes).some(node => node.nodeType === Node.TEXT_NODE && node.textContent?.trim())) return
      if (target instanceof HTMLElement && target.scrollHeight > target.clientHeight &&
          event.clientX >= target.getBoundingClientRect().right - 16) return
    }
    down = event
  }, options)

  window.addEventListener('pointermove', (event) => {
    if (!down || event.pointerId !== down.pointerId) return
    if (!(event.buttons & 1) || !canDrag()) { down = null; return }
    if (Math.hypot(event.screenX - down.screenX, event.screenY - down.screenY) < 5) return
    down = null
    suppressClick = true
    startDrag()
  }, options)
  const release = () => { down = null }
  window.addEventListener('pointerup', release, options)
  window.addEventListener('pointercancel', release, options)
  window.addEventListener('blur', release, { signal: listeners.signal })
  shell.addEventListener('click', (event) => {
    if (suppressClick && event.detail > 0) {
      event.preventDefault()
      event.stopImmediatePropagation()
    }
  }, options)
  return () => listeners.abort()
}

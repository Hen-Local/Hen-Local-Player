// Keep a closing drawer mounted until its shared CSS slide finishes.
export function finishPanelSlide(node: Element) {
  return { duration: parseFloat(getComputedStyle(node).getPropertyValue('--drawer-slide-ms')) }
}

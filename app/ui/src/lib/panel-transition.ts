import { fly } from 'svelte/transition'

export function slidePanel(node: Element, side: 'left' | 'right') {
  return fly(node, {
    x: side === 'left' ? '-100%' : '100%',
    opacity: 1,
    duration: matchMedia('(prefers-reduced-motion: reduce)').matches ? 0 : 180,
  })
}

<script lang="ts">
  import type { PipelineStatus } from './types'

  let { status, duration }: { status: PipelineStatus; duration: number } = $props()
  let canvas = $state<HTMLCanvasElement>()
  let width = $state(0)

  $effect(() => {
    const ctx = canvas?.getContext('2d')
    if (!ctx || !width || !duration) return
    const pixelsPerMs = width / duration
    ctx.clearRect(0, 0, width, 2)
    ctx.fillStyle = '#a8bbbb'
    ctx.fillRect(0, 0, status.vadComplete ? width : status.extractedUntilMs * pixelsPerMs, 2)

    // Combine pending spans by screen pixel. Long recordings still draw a
    // single strip, even with thousands of separate speech intervals.
    const draw = (spans: [number, number][], y: number, color: string) => {
      const boundaries = new Int32Array(width + 1)
      for (const [start, end] of spans) {
        const left = Math.max(0, Math.min(width, Math.floor(start * pixelsPerMs)))
        const right = Math.max(0, Math.min(width, Math.ceil(end * pixelsPerMs)))
        boundaries[left]++
        boundaries[right]--
      }
      ctx.fillStyle = color
      let active = 0, first = 0
      for (let x = 0; x <= width; x++) {
        const next = active + boundaries[x]
        if (!active && next) first = x
        if (active && !next) ctx.fillRect(first, y, x - first, 1)
        active = next
      }
    }
    draw(status.asrPendingSpans, 0, '#d5b486')
    draw(status.trPendingSpans, 1, '#c0bcee')
  })
</script>

<canvas bind:this={canvas} bind:clientWidth={width} {width} height="2" aria-hidden="true"></canvas>

<style>
  canvas {
    position: absolute;
    top: calc(50% + 6px);
    left: 0;
    width: 100%;
    height: 2px;
    pointer-events: none;
  }
</style>

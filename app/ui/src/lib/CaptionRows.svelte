<script lang="ts">
  import { captionPages, type RowMetrics } from './caption-pages'
  let { lines, progress, authored = false, audio = false }: {
    lines: string[]; progress: number; authored?: boolean; audio?: boolean
  } = $props()

  let metrics = $state<(RowMetrics | undefined)[]>([])
  function measure(row: HTMLElement, index: number) {
    function update() {
      const css = getComputedStyle(row)
      // Leave room for the label padding (.1em on either side), text stroke
      // and fractional WebKit layout rounding.
      const width = row.clientWidth - parseFloat(css.fontSize) * .32 - 2
      const font = css.fontWeight + ' ' + css.fontSize + ' ' + css.fontFamily
      const spacing = css.letterSpacing === 'normal' ? '0px' : css.letterSpacing
      const old = metrics[index]
      if (!old || old.width !== width || old.font !== font || old.spacing !== spacing)
        metrics[index] = { width, font, spacing }
    }
    const observer = new ResizeObserver(update)
    observer.observe(row)
    update()
    return { destroy: () => observer.disconnect() }
  }
  let rows = $derived(lines.map((line, i) =>
    authored || !metrics[i] ? [line] : captionPages(line, metrics[i]!)))
  let count = $derived(rows.reduce((max, parts) => Math.max(max, parts.length), 1))
  let page = $derived(Math.min(count - 1, Math.max(0, Math.floor(progress * count))))
</script>

{#each rows as parts, i}
  <svelte:element this={audio ? 'span' : 'div'} dir="auto" class="caption-row"
    class:secondary={!authored && i > 0} class:authored use:measure={i}>
    <span>{parts[Math.floor(page * parts.length / count)]}</span>
  </svelte:element>
{/each}

<style>
  .caption-row {
    display: flex;
    width: 100%;
    min-width: 0;
    align-items: center;
    justify-content: center;
    white-space: nowrap;
  }
  .caption-row > span { flex: none; }
  .caption-row.authored > span {
    max-width: 100%;
    white-space: pre-wrap;
    overflow-wrap: anywhere;
  }
</style>

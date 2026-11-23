<script lang="ts">
  import { player as p } from './stores.svelte'
  let {
    box,
    clearance,
  }: { box: { width: number; height: number; left: number; top: number }; clearance: number } =
    $props()
  let cue = $derived(p.cues[p.active])
  let lines = $derived.by(() => {
    if (!cue || p.settings.subtitleMode === 'off') return []
    if (p.settings.subtitleMode === 'original') return [cue.original]
    if (p.settings.subtitleMode === 'translation') return [cue.translation ?? '']
    return (
      p.settings.bilingualOrder === 'translation-first'
        ? [cue.translation, cue.original]
        : [cue.original, cue.translation]
    ).filter(Boolean)
  })
</script>

{#if lines.length && box.height > 0}<div
    class="content-area"
    style:width={box.width + 'px'}
    style:height={box.height + 'px'}
    style:left={box.left + 'px'}
    style:top={box.top + 'px'}
  >
    <div
      class="subtitle-overlay"
      style:bottom={Math.max(box.height * p.settings.subtitleBottomOffset, clearance) + 'px'}
      style:--main-size={box.height * 0.042 * p.settings.subtitleScale + 'px'}
      style:--sub-size={box.height * 0.034 * p.settings.subtitleScale + 'px'}
      style:--sub-bg={p.settings.subtitleBackground}
    >
      {#each lines as line, i}{#if line}<div dir="auto" class:secondary={i > 0}>
            <span>{line}</span>
          </div>{/if}{/each}
    </div>
  </div>{/if}

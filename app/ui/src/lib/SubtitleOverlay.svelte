<script lang="ts">
  import { onMount, untrack } from 'svelte'
  import { player as p, mockMode, cueAt } from './stores.svelte'
  import { subtitleTracks as tracks, generatedLines } from './subtitle-tracks.svelte'
  import CaptionRows from './CaptionRows.svelte'
  let {
    box,
    clearance,
    audioOnly,
  }: { box: { width: number; height: number; left: number; top: number }; clearance: number; audioOnly: boolean } =
    $props()
  let picker: HTMLInputElement
  let at = $derived(p.ms - tracks.delayMs)
  let cue = $derived(!audioOnly && tracks.selected === 'generated' ? p.cues[cueAt(p.cues, at)] : undefined)
  let lines = $derived(!audioOnly && tracks.authored
    ? tracks.linesAt(p.ms, p.cues, p.settings) : generatedLines(cue, p.settings, true))
  let progress = $derived(cue ? (at - cue.startMs) / (cue.endMs - cue.startMs) : 0)
  // Bound the type scale geometrically on extreme portrait media, independently
  // of the text being displayed.
  let base = $derived(Math.min(box.height, box.width * 1.9) * p.settings.subtitleScale)
  $effect(() => {
    const id = p.media?.sessionId ?? ''
    untrack(() => {
      void tracks.discover(id)
    })
  })
  function importTrack() {
    if (!p.media) return
    p.error = ''
    tracks.attach(p.media.sessionId)
    if (mockMode) picker.click()
    else void p.act(() => tracks.importFile())
  }
  onMount(() => {
    window.addEventListener('hlp:subtitle-file', importTrack)
    return () => window.removeEventListener('hlp:subtitle-file', importTrack)
  })
</script>

<input
  type="file"
  hidden
  bind:this={picker}
  accept=".srt,.vtt,text/vtt,application/x-subrip"
  aria-label={p.locale === 'zh' ? '字幕文件' : 'Subtitle File'}
  onchange={(event) => {
    const file = event.currentTarget.files?.[0]
    event.currentTarget.value = ''
    if (file) void p.act(() => tracks.importFile(file))
  }}
/>

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
      style:--main-size={base * 0.042 + 'px'}
      style:--sub-size={base * 0.034 + 'px'}
      style:--sub-bg={p.settings.subtitleBackground}
    >
      <CaptionRows {lines} {progress} authored={!!tracks.authored} />
    </div>
  </div>{/if}

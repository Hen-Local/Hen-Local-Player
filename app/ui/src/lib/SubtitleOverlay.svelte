<script lang="ts">
  import { onMount, untrack } from 'svelte'
  import { player as p, mockMode } from './stores.svelte'
  import { subtitleTracks as tracks } from './subtitle-tracks.svelte'
  import { fitCaption } from './fit-caption'
  let {
    box,
    clearance,
    audioOnly,
  }: { box: { width: number; height: number; left: number; top: number }; clearance: number; audioOnly: boolean } =
    $props()
  let picker: HTMLInputElement
  let lines = $derived(audioOnly ? [] : tracks.linesAt(p.ms, p.cues, p.settings))
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
    style:--picture-gutter={box.left + 'px'}
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
      {#each lines as line, i}
        <div dir="auto" class:secondary={!tracks.timeline && i > 0} use:fitCaption>
          <span>{line}</span>
        </div>
      {/each}
    </div>
  </div>{/if}

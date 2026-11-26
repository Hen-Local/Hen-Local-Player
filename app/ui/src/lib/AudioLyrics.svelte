<script lang="ts">
  import { tick } from 'svelte'
  import { player as p } from './stores.svelte'
  import { generatedLines, subtitleTracks as tracks } from './subtitle-tracks.svelte'
  import { lastStartedCue, visibleIndices } from './subtitle-tracks'
  import CaptionRows from './CaptionRows.svelte'
  import Icon from './Icon.svelte'

  let { onPageEnd }: { onPageEnd?: (ms: number | null) => void } = $props()
  let authored = $derived(tracks.authored)
  let cues = $derived(tracks.selected === 'off' || (!authored && p.settings.subtitleMode === 'off')
    ? [] : authored?.cues ?? p.cues)
  let time = $derived(authored ? p.ms : p.ms - tracks.delayMs)
  let latest = $derived(lastStartedCue(cues, time))
  let current = $derived(authored ? (visibleIndices(authored, time).at(-1) ?? -1)
    : latest >= 0 && time < cues[latest].endMs ? latest : -1)
  function nextPage(fraction: number | null) {
    const cue = cues[current]
    onPageEnd?.(cue && fraction !== null
      ? cue.startMs + (cue.endMs - cue.startMs) * fraction + tracks.delayMs : null)
  }
  $effect(() => { if (authored || current < 0) onPageEnd?.(null) })
  let anchor = $derived(Math.max(0, current < 0 ? latest : current))
  let viewport = $state<HTMLDivElement>()
  let step = $state(100), height = $state(0), scroll = $state(0)
  let browsing = $state(false)
  let center = $derived(Math.max(0, Math.min(cues.length - 1, Math.round(scroll / step))))
  const overscan = 2
  let radius = $derived(Math.ceil(height / step) + overscan)
  let nearby = $derived.by(() => {
    const first = Math.max(0, center - radius)
    return cues.slice(first, center + radius + 1).map((cue, i) => ({
      id: 'id' in cue ? cue.id : first + i,
      index: first + i,
      lines: 'text' in cue ? cue.text.split('\n') : generatedLines(cue, p.settings, true),
      startMs: cue.startMs, endMs: cue.endMs,
      seekTo: Math.max(0, cue.startMs + (authored ? 0 : tracks.delayMs)),
    }))
  })
  $effect(() => {
    void tracks.selected
    void p.media?.sessionId
    void p.track
    void p.seekRevision
    browsing = false
  })
  $effect(() => {
    if (browsing || !viewport || !height || !cues.length) return
    const top = anchor * step
    const adjacent = Math.abs(top - viewport.scrollTop) <= step * 1.5
    const smooth = adjacent && !matchMedia('(prefers-reduced-motion: reduce)').matches
    viewport.scrollTo({ top, behavior: smooth ? 'smooth' : 'instant' })
  })
  function browse() {
    if (!viewport || browsing) return
    viewport.scrollTo({ top: viewport.scrollTop, behavior: 'instant' })
    browsing = true
  }
  async function navigate(event: KeyboardEvent) {
    if (!viewport || event.altKey || event.ctrlKey || event.metaKey) return
    const page = Math.max(1, Math.floor(height / step))
    const destinations: Record<string, number> = {
      ArrowUp: center - 1, ArrowDown: center + 1,
      PageUp: center - page, PageDown: center + page, Home: 0, End: cues.length - 1,
    }
    const target = destinations[event.key]
    if (target === undefined) return
    event.preventDefault()
    browse()
    const index = Math.max(0, Math.min(cues.length - 1, target))
    viewport.scrollTop = index * step
    scroll = viewport.scrollTop
    await tick()
    viewport?.querySelector<HTMLButtonElement>(`[data-index="${index}"]`)?.focus({ preventScroll: true })
  }
</script>

<div class="audio-canvas" aria-label={p.t('audioOnly')}
  style:--lyric-scale={p.settings.subtitleScale}
  style:--reading-position={(51 - p.settings.subtitleBottomOffset * 55) + 'cqh'}
  style:--lyric-background={p.settings.subtitleBackground}>
  {#if !cues.length}
    <div class="audio-placeholder">
      <Icon name="audio" size={42} />
      <span>{p.t('audioOnly')}</span>
    </div>
  {:else}
    <div class="lyrics" role="region" aria-label={p.t('subtitles')} class:browsing
      bind:this={viewport} bind:clientHeight={height}
      onscroll={event => scroll = event.currentTarget.scrollTop}
      onwheel={event => { if (event.deltaY) browse() }}
      ontouchmove={browse}
      onpointerdown={event => { if (event.target === viewport) browse() }}>
      <div class="lyric-step" aria-hidden="true" bind:clientHeight={step}></div>
      <div class="lyric-track" style:height={`calc(100% + ${(cues.length - 1) * step}px)`}>
        {#each nearby as row (row.id)}
          <button class="lyric" data-drag-surface class:current={row.index === current}
            class:resting={current < 0 && row.index === anchor}
            class:distant={Math.abs(row.index - center) > 1}
            data-index={row.index} style:--row-offset={row.index * step + 'px'}
            aria-current={row.index === current ? 'time' : undefined}
            tabindex={row.index === center ? 0 : -1} onkeydown={navigate}
            onclick={() => { browsing = false; p.seek(row.seekTo) }}>
            <CaptionRows lines={row.lines} progress={(time - row.startMs) / (row.endMs - row.startMs)}
              authored={!!authored} onNextPage={!authored && row.index === current ? nextPage : undefined} audio />
          </button>
        {/each}
      </div>
    </div>
    {#if browsing}
      <button class="follow-lyrics" onclick={() => { browsing = false }}>
        <Icon name="audio" size={15} />{p.t('nowPlaying')}
      </button>
    {/if}
  {/if}
</div>

<style>
  .audio-canvas {
    position: absolute;
    inset: var(--stage-header) 0 var(--stage-footer-gap);
    container-type: size;
    color: #f6f5fc;
  }
  .audio-placeholder {
    position: absolute;
    inset: 0;
    display: grid;
    place-content: center;
    justify-items: center;
    gap: 13px;
    color: #bfbfcf;
    font-size: 14px;
    font-weight: 550;
    letter-spacing: -.015em;
    pointer-events: none;
  }
  .audio-placeholder :global(svg) { color: var(--accent); }
  .lyrics {
    position: absolute;
    inset: 0;
    overflow: hidden auto;
    overscroll-behavior: contain;
    overflow-anchor: none;
    --reading-size: calc(clamp(16px, min(4.2cqw, 6.4cqh), 44px) * var(--lyric-scale));
    --line-step: max(2.8em, clamp(49px, 22cqh, 112px));
    font-size: var(--reading-size);
    mask-image: linear-gradient(transparent, black 12%, black 85%, transparent);
  }
  .lyric-step {
    position: absolute;
    visibility: hidden;
    height: var(--line-step);
    pointer-events: none;
  }
  .lyric-track { position: relative; }
  .follow-lyrics {
    position: absolute;
    inset-block-start: 8px;
    inset-inline-end: 16px;
    gap: 6px;
  }
  button.lyric {
    position: absolute;
    inset-inline: 8%;
    top: calc(var(--reading-position) + var(--row-offset));
    width: 84%;
    display: flex;
    flex-direction: column;
    gap: .15em;
    padding: .1em 0;
    border: 0;
    border-radius: 8px;
    background: transparent;
    color: inherit;
    font: inherit;
    line-height: 1.2;
    font-weight: 650;
    letter-spacing: -.025em;
    opacity: .43;
    transform: translateY(-50%);
    transition: opacity 220ms;
  }
  button.lyric.distant { opacity: .21; }
  .browsing button.lyric { opacity: .75; }
  button.lyric.resting { opacity: .68; }
  button.lyric.current, button.lyric:hover { opacity: 1; }
  button.lyric:hover { background: transparent; }
  .lyric :global(.caption-row) { min-height: 1.2em; }
  .lyric :global(.caption-row.secondary) { font-size: .74em; font-weight: 500; letter-spacing: 0; }
  .current :global(.caption-row > span:not(:empty)) {
    background: rgb(0 0 0 / var(--lyric-background));
    border-radius: .12em;
  }
  @media (prefers-reduced-motion: reduce) {
    button.lyric { transition: none; }
  }
</style>

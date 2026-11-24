<script lang="ts">
  import { player as p } from './stores.svelte'
  import { generatedLines, subtitleTracks as tracks } from './subtitle-tracks.svelte'
  import { lastStartedCue, visibleIndices } from './subtitle-tracks'
  import CaptionRows from './CaptionRows.svelte'
  import Icon from './Icon.svelte'

  let authored = $derived(tracks.authored)
  let cues = $derived(tracks.selected === 'off' || (!authored && p.settings.subtitleMode === 'off')
    ? [] : authored?.cues ?? p.cues)
  let time = $derived(authored ? p.ms : p.ms - tracks.delayMs)
  let latest = $derived(lastStartedCue(cues, time))
  let current = $derived(authored ? (visibleIndices(authored, time).at(-1) ?? -1)
    : latest >= 0 && time < cues[latest].endMs ? latest : -1)
  // Hold the reading position through pauses. Only five cue groups have DOM
  // nodes even when the recording contains hours of speech.
  let anchor = $derived(current < 0 ? Math.max(0, latest) : current)
  let nearby = $derived.by(() => {
    const first = Math.max(0, anchor - 2)
    return cues.slice(first, anchor + 3).map((cue, i) => ({
      id: 'id' in cue ? cue.id : first + i,
      index: first + i,
      lines: 'text' in cue ? cue.text.split('\n') : generatedLines(cue, p.settings, true),
      startMs: cue.startMs, endMs: cue.endMs,
      seekTo: Math.max(0, cue.startMs + (authored ? 0 : tracks.delayMs)),
    }))
  })
</script>

<div class="audio-canvas" aria-label={p.t('audioOnly')}
  style:--lyric-scale={p.settings.subtitleScale}
  style:--reading-position={(51 - p.settings.subtitleBottomOffset * 55) + '%'}
  style:--lyric-background={p.settings.subtitleBackground}>
  {#if !cues.length}
    <div class="audio-placeholder">
      <Icon name="audio" size={42} />
      <span>{p.t('audioOnly')}</span>
    </div>
  {:else}
    <div class="lyrics" aria-label={p.t('subtitles')}>
      {#each nearby as row (row.id)}
        <button class="lyric" data-drag-surface class:current={row.index === current}
          class:resting={current < 0 && row.index === anchor}
          class:distant={Math.abs(row.index - anchor) > 1}
          style:--distance={row.index - anchor}
          aria-current={row.index === current ? 'time' : undefined}
          tabindex={row.index === anchor ? 0 : -1}
          onclick={() => p.seek(row.seekTo)}>
          <CaptionRows lines={row.lines} progress={(time - row.startMs) / (row.endMs - row.startMs)}
            authored={!!authored} audio />
        </button>
      {/each}
    </div>
  {/if}
</div>

<style>
  .audio-canvas {
    position: absolute;
    inset: var(--stage-header) var(--caption-right) var(--stage-footer-gap) var(--caption-left);
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
  .audio-placeholder :global(svg) { color: var(--glass-accent); }
  .lyrics {
    position: absolute;
    inset: 0;
    overflow: hidden;
    --reading-size: calc(clamp(16px, min(4.2cqw, 6.4cqh), 44px) * var(--lyric-scale));
    --line-step: max(2.8em, clamp(49px, 22cqh, 112px));
    font-size: var(--reading-size);
    mask-image: linear-gradient(transparent, black 12%, black 85%, transparent);
  }
  button.lyric {
    position: absolute;
    inset-inline: 8%;
    top: var(--reading-position);
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
    transform: translateY(calc(-50% + var(--distance) * var(--line-step)));
    transition: transform 280ms cubic-bezier(.2,.75,.3,1), opacity 220ms;
  }
  button.lyric.distant { opacity: .21; }
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

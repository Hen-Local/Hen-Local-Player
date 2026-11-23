<script lang="ts">
  import { player as p } from './stores.svelte'
  import { clock } from './format'
  import Icon from './Icon.svelte'

  let {
    buffered = 0,
    subtitlesOpen = false,
    playlistOpen = false,
    onsettings,
    onplaylist,
    onmore,
    onhold,
  }: {
    buffered?: number
    subtitlesOpen?: boolean
    playlistOpen?: boolean
    onsettings: () => void
    onplaylist: () => void
    onmore: () => void
    onhold: (held: boolean) => void
  } = $props()

  let bar: HTMLDivElement
  let hoveringBar = $state(false),
    keyboardFocus = $state(false),
    pointerHeld = $state(false)
  let hover = $state<number | null>(null),
    scrubFocus = $state(false)
  const pct = (ms: number) => Math.min(100, Math.max(0, (ms * 100) / (p.duration || 1)))

  $effect(() => onhold(hoveringBar || keyboardFocus || pointerHeld))

  function updateFocus() {
    keyboardFocus = Boolean(bar?.querySelector(':focus-visible'))
  }
</script>

<svelte:window
  onpointerup={() => (pointerHeld = false)}
  onpointercancel={() => (pointerHeld = false)}
/>

<div
  class="float-bar"
  bind:this={bar}
  data-transport
  role="group"
  aria-label={p.t('playMedia')}
  onpointerenter={(e) => (hoveringBar = e.pointerType !== 'touch')}
  onpointerleave={() => (hoveringBar = false)}
  onpointerdown={() => {
    pointerHeld = true
    keyboardFocus = false
  }}
  onfocusin={updateFocus}
  onfocusout={(e) => {
    if (!(e.relatedTarget instanceof Node) || !bar.contains(e.relatedTarget)) keyboardFocus = false
  }}
>
  <div class="sound">
    <button
      class="float-button"
      title={p.volume ? p.t('mute') : p.t('unmute')}
      aria-label={p.volume ? p.t('mute') : p.t('unmute')}
      onclick={() => p.mute()}
    >
      <Icon name={p.volume ? 'volume' : 'mute'} size={18} />
    </button>
    <input
      class="level"
      type="range"
      min="0"
      max="1"
      step=".05"
      bind:value={p.volume}
      aria-label={p.t('volume')}
      aria-valuetext={Math.round(p.volume * 100) + '%'}
      style:--level={p.volume * 100 + '%'}
    />
  </div>

  <div class="playback-buttons">
    <button
      class="float-button skip-control"
      title={p.t('skipBack')}
      aria-label={p.t('skipBack')}
      disabled={!p.video}
      onclick={() => p.skip(-10000)}><Icon name="rewind" size={21} /></button
    >
    <button
      class="float-button play-button"
      title={p.playing ? p.t('pause') : p.t('play')}
      aria-label={p.playing ? p.t('pause') : p.t('play')}
      disabled={!p.video}
      onclick={() => p.toggle()}
    >
      <Icon name={p.playing ? 'pause' : 'play'} size={23} />
    </button>
    <button
      class="float-button skip-control"
      title={p.t('skipForward')}
      aria-label={p.t('skipForward')}
      disabled={!p.video}
      onclick={() => p.skip(10000)}><Icon name="fast-forward" size={21} /></button
    >
  </div>

  <div class="tool-buttons">
    <button
      class="float-button playlist-trigger"
      title={p.t('playlist')}
      aria-label={p.t('playlist')}
      aria-controls={playlistOpen ? 'playlist-sidebar' : undefined}
      aria-expanded={playlistOpen}
      onclick={onplaylist}><Icon name="list" size={19} /></button
    >
    <button
      class="float-button settings-trigger"
      title={p.t('playerSettings')}
      aria-label={p.t('playerSettings')}
      aria-controls="player-settings"
      aria-expanded={subtitlesOpen}
      onclick={onsettings}><Icon name="settings" size={18} /></button
    >
    <button
      class="float-button full-trigger"
      title={p.fullscreen ? p.t('exitFullscreen') : p.t('fullscreen')}
      aria-label={p.fullscreen ? p.t('exitFullscreen') : p.t('fullscreen')}
      onclick={() => p.act(() => p.full(!p.fullscreen))}
      ><Icon name={p.fullscreen ? 'collapse' : 'full'} size={17} /></button
    >
    <button
      class="float-button more-trigger"
      title={p.t('more')}
      aria-label={p.t('more')}
      aria-haspopup="menu"
      onclick={onmore}><Icon name="more" size={19} /></button
    >
  </div>

  <div class="timeline">
    <time>{clock(p.ms)}</time>
    <div
      class="scrubber"
      role="presentation"
      onpointermove={(e) => {
        const rect = e.currentTarget.getBoundingClientRect()
        hover = Math.max(0, Math.min(1, (e.clientX - rect.left) / rect.width))
      }}
      onpointerleave={() => (hover = null)}
    >
      <div class="scrub-track" aria-hidden="true">
        <i class="buffer" style:width={pct(buffered) + '%'}></i>
        <i class="progress" style:width={pct(p.ms) + '%'}></i>
        <b style:left={pct(p.ms) + '%'}></b>
      </div>
      {#if (hover !== null || scrubFocus) && p.status && !['complete', 'needs-setup'].includes(p.status.stage)}
        <div class="coverage-detail" aria-hidden="true">
          <i
            class="extracted"
            style:width={(p.status.vadComplete ? 100 : pct(p.status.extractedUntilMs)) + '%'}
          ></i>
          {#each p.status.asrPendingSpans as [a, b]}<i
              class="asr-hole"
              style:left={pct(a) + '%'}
              style:width={pct(b - a) + '%'}
            ></i>{/each}
          {#each p.status.trPendingSpans as [a, b]}<i
              class="tr-hole"
              style:left={pct(a) + '%'}
              style:width={pct(b - a) + '%'}
            ></i>{/each}
        </div>
      {/if}
      <input
        class="timeline-input"
        type="range"
        min="0"
        max={p.duration || 1}
        step="50"
        value={p.ms}
        disabled={!p.video || !p.duration}
        aria-label={p.t('seek')}
        aria-valuetext={clock(p.ms)}
        oninput={(e) => p.seek(+e.currentTarget.value)}
        onfocus={() => (scrubFocus = true)}
        onblur={() => (scrubFocus = false)}
      />
      {#if hover !== null && p.duration}
        <span class="time-tip" style:left={hover * 100 + '%'}>{clock(hover * p.duration)}</span>
      {/if}
    </div>
    <time class="total" title={p.t('remainingTime')}>−{clock(Math.max(0, p.duration - p.ms))}</time>
  </div>
</div>

<style>
  .float-bar {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto minmax(0, 1fr);
    align-items: center;
    gap: 0 4px;
    width: min(100%, 540px);
    margin: 0 auto;
    padding: 5px 12px 4px;
    box-sizing: border-box;
    color: #fff;
    background: rgb(23 25 31 / 76%);
    border: 1px solid rgb(255 255 255 / 18%);
    border-radius: 12px;
    box-shadow: 0 4px 18px rgb(0 0 0 / 20%);
    -webkit-backdrop-filter: blur(18px);
    backdrop-filter: blur(18px);
    pointer-events: auto;
    user-select: none;
  }
  .playback-buttons,
  .tool-buttons,
  .sound {
    display: flex;
    align-items: center;
    gap: 2px;
  }
  .tool-buttons {
    justify-self: end;
  }
  .float-bar button.float-button {
    display: inline-grid;
    place-items: center;
    flex: none;
    width: 31px;
    height: 34px;
    min-width: 0;
    min-height: 0;
    margin: 0;
    padding: 0;
    border: 0;
    border-radius: 6px;
    color: #eeeef1;
    background: transparent;
    box-shadow: none;
  }
  .float-bar button.play-button {
    width: 39px;
    color: #fff;
  }
  .float-bar button:hover:not(:disabled),
  .float-bar button[aria-expanded='true'] {
    color: white;
    background: rgb(255 255 255 / 14%);
  }
  .float-bar button:disabled {
    opacity: 0.4;
  }
  .float-bar button:focus-visible,
  .level:focus-visible,
  .scrubber:has(:focus-visible) {
    outline: 2px solid #d1d3ff;
    outline-offset: 2px;
  }
  .timeline {
    grid-column: 1 / -1;
    display: grid;
    grid-template-columns: auto minmax(0, 1fr) auto;
    align-items: center;
    gap: 10px;
  }
  .timeline time,
  .time-tip {
    font:
      500 11px/1.25 -apple-system,
      BlinkMacSystemFont,
      'Segoe UI',
      sans-serif;
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
  }
  .timeline time {
    color: #f3f3f5;
  }
  .timeline time.total {
    color: #c6c7cd;
  }
  .scrubber {
    position: relative;
    height: 21px;
    min-width: 0;
    border-radius: 4px;
  }
  .scrub-track,
  .coverage-detail {
    position: absolute;
    top: 50%;
    left: 0;
    right: 0;
    height: 3px;
    margin-top: -1.5px;
    border-radius: 4px;
    background: rgb(255 255 255 / 22%);
  }
  .scrub-track i,
  .coverage-detail i {
    position: absolute;
    inset: 0 auto 0 0;
    border-radius: inherit;
  }
  .scrub-track .buffer {
    background: rgb(255 255 255 / 21%);
  }
  .scrub-track .progress {
    background: color-mix(in srgb, var(--accent, #565be9) 65%, white);
  }
  .scrub-track b {
    position: absolute;
    top: 50%;
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: #fff;
    transform: translate(-50%, -50%);
  }
  .coverage-detail {
    top: calc(50% + 6px);
    height: 2px;
    overflow: hidden;
  }
  .coverage-detail .extracted {
    background: #a8bbbb;
  }
  .coverage-detail .asr-hole {
    background: #ad8f6b;
  }
  .coverage-detail .tr-hole {
    background: #a7a2d6;
  }
  .timeline-input {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    min-height: 0;
    margin: 0;
    padding: 0;
    opacity: 0;
    cursor: pointer;
  }
  .time-tip {
    position: absolute;
    bottom: calc(100% + 3px);
    transform: translateX(-50%);
    background: #1c1d22;
    border: 1px solid rgb(255 255 255 / 20%);
    border-radius: 4px;
    padding: 4px 6px;
    pointer-events: none;
  }
  .sound {
    gap: 1px;
    margin-right: 4px;
  }
  .level {
    appearance: none;
    -webkit-appearance: none;
    width: 45px;
    height: 26px;
    min-height: 0;
    margin: 0;
    padding: 0;
    border: 0;
    background: transparent;
    cursor: pointer;
  }
  .level::-webkit-slider-runnable-track {
    height: 3px;
    border-radius: 4px;
    background: linear-gradient(to right, #eee var(--level), rgb(255 255 255 / 27%) var(--level));
  }
  .level::-webkit-slider-thumb {
    -webkit-appearance: none;
    width: 7px;
    height: 7px;
    margin-top: -2px;
    border-radius: 50%;
    background: white;
  }
  .level::-moz-range-track {
    height: 3px;
    border-radius: 4px;
    background: rgb(255 255 255 / 27%);
  }
  .level::-moz-range-progress {
    height: 3px;
    background: #eee;
  }
  .level::-moz-range-thumb {
    width: 7px;
    height: 7px;
    border: 0;
    border-radius: 50%;
    background: white;
  }

  @container (max-width: 450px) {
    .float-bar {
      padding-inline: 8px;
    }
    .float-bar button.float-button {
      width: 28px;
    }
    .tool-buttons {
      gap: 0;
    }
  }
  @container (max-width: 370px) {
    .float-bar button.float-button {
      width: 25px;
    }
    .level {
      width: 30px;
    }
    .playback-buttons {
      gap: 0;
    }
  }
  @media (prefers-reduced-transparency: reduce) {
    .float-bar {
      background: #24262d;
      -webkit-backdrop-filter: none;
      backdrop-filter: none;
    }
  }
</style>

<script lang="ts">
  import { onDestroy, tick, untrack, type Snippet } from 'svelte'
  import { playbackSpeedPresets, player as p } from './stores.svelte'
  import { clock } from './format'
  import Icon from './Icon.svelte'
  import PlaybackOutputs from './PlaybackOutputs.svelte'
  import SeekCoverage from './SeekCoverage.svelte'

  let {
    visible,
    children,
    buffered,
    pipAvailable,
    pipActive,
    onpip,
    onhold,
  }: {
    visible: boolean
    children: Snippet
    buffered: [number, number][]
    pipAvailable: boolean
    pipActive: boolean
    onpip: () => void
    onhold: (held: boolean, pressed: boolean) => void
  } = $props()

  let bar: HTMLDivElement
  let hoveringBar = $state(false),
    keyboardFocus = $state(false),
    pointerHeld = $state(false)
  let outputsOpen = $state(false)
  let hover = $state<number | null>(null),
    scrubFocus = $state(false)
  let speedOpen = $state<'hover' | 'active' | null>(null)
  let speedGroup: HTMLDivElement, speedButton: HTMLButtonElement
  let speedSlider = $state<HTMLInputElement>()
  let closeTimer = 0, speedPointer = ''
  let scrubWidth = $state(0), sampledTime = $state(0)
  // The native clock is sufficient when the scrubber advances less than one
  // pixel per timeupdate. Short clips and high speeds benefit from intermediate
  // samples, local to the visible transport.
  const sampleInterval = $derived(scrubWidth > 0 && p.duration > 0
    ? Math.max(33, p.duration / (scrubWidth * p.rate)) : Infinity)
  const sampleTransport = $derived(visible && p.playing && sampleInterval < 250)
  $effect(() => {
    const element = p.video
    if (!sampleTransport || !element) return
    const sample = () => { sampledTime = element.currentTime * 1000 }
    sample()
    const timer = window.setInterval(sample, sampleInterval)
    element.addEventListener('seeking', sample)
    return () => {
      clearInterval(timer)
      element.removeEventListener('seeking', sample)
    }
  })
  // Keep hidden controls mounted for keyboard access without writing their DOM.
  const playhead = $derived(!visible ? untrack(() => p.ms) : sampleTransport ? sampledTime : p.ms)
  const pct = (ms: number) => Math.min(100, Math.max(0, (ms * 100) / (p.duration || 1)))

  $effect(() => onhold(hoveringBar || keyboardFocus || pointerHeld || !!speedOpen || outputsOpen, pointerHeld))

  $effect(() => {
    if (!speedOpen) return
    function dismiss(e: PointerEvent | FocusEvent) {
      if (e.target instanceof Node && !speedGroup.contains(e.target)) speedOpen = null
    }
    document.addEventListener('pointerdown', dismiss, true)
    document.addEventListener('focusin', dismiss)
    return () => {
      document.removeEventListener('pointerdown', dismiss, true)
      document.removeEventListener('focusin', dismiss)
    }
  })

  function enterSpeed(e: PointerEvent) {
    clearTimeout(closeTimer)
    if (e.pointerType === 'mouse' && !speedOpen) speedOpen = 'hover'
  }
  function leaveSpeed() {
    clearTimeout(closeTimer)
    if (speedOpen === 'hover' && !pointerHeld)
      closeTimer = window.setTimeout(() => { if (speedOpen === 'hover') speedOpen = null }, 180)
  }
  async function activateSpeed(e: MouseEvent) {
    if (e.detail && speedPointer === 'mouse') {
      speedOpen = 'hover'
      return
    }
    speedOpen = speedOpen === 'active' ? null : 'active'
    if (speedOpen && !e.detail) {
      await tick()
      speedSlider?.focus({ preventScroll: true })
    }
  }
  function releasePointer() {
    pointerHeld = false
    if (!speedGroup?.matches(':hover')) leaveSpeed()
  }
  onDestroy(() => clearTimeout(closeTimer))

  function updateFocus() {
    keyboardFocus = Boolean(bar?.querySelector(':focus-visible'))
  }
</script>

<svelte:window
  onpointermove={(e) => {
    if (keyboardFocus && e.pointerType === 'mouse' && e.target instanceof Node && !bar.contains(e.target)) keyboardFocus = false
  }}
  onpointerup={releasePointer}
  onpointercancel={releasePointer}
  onblur={() => { pointerHeld = false; speedOpen = null }}
  onkeydowncapture={(e) => {
    if (speedOpen && e.key === 'Escape') {
      e.preventDefault()
      speedOpen = null
      speedButton.focus({ preventScroll: true })
    } else if (speedOpen && e.target instanceof Node && speedGroup.contains(e.target)
      && (e.target !== speedButton || e.key === 'Tab')) {
      speedOpen = 'active'
    }
  }}
/>

<div
  dir="ltr"
  bind:this={bar}
  data-transport-stack
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
  <div class="float-bar" data-transport>
    <div class="sound">
      <button class="float-button mute-trigger" aria-label={p.volume ? p.t('mute') : p.t('unmute')}
        title={p.volume ? p.t('mute') : p.t('unmute')} onclick={() => p.mute()}>
        <Icon name={p.volume ? 'volume' : 'mute'} size={21} />
      </button>
      <input id="transport-volume-slider" type="range" min="0" max="1" step=".05" bind:value={p.volume}
        aria-label={p.t('volume')} aria-valuetext={Math.round(p.volume * 100) + '%'} style:--level={p.volume * 100 + '%'} />
      <div class="speed-control" bind:this={speedGroup} role="group" aria-label={p.t('speed')}
        onpointerenter={enterSpeed} onpointerleave={leaveSpeed}>
        <button class="float-button speed-trigger" bind:this={speedButton}
          aria-label={p.t('speed') + ' ' + p.rate + '×'} title={p.t('speed')} aria-haspopup="dialog"
          aria-expanded={!!speedOpen} aria-controls={speedOpen ? 'transport-speed' : undefined}
          onpointerdown={(e) => speedPointer = e.pointerType} onclick={activateSpeed}>{p.rate}×</button>
        {#if speedOpen}
          <div id="transport-speed" class="adjustment-popover" role="dialog" aria-label={p.t('speed')}>
            <label dir="auto" for="transport-speed-slider">{p.t('speed')} <span>{p.rate}×</span></label>
            <input id="transport-speed-slider" bind:this={speedSlider} type="range" min="0.25" max="5" step="0.25"
              bind:value={p.rate} aria-valuetext={p.rate + '×'} style:--level={(p.rate - 0.25) / 4.75 * 100 + '%'} />
            <div class="speed-presets">
              {#each playbackSpeedPresets as rate}
                <button aria-pressed={p.rate === rate} onclick={() => p.rate = rate}>{rate}×</button>
              {/each}
            </div>
          </div>
        {/if}
      </div>
    </div>

    <div class="playback-buttons">
      <button
        class="float-button skip-control"
        title={p.t('previousFile')}
        aria-label={p.t('previousFile')}
        disabled={p.queueIndex <= 0 && !(p.repeat === 'all' && p.queue.length > 1)}
        onclick={() => p.previous()}><Icon name="previous" size={23} /></button
      >
      <button
        class="float-button play-button"
        title={p.playing ? p.t('pause') : p.t('play')}
        aria-label={p.playing ? p.t('pause') : p.t('play')}
        disabled={!p.video}
        onclick={() => p.toggle()}
      >
        <Icon name={p.playing ? 'pause' : 'play'} size={29} />
      </button>
      <button
        class="float-button skip-control"
        title={p.t('nextFile')}
        aria-label={p.t('nextFile')}
        disabled={!p.queue.length || (p.queueIndex + 1 >= p.queue.length && p.repeat !== 'all')}
        onclick={() => p.next()}><Icon name="next" size={23} /></button
      >
    </div>

    <div class="tool-buttons">
      {@render children()}
      {#key p.media?.sessionId}
        <PlaybackOutputs {pipAvailable} {pipActive} {onpip} onopen={(open) => { outputsOpen = open; if (open) speedOpen = null }} />
      {/key}
    </div>

    <div class="timeline" style:--time-width={clock(p.duration).length + 'ch'}>
      <time>{clock(playhead)}</time>
      <div
        class="scrubber"
        bind:clientWidth={scrubWidth}
        role="presentation"
        onpointermove={(e) => {
          const rect = e.currentTarget.getBoundingClientRect()
          hover = Math.max(0, Math.min(1, (e.clientX - rect.left) / rect.width))
        }}
        onpointerleave={() => (hover = null)}
      >
        <div class="scrub-track" aria-hidden="true">
          {#each buffered as [start, end] (start)}
            <i class="buffer" style:left={pct(start) + '%'} style:width={pct(end) - pct(start) + '%'}></i>
          {/each}
          <i class="progress" style:width={pct(playhead) + '%'}></i>
          <b style:left={pct(playhead) + '%'}></b>
        </div>
        {#if (hover !== null || scrubFocus) && p.status && !['complete', 'needs-setup'].includes(p.status.stage)}
          <SeekCoverage status={p.status} duration={p.duration} />
        {/if}
        <input
          class="timeline-input"
          type="range"
          min="0"
          max={p.duration || 1}
          step="50"
          value={playhead}
          disabled={!p.video || !p.duration}
          aria-label={p.t('seek')}
          aria-valuetext={clock(playhead)}
          oninput={(e) => p.seek(+e.currentTarget.value)}
          onfocus={() => (scrubFocus = true)}
          onblur={() => (scrubFocus = false)}
        />
        {#if hover !== null && p.duration}
          <span class="time-tip" style:left={hover * 100 + '%'}>{clock(hover * p.duration)}</span>
        {/if}
      </div>
      <time class="total" title={p.t('remainingTime')}>−{clock(Math.max(0, p.duration - playhead))}</time>
    </div>
  </div>
</div>

<style>
  .float-bar {
    --transport-opacity: 1;
    --transport-fade: 160ms;
    --hover: transparent;
    --icon-width: 36px;
    --glass-visibility: visible;
    --glass-hide-delay: 0s;
    position: relative;
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto minmax(0, 1fr);
    align-items: center;
    gap: 3px 6px;
    width: min(100%, 652px);
    margin: 0 auto;
    padding: 9px 18px 8px;
    box-sizing: border-box;
    color: #fafbfd;
    border-radius: 21px;
    isolation: isolate;
    pointer-events: auto;
    user-select: none;
  }
  /* A fading ancestor isolates its descendants from the video backdrop.
     Fade the glass itself and its neighboring controls on the same clock. */
  :global(.transport-hidden) .float-bar {
    --transport-opacity: 0;
    --glass-visibility: hidden;
    --glass-hide-delay: var(--transport-fade);
  }
  .float-bar::before, .float-bar::after, .adjustment-popover::before, .adjustment-popover::after,
  .adjustment-popover > label, .adjustment-popover > input, .speed-presets,
  .playback-buttons, .tool-buttons, .timeline,
  .sound > input, .sound > button, .speed-trigger {
    opacity: var(--transport-opacity);
    transition: opacity var(--transport-fade) ease-in-out;
  }
  .float-bar::before, .float-bar::after, .adjustment-popover::before, .adjustment-popover::after {
    content: '';
    position: absolute;
    inset: 0;
    pointer-events: none;
    border-radius: inherit;
  }
  .float-bar::before, .adjustment-popover::before {
    z-index: -1;
    visibility: var(--glass-visibility);
    transition: opacity var(--transport-fade) ease-in-out, visibility 0s var(--glass-hide-delay);
    background: #24262d;
    box-shadow: 0 5px 20px rgb(0 0 0 / 28%);
  }
  /* Keep the menu shadows within the gap above the playback buttons. */
  .adjustment-popover::before, .float-bar :global(#playback-outputs) {
    clip-path: inset(-36px -36px -4px);
  }
  .float-bar::after, .adjustment-popover::after {
    padding: 1px;
    background: var(--glass-reflection);
    -webkit-mask: linear-gradient(#000 0 0) content-box, linear-gradient(#000 0 0);
    mask: linear-gradient(#000 0 0) content-box, linear-gradient(#000 0 0);
    -webkit-mask-composite: xor;
    mask-composite: exclude;
  }
  @supports ((backdrop-filter: blur(1px)) or (-webkit-backdrop-filter: blur(1px))) {
    .float-bar::before, .adjustment-popover::before {
      background: var(--glass-fill);
      -webkit-backdrop-filter: var(--glass-backdrop);
      backdrop-filter: var(--glass-backdrop);
    }
  }
  .playback-buttons,
  .tool-buttons,
  .sound {
    display: flex;
    align-self: stretch;
    align-items: stretch;
    gap: 3px;
  }
  .tool-buttons {
    justify-self: end;
    min-width: 0;
    max-width: 100%;
  }
  .float-bar :global(button.float-button) {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    flex: none;
    width: var(--icon-width);
    min-width: 0;
    min-height: 36px;
    margin: 0;
    padding: 0;
    border: 0;
    border-radius: 8px;
    color: #f7f8fa;
    background: transparent;
    box-shadow: none;
    transition: opacity var(--transport-fade) ease-in-out;
  }
  .float-bar :global(button.float-button:active:not(:disabled)) { background: rgb(255 255 255 / 23%); }
  .float-bar :global(button.float-button:focus-visible) {
    outline: 2px solid var(--glass-focus);
    outline-offset: 2px;
  }
  .float-bar :global(.panel-toggle[aria-expanded='true'] svg) {
    color: var(--indicator-accent);
    filter: var(--indicator-shadow);
  }
  .float-bar :global(button.panel-toggle) {
    flex: 0 1 auto;
    width: auto;
    gap: 4px;
    padding: 3px 7px;
    font-size: 12px;
    font-weight: 570;
    line-height: 1.15;
  }
  .tool-buttons :global(.panel-toggle svg) { flex: none; }
  .tool-buttons :global(.panel-toggle span) { min-width: 0; overflow-wrap: anywhere; text-wrap: balance; }
  .speed-control { display: flex; flex: none; }
  .float-bar button.speed-trigger {
    position: relative;
    width: 45px;
    font: 600 12px/1 -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif;
    font-variant-numeric: tabular-nums;
  }
  /* Keep a continuous hover target across the space above the speed button. */
  .speed-trigger[aria-expanded='true']::before {
    content: '';
    position: absolute;
    inset: -24px 0 100%;
  }
  .adjustment-popover {
    position: absolute;
    bottom: calc(100% + 8px);
    left: 2px;
    z-index: 1;
    isolation: isolate;
    display: grid;
    gap: 6px;
    width: min(352px, calc(100% - 4px));
    padding: 13px 14px 10px;
    border-radius: 15px;
    font-size: 12px;
    font-variant-numeric: tabular-nums;
  }
  .adjustment-popover label,
  .speed-presets {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }
  .adjustment-popover label {
    font-weight: 600;
  }
  .adjustment-popover label span {
    color: #dddfea;
  }
  .speed-presets button {
    min-width: 0;
    flex: 1;
    padding: 6px 0;
    min-height: 28px;
    border-radius: 7px;
    border: none;
    background: transparent;
    color: #e3e4e8;
    font-size: 11px;
    font-weight: 550;
  }
  .speed-presets button[aria-pressed='true'] {
    color: white;
    background: var(--surface-selection);
  }
  .float-bar button:disabled {
    opacity: 0.4;
  }
  .float-bar button:focus-visible,
  .float-bar input[type='range']:focus-visible,
  .scrubber:has(:focus-visible) {
    outline: 2px solid var(--glass-focus);
    outline-offset: 2px;
  }
  .timeline {
    grid-column: 1 / -1;
    display: grid;
    grid-template-columns: var(--time-width) minmax(0, 1fr) calc(var(--time-width) + 1ch);
    align-items: center;
    gap: 10px;
  }
  .timeline,
  .time-tip {
    font:
      500 12px/1.25 -apple-system,
      BlinkMacSystemFont,
      'Segoe UI',
      sans-serif;
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
  }
  .timeline time {
    font-family: inherit;
    color: #f3f3f5;
  }
  .timeline time.total {
    color: #e0e2e9;
    text-align: right;
  }
  .scrubber {
    position: relative;
    height: 25px;
    min-width: 0;
    border-radius: 4px;
  }
  .scrub-track {
    position: absolute;
    top: 50%;
    left: 0;
    right: 0;
    height: 5px;
    transform: translateY(-50%);
    border-radius: 4px;
    background: rgb(255 255 255 / 22%);
  }
  .scrub-track i {
    position: absolute;
    inset: 0 auto 0 0;
    border-radius: inherit;
  }
  .scrub-track .buffer {
    background: rgb(255 255 255 / 21%);
  }
  .scrub-track .progress {
    background: #fff;
  }
  .scrub-track b {
    position: absolute;
    top: 50%;
    width: 15px;
    height: 10px;
    border-radius: 6px;
    background: #fff;
    box-shadow: 0 1px 3px #0004;
    transform: translate(-50%, -50%);
  }
  @media (forced-colors: active) {
    .scrub-track {
      forced-color-adjust: none;
      background: Canvas;
      outline: 1px solid GrayText;
    }
    .scrub-track .buffer { background: GrayText; }
    .scrub-track .progress { background: CanvasText; }
    .scrub-track b { background: CanvasText; box-shadow: 0 0 0 1px Canvas; }
    .float-bar :global(.panel-toggle[aria-expanded='true'] svg) { forced-color-adjust: none; }
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
    box-shadow: 0 2px 8px rgb(0 0 0 / 28%);
    border-radius: 4px;
    padding: 4px 6px;
    pointer-events: none;
  }
  .sound { min-width: 0; }
  .sound input[type='range'] {
    appearance: none;
    -webkit-appearance: none;
    width: 100%;
    height: 24px;
    min-height: 0;
    margin: 0;
    padding: 0;
    border: 0;
    background: transparent;
    cursor: pointer;
  }
  .sound #transport-volume-slider {
    align-self: center;
    width: 58px;
    min-width: 24px;
    flex: 0 1 58px;
    margin-right: 5px;
  }
  .sound input::-webkit-slider-runnable-track {
    height: 4px;
    border-radius: 4px;
    background: linear-gradient(to right, #eee var(--level), rgb(255 255 255 / 27%) var(--level));
  }
  .sound input::-webkit-slider-thumb {
    -webkit-appearance: none;
    width: 10px;
    height: 10px;
    margin-top: -3px;
    border-radius: 50%;
    background: white;
  }
  .sound input::-moz-range-track {
    height: 4px;
    border-radius: 4px;
    background: rgb(255 255 255 / 27%);
  }
  .sound input::-moz-range-progress {
    height: 4px;
    background: #eee;
  }
  .sound input::-moz-range-thumb {
    width: 10px;
    height: 10px;
    border: 0;
    border-radius: 50%;
    background: white;
  }

  @container (max-width: 595px) {
    .float-bar :global(button.panel-toggle) { gap: 3px; padding-inline: 6px; }
    .tool-buttons :global(.panel-toggle svg) { width: 13px; }
  }
  @container (max-width: 530px) {
    .float-bar :global(button.panel-toggle) {
      flex-direction: column;
      gap: 2px;
      padding: 2px 3px;
      font-size: 11.5px;
    }

    .float-bar { --icon-width: 32px; padding-inline: 12px; }
    .tool-buttons { gap: 1px; }
  }
  @container (max-width: 420px) {
    .float-bar {
      --icon-width: 28px;
      gap: 2px 4px;
      padding: 7px 9px 6px;
    }
    .float-bar button.speed-trigger { width: 37px; }
    .playback-buttons { justify-content: center; gap: 1px; }
    .sound { gap: 1px; }
    .sound #transport-volume-slider { margin-right: 3px; }
  }
  @container (max-width: 366px) {
    .float-bar {
      grid-template-columns: minmax(0, 2fr) minmax(0, 3fr);
      grid-template-rows: repeat(2, minmax(36px, 1fr)) auto;
      padding-inline: 7px;
    }
    .playback-buttons { grid-row: 2; grid-column: 1 / -1; }
    .float-bar :global(button.panel-toggle) { flex-direction: row; }
  }
  @media (prefers-reduced-motion: reduce) {
    .float-bar { --transport-fade: 0s; }
  }
  @media (prefers-reduced-transparency: reduce), (prefers-contrast: more) {
    .float-bar::before, .adjustment-popover::before {
      background: #24262d;
      -webkit-backdrop-filter: none;
      backdrop-filter: none;
    }
    .float-bar::after, .adjustment-popover::after { content: none; }
  }
</style>

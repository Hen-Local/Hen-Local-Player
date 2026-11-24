<script lang="ts">
  import { tick } from 'svelte'
  import { playbackSpeedPresets, player as p } from './stores.svelte'
  import { clock } from './format'
  import Icon from './Icon.svelte'
  import SeekCoverage from './SeekCoverage.svelte'

  let {
    buffered = 0,
    subtitlesOpen = false,
    playlistOpen = false,
    pipAvailable,
    pipActive,
    onpip,
    onsettings,
    onplaylist,
    onmore,
    onhold,
  }: {
    buffered?: number
    subtitlesOpen?: boolean
    playlistOpen?: boolean
    pipAvailable: boolean
    pipActive: boolean
    onpip: () => void
    onsettings: () => void
    onplaylist: () => void
    onmore: () => void
    onhold: (held: boolean, pressed: boolean) => void
  } = $props()

  let bar: HTMLDivElement
  let hoveringBar = $state(false),
    keyboardFocus = $state(false),
    pointerHeld = $state(false)
  let hover = $state<number | null>(null),
    scrubFocus = $state(false)
  let speedOpen = $state(false)
  let speedGroup: HTMLDivElement, speedButton: HTMLButtonElement
  let speedSlider = $state<HTMLInputElement>()
  const pct = (ms: number) => Math.min(100, Math.max(0, (ms * 100) / (p.duration || 1)))

  $effect(() => onhold(hoveringBar || keyboardFocus || pointerHeld || speedOpen, pointerHeld))

  $effect(() => {
    if (!speedOpen) return
    function dismiss(e: PointerEvent | FocusEvent) {
      if (e.target instanceof Node && !speedGroup.contains(e.target)) speedOpen = false
    }
    document.addEventListener('pointerdown', dismiss, true)
    document.addEventListener('focusin', dismiss)
    return () => {
      document.removeEventListener('pointerdown', dismiss, true)
      document.removeEventListener('focusin', dismiss)
    }
  })

  async function toggleSpeed() {
    speedOpen = !speedOpen
    if (speedOpen) {
      await tick()
      speedSlider?.focus({ preventScroll: true })
    }
  }

  function updateFocus() {
    keyboardFocus = Boolean(bar?.querySelector(':focus-visible'))
  }
</script>

<svelte:window
  onpointerup={() => (pointerHeld = false)}
  onpointercancel={() => (pointerHeld = false)}
  onblur={() => { pointerHeld = false; speedOpen = false }}
  onkeydowncapture={(e) => {
    if (speedOpen && e.key === 'Escape') {
      e.preventDefault()
      speedOpen = false
      speedButton.focus({ preventScroll: true })
    }
  }}
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
      <Icon name={p.volume ? 'volume' : 'mute'} size={21} />
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
    <div class="speed-control" bind:this={speedGroup}>
      <button
        class="float-button speed-trigger"
        bind:this={speedButton}
        aria-label={p.t('speed') + ' ×' + p.rate}
        title={p.t('speed')}
        aria-expanded={speedOpen}
        aria-haspopup="dialog"
        aria-controls={speedOpen ? 'transport-speed' : undefined}
        onclick={toggleSpeed}
      >×{p.rate}</button>
      {#if speedOpen}
        <div id="transport-speed" class="speed-popover" role="dialog" aria-label={p.t('speed')}>
          <label for="transport-speed-slider">{p.t('speed')} <span>×{p.rate}</span></label>
          <input id="transport-speed-slider" type="range" min="0.25" max="5" step="0.25"
            bind:this={speedSlider} bind:value={p.rate} aria-label={p.t('speed')} aria-valuetext={'×' + p.rate}
            style:--level={(p.rate - 0.25) / 4.75 * 100 + '%'} />
          <div class="speed-presets">
            {#each playbackSpeedPresets as rate}
              <button aria-pressed={p.rate === rate} onclick={() => p.rate = rate}>×{rate}</button>
            {/each}
          </div>
        </div>
      {/if}
    </div>
  </div>

  <div class="playback-buttons">
    <button
      class="float-button skip-control"
      title={p.t('skipBack')}
      aria-label={p.t('skipBack')}
      disabled={!p.video}
      onclick={() => p.skip(-10000)}><Icon name="rewind" size={25} /></button
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
      title={p.t('skipForward')}
      aria-label={p.t('skipForward')}
      disabled={!p.video}
      onclick={() => p.skip(10000)}><Icon name="fast-forward" size={25} /></button
    >
  </div>

  <div class="tool-buttons">
    {#if pipAvailable}
      <button class="float-button pip-trigger" title={p.t('pictureInPicture')}
        aria-label={p.t('pictureInPicture')} aria-pressed={pipActive}
        onclick={onpip}><Icon name="pip" size={21} /></button>
    {/if}
    <button
      class="float-button playlist-trigger"
      title={p.t('playlist')}
      aria-label={p.t('playlist')}
      aria-controls={playlistOpen ? 'playlist-sidebar' : undefined}
      aria-expanded={playlistOpen}
      onclick={onplaylist}><Icon name="list" size={22} /></button
    >
    <button
      class="float-button settings-trigger"
      title={p.t('subtitles')}
      aria-label={p.t('subtitles')}
      aria-controls="player-settings"
      aria-expanded={subtitlesOpen}
      onclick={onsettings}><Icon name="settings" size={21} /></button
    >
    <button
      class="float-button more-trigger"
      title={p.t('more')}
      aria-label={p.t('more')}
      aria-haspopup="menu"
      onclick={onmore}><Icon name="more" size={22} /></button
    >
  </div>

  <div class="timeline" style:--time-width={clock(p.duration).length + 'ch'}>
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
        <SeekCoverage status={p.status} duration={p.duration} />
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
    position: relative;
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto minmax(0, 1fr);
    align-items: center;
    gap: 3px 6px;
    width: min(100%, 600px);
    margin: 0 auto;
    padding: 9px 18px 8px;
    box-sizing: border-box;
    color: #fafbfd;
    border-radius: 21px;
    isolation: isolate;
    pointer-events: auto;
    user-select: none;
  }
  /* Keep the optical layer separate so the slider popover samples the movie
     directly too. Only the small controls area needs a backdrop blur. */
  .float-bar::before,
  .speed-popover::before {
    content: '';
    position: absolute;
    inset: 0;
    z-index: -1;
    pointer-events: none;
    border-radius: inherit;
    border: 1px solid rgb(255 255 255 / 30%);
    background: linear-gradient(155deg, rgb(62 69 84 / 43%), rgb(17 21 30 / 54%));
    box-shadow: inset 0 1px 0 rgb(255 255 255 / 22%), inset 0 -1px 0 rgb(255 255 255 / 7%),
      0 5px 22px rgb(0 0 0 / 27%);
    -webkit-backdrop-filter: saturate(165%) blur(20px);
    backdrop-filter: saturate(165%) blur(20px);
  }
  .playback-buttons,
  .tool-buttons,
  .sound {
    display: flex;
    align-items: center;
    gap: 3px;
  }
  .tool-buttons {
    justify-self: end;
  }
  .float-bar button.float-button {
    display: inline-grid;
    place-items: center;
    flex: none;
    width: 40px;
    height: 42px;
    min-width: 0;
    min-height: 0;
    margin: 0;
    padding: 0;
    border: 0;
    border-radius: 9px;
    color: #eeeef1;
    background: transparent;
    box-shadow: none;
  }
  .float-bar button.play-button {
    width: 48px;
    color: #fff;
  }
  .float-bar button.speed-trigger {
    width: 51px;
    height: 33px;
    border: 1px solid rgb(255 255 255 / 16%);
    border-radius: 20px;
    font: 600 12px/1 -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif;
    font-variant-numeric: tabular-nums;
  }
  .speed-control {
    margin-left: 6px;
  }
  .speed-popover {
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
  .speed-popover::before {
    background: linear-gradient(160deg, rgb(52 58 71 / 62%), rgb(27 30 39 / 71%));
  }
  .speed-popover label,
  .speed-presets {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }
  .speed-popover label {
    font-weight: 600;
  }
  .speed-popover label span {
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
    background: rgb(255 255 255 / 18%);
  }
  .float-bar button:hover:not(:disabled),
  .float-bar button[aria-pressed='true'],
  .float-bar button[aria-expanded='true'] {
    color: white;
    background: rgb(255 255 255 / 14%);
  }
  .float-bar button:disabled {
    opacity: 0.4;
  }
  .float-bar button:focus-visible,
  .float-bar input[type='range']:focus-visible,
  .scrubber:has(:focus-visible) {
    outline: 2px solid #d1d3ff;
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
    color: #c6c7cd;
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
    background: color-mix(in srgb, var(--accent, #565be9) 36%, #f7f8ff);
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
  .level, .speed-popover input {
    appearance: none;
    -webkit-appearance: none;
    width: 56px;
    height: 32px;
    min-height: 0;
    margin: 0;
    padding: 0;
    border: 0;
    background: transparent;
    cursor: pointer;
  }
  .speed-popover input { width: 100%; height: 24px; }
  :is(.level, .speed-popover input)::-webkit-slider-runnable-track {
    height: 4px;
    border-radius: 4px;
    background: linear-gradient(to right, #eee var(--level), rgb(255 255 255 / 27%) var(--level));
  }
  :is(.level, .speed-popover input)::-webkit-slider-thumb {
    -webkit-appearance: none;
    width: 10px;
    height: 10px;
    margin-top: -3px;
    border-radius: 50%;
    background: white;
  }
  :is(.level, .speed-popover input)::-moz-range-track {
    height: 4px;
    border-radius: 4px;
    background: rgb(255 255 255 / 27%);
  }
  :is(.level, .speed-popover input)::-moz-range-progress {
    height: 4px;
    background: #eee;
  }
  :is(.level, .speed-popover input)::-moz-range-thumb {
    width: 10px;
    height: 10px;
    border: 0;
    border-radius: 50%;
    background: white;
  }

  @container (max-width: 530px) {
    .float-bar { padding-inline: 12px; }
    .float-bar button.float-button { width: 34px; }
    .float-bar button.play-button { width: 42px; }
    .float-bar button.speed-trigger { width: 46px; }
    .tool-buttons { gap: 1px; }
    .speed-control { margin-left: 3px; }
    .level { width: 39px; }
  }
  @container (max-width: 420px) {
    .float-bar {
      gap: 2px 4px;
      padding: 7px 9px 6px;
    }
    .float-bar button.float-button { width: 30px; height: 38px; }
    .float-bar button.play-button { width: 35px; }
    .float-bar button.speed-trigger { width: 43px; height: 30px; }
    .playback-buttons { justify-content: center; gap: 1px; }
    .level { width: 32px; }
    .sound { margin-right: 0; }
  }
  @container (max-width: 366px) {
    .float-bar { padding-inline: 5px; gap: 2px; }
    .float-bar button.float-button { width: 26px; }
    .float-bar button.play-button { width: 32px; }
    .float-bar button.speed-trigger { width: 39px; }
    .speed-control { margin-left: 1px; }
    .level { width: 24px; }
    .tool-buttons, .playback-buttons { gap: 0; }
  }
  @media (prefers-reduced-transparency: reduce) {
    .float-bar::before, .speed-popover::before {
      background: #24262d;
      -webkit-backdrop-filter: none;
      backdrop-filter: none;
    }
  }
</style>

<script lang="ts">
  import { player as p } from './stores.svelte'
  import { clock } from './format'
  import Icon from './Icon.svelte'
  let { buffered = 0, hidden = false }: { buffered?: number; hidden?: boolean } = $props()
  let hover = $state<number | null>(null),
    lastVolume = 0.85
  const pct = (ms: number) => Math.min(100, Math.max(0, (ms * 100) / (p.duration || 1)))
  function mute() {
    if (p.volume > 0) {
      lastVolume = p.volume
      p.volume = 0
    } else p.volume = lastVolume || 0.85
  }
</script>

<div class="controls" class:idle={hidden}>
  <div
    class="seek-wrap"
    role="presentation"
    onpointermove={(e) => {
      const r = e.currentTarget.getBoundingClientRect()
      hover = Math.max(0, Math.min(1, (e.clientX - r.left) / r.width))
    }}
    onpointerleave={() => (hover = null)}
  >
    <div class="rail">
      <i class="buffered" style:width={pct(buffered) + '%'}></i>
      <i class="extracted" style:width={pct(p.status?.extractedUntilMs ?? 0) + '%'}></i>
      {#each p.status?.asrPendingSpans ?? [] as span}<i
          class="asr-hole"
          style:left={pct(span[0]) + '%'}
          style:width={pct(span[1] - span[0]) + '%'}
        ></i>{/each}
      {#each p.status?.trPendingSpans ?? [] as span}<i
          class="tr-hole"
          style:left={pct(span[0]) + '%'}
          style:width={pct(span[1] - span[0]) + '%'}
        ></i>{/each}
      <i class="played" style:width={pct(p.ms) + '%'}></i>
      <b style:left={pct(p.ms) + '%'}></b>
    </div>
    <input
      class="seek-input"
      type="range"
      min="0"
      max={p.duration || 1}
      step="50"
      value={p.ms}
      oninput={(e) => p.seek(+e.currentTarget.value)}
      disabled={!p.media}
      aria-label={p.t('seek')}
      aria-valuetext={clock(p.ms)}
    />
    {#if hover !== null && p.media}<span class="seek-tip" style:left={hover * 100 + '%'}>
        {clock(hover * p.duration)}
      </span>{/if}
  </div>
  <div class="transport">
    <button
      class="play-button icon"
      title={p.playing ? p.t('pause') : p.t('play')}
      aria-label={p.playing ? p.t('pause') : p.t('play')}
      disabled={!p.media}
      onclick={() => p.toggle()}
    >
      <Icon name={p.playing ? 'pause' : 'play'} size={17} />
    </button>
    <span class="time">
      {clock(p.ms)}
      <span>/ {clock(p.duration)}</span>
    </span>
    <span class="spacer"></span>
    <button
      class="icon"
      title={p.volume ? p.t('mute') : p.t('unmute')}
      aria-label={p.t('volume')}
      onclick={mute}
    >
      <Icon name={p.volume ? 'volume' : 'mute'} />
    </button>
    <input
      class="volume-range"
      type="range"
      aria-label={p.t('volume')}
      min="0"
      max="1"
      step=".05"
      bind:value={p.volume}
    />
    <select class="rate" aria-label={p.t('speed')} bind:value={p.rate}>
      {#each [0.5, 0.75, 1, 1.25, 1.5, 1.75, 2] as speed}<option value={speed}>
          {speed}×
        </option>{/each}
    </select>
    <span class="separator"></span>
    <button
      class="text-icon"
      title={p.t('sizeDown')}
      aria-label={p.t('sizeDown')}
      onclick={() => p.act(() => p.scale(-0.1))}
    >
      A−
    </button>
    <button
      class="text-icon"
      title={p.t('sizeUp')}
      aria-label={p.t('sizeUp')}
      onclick={() => p.act(() => p.scale(0.1))}
    >
      A+
    </button>
    <button
      class="icon"
      title={p.t('fullscreen')}
      aria-label={p.fullscreen ? p.t('exitFullscreen') : p.t('fullscreen')}
      onclick={() => p.act(() => p.full(!p.fullscreen))}
    >
      <Icon name={p.fullscreen ? 'collapse' : 'full'} />
    </button>
  </div>
</div>

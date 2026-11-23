<script lang="ts">
  import { player as p } from './stores.svelte'
  import { clock } from './format'
  import Icon from './Icon.svelte'
  import { togglePiP } from './picture-in-picture'
  let {
    buffered = 0,
    hidden = false,
    pipAvailable = false,
    pipActive = false,
  }: {
    buffered?: number
    hidden?: boolean
    pipAvailable?: boolean
    pipActive?: boolean
  } = $props()
  let hover = $state<number | null>(null)
  let repeatLabel = $derived(
    p.t(p.repeat === 'all' ? 'repeatAll' : p.repeat === 'one' ? 'repeatOne' : 'repeatOff'),
  )
  const pct = (ms: number) => Math.min(100, Math.max(0, (ms * 100) / (p.duration || 1)))
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
      <i
        class="extracted"
        style:width={(p.status?.vadComplete ? 100 : pct(p.status?.extractedUntilMs ?? 0)) + '%'}
      ></i>
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
      class="icon queue-control"
      title={p.t('previousFile')}
      aria-label={p.t('previousFile')}
      disabled={!p.media}
      onclick={() => p.previous()}><Icon name="previous" size={16} /></button
    >
    <button
      class="icon skip-control"
      title={p.t('skipBack')}
      aria-label={p.t('skipBack')}
      disabled={!p.media}
      onclick={() => p.skip(-10000)}><Icon name="skip-back" size={20} /></button
    >
    <button
      class="play-button icon"
      title={p.playing ? p.t('pause') : p.t('play')}
      aria-label={p.playing ? p.t('pause') : p.t('play')}
      disabled={!p.media}
      onclick={() => p.toggle()}
    >
      <Icon name={p.playing ? 'pause' : 'play'} size={17} />
    </button>
    <button
      class="icon skip-control"
      title={p.t('skipForward')}
      aria-label={p.t('skipForward')}
      disabled={!p.media}
      onclick={() => p.skip(10000)}><Icon name="skip-forward" size={20} /></button
    >
    <button
      class="icon queue-control"
      title={p.t('nextFile')}
      aria-label={p.t('nextFile')}
      disabled={p.queueIndex + 1 >= p.queue.length && p.repeat !== 'all'}
      onclick={() => p.next()}><Icon name="next" size={16} /></button
    >
    <span class="time">
      {clock(p.ms)}
      <span>/ {clock(p.duration)}</span>
    </span>
    <span class="spacer"></span>
    <button
      class="icon"
      title={p.volume ? p.t('mute') : p.t('unmute')}
      aria-label={p.volume ? p.t('mute') : p.t('unmute')}
      onclick={() => p.mute()}
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
      {#each [0.25, 0.5, 0.75, 1, 1.25, 1.5, 1.75, 2, 2.5, 3, 4] as speed}<option value={speed}>
          {speed}×
        </option>{/each}
    </select>
    <button
      class="icon repeat-control"
      title={repeatLabel}
      aria-label={repeatLabel}
      aria-pressed={p.repeat !== 'off'}
      onclick={() => p.toggleRepeat()}
    >
      <Icon name={p.repeat === 'one' ? 'repeat-one' : 'repeat'} size={17} />
    </button>
    {#if p.fullscreen}
      {#each ['subtitles', 'playlist'] as const as panel}
        <button
          class="icon fullscreen-panel-control"
          title={p.t(panel)}
          aria-label={p.t(panel)}
          onclick={() => window.dispatchEvent(new CustomEvent('player:panel', { detail: panel }))}
        >
          <Icon name={panel === 'subtitles' ? 'cc' : 'list'} size={17} />
        </button>
      {/each}
    {/if}
    {#if pipAvailable}
      <button
        class="icon pip-control"
        aria-label={p.t('pictureInPicture')}
        title={p.t('pictureInPicture')}
        aria-pressed={pipActive}
        onclick={() =>
          p.act(async () => {
            if (p.video) await togglePiP(p.video)
          })}
      >
        <Icon name="pip" size={17} />
      </button>
    {/if}
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

<script lang="ts">
  import { onMount, tick } from 'svelte'
  import { backend, player as p } from './stores.svelte'
  import SubtitleOverlay from './SubtitleOverlay.svelte'
  import Controls from './Controls.svelte'
  import Icon from './Icon.svelte'
  let stage: HTMLDivElement,
    stageHeight = $state(0)
  let video = $state<HTMLVideoElement>()
  let box = $state({ width: 0, height: 0, left: 0, top: 0 }),
    buffered = $state(0),
    idle = $state(false),
    drag = $state(false),
    audioOnly = $state(false)
  let lastReport = 0,
    raf = 0,
    idleTimer = 0
  let url = $derived(p.prep?.mediaUrl || p.media?.mediaUrl || null)
  function measure() {
    if (!stage || !video) return
    const { clientWidth: w, clientHeight: h } = stage,
      aspect = (video.videoWidth || 16) / (video.videoHeight || 9)
    const width = Math.min(w, h * aspect),
      height = width / aspect
    box = { width, height, left: (w - width) / 2, top: (h - height) / 2 }
  }
  function sync(force = false) {
    if (!video || !p.media) return
    p.ms = video.currentTime * 1000
    if (Number.isFinite(video.duration) && video.duration > 0) p.duration = video.duration * 1000
    if (force || (p.playing && performance.now() - lastReport >= 250)) {
      lastReport = performance.now()
      void p.act(() =>
        backend.call('set_playhead', {
          sessionId: p.media!.sessionId,
          ms: Math.round(p.ms),
          playing: p.playing,
        }),
      )
    }
  }
  function frame() {
    if (!video || video.paused) return
    sync()
    raf = requestAnimationFrame(frame)
  }
  function activity() {
    idle = false
    clearTimeout(idleTimer)
    if (p.playing) idleTimer = window.setTimeout(() => (idle = true), 2500)
  }
  function play() {
    p.playing = true
    sync(true)
    cancelAnimationFrame(raf)
    raf = requestAnimationFrame(frame)
    activity()
  }
  function pause() {
    p.playing = false
    cancelAnimationFrame(raf)
    activity()
    sync(true)
  }
  function loaded() {
    if (!video) return
    measure()
    audioOnly = !video.videoWidth
    sync(true)
    void video.play().catch(() => {
      p.playing = false
    })
  }
  function reportError() {
    if (!p.media || !video?.error) return
    void p.act(() =>
      backend.call('report_playback_error', {
        sessionId: p.media!.sessionId,
        code: video!.error!.code,
      }),
    )
  }
  $effect(() => {
    p.video = video ?? null
    if (video) {
      video.volume = p.volume
      video.playbackRate = p.rate
    }
  })
  $effect(() => {
    const id = p.media?.sessionId
    if (id) {
      buffered = 0
      idle = false
      audioOnly = false
    }
  })
  onMount(() => {
    const size = new ResizeObserver(measure)
    size.observe(stage)
    return () => {
      size.disconnect()
      cancelAnimationFrame(raf)
      clearTimeout(idleTimer)
      p.video = null
    }
  })
</script>

<div
  bind:this={stage}
  bind:clientHeight={stageHeight}
  class="video-stage"
  class:empty={!p.media}
  class:dragging={drag}
  class:hide-cursor={p.fullscreen && idle}
  role="presentation"
  onpointermove={activity}
  onpointerdown={activity}
  onfocusin={activity}
  ondragover={(e) => {
    e.preventDefault()
    drag = true
  }}
  ondragleave={() => (drag = false)}
  ondrop={() => (drag = false)}
>
  {#if !p.media}
    <div class="dropzone">
      <div class="film-card" aria-hidden="true">
        <i></i>
        <span><Icon name="play" size={31} /></span>
        <i></i>
      </div>
      <h1>{p.t('drop')}</h1>
      <p>{p.t('dropHint')}</p>
      <button class="primary" onclick={() => p.choose()}>
        <Icon name="folder" size={17} />{p.t('open')}
      </button>
      <button class="quiet-link" onclick={() => (p.dialog = 'url')}>
        {p.t('paste')}
        <span>↗</span>
      </button>
    </div>
  {:else}
    {#key p.media.sessionId}
      {#if url}<video
          bind:this={video}
          src={url}
          preload="auto"
          playsinline
          onloadedmetadata={loaded}
          onresize={measure}
          onplay={play}
          onpause={pause}
          onended={pause}
          onseeked={() => sync(true)}
          ontimeupdate={() => {
            if (!p.playing) sync()
          }}
          onprogress={() => {
            if (video?.buffered.length)
              buffered = video.buffered.end(video.buffered.length - 1) * 1000
          }}
          onerror={reportError}
        >
          <track kind="captions" />
        </video>
        {#if audioOnly}<div class="stage-state">
            <Icon name="audio" size={54} />
            <span>{p.t('audioOnly')}</span>
          </div>{/if}
      {:else}<div class="stage-state">
          <Icon name="film" size={40} />
          <strong>{p.t('preparing')}</strong>
          <progress max="1" value={p.prep?.progress ?? 0}></progress>
          <small>{Math.round((p.prep?.progress ?? 0) * 100)}%</small>
        </div>{/if}
    {/key}
    <SubtitleOverlay
      {box}
      clearance={idle && p.playing ? 0 : Math.max(0, 85 - (stageHeight - box.top - box.height))}
    />
    {#if p.status?.stage === 'needs-setup'}<button
        class="stage-chip"
        onclick={() => (p.dialog = 'setup')}
      >
        {p.t('needsSetup')} →
      </button>
    {:else if p.status?.asrPendingSpans.some(([a, b]) => a <= p.ms && p.ms < b)}<span
        class="stage-chip"
      >
        {p.t('transcribing')}
      </span>{/if}
  {/if}
  <Controls {buffered} hidden={idle && p.playing} />
</div>

<script lang="ts">
  import { onMount } from 'svelte'
  import { backend, mockMode, player as p } from './stores.svelte'
  import SubtitleOverlay from './SubtitleOverlay.svelte'
  import Controls from './Controls.svelte'
  import Icon from './Icon.svelte'
  import { inPiP, supportsPiP, updatePiPCaptions } from './picture-in-picture'
  import { subtitleTracks } from './subtitle-tracks.svelte'
  let stage: HTMLDivElement,
    stageHeight = $state(0)
  let video = $state<HTMLVideoElement>()
  let box = $state({ width: 0, height: 0, left: 0, top: 0 }),
    buffered = $state(0),
    idle = $state(false),
    pipActive = $state(false),
    pipAvailable = $state(false),
    audioOnly = $state(false)
  let lastReport = 0,
    raf = 0,
    idleTimer = 0,
    clickTimer = 0,
    lastPosition = 0
  let url = $derived(p.prep?.mediaUrl || p.media?.mediaUrl || null)
  function measure() {
    if (!stage || !video) return
    const { clientWidth: w, clientHeight: h } = stage,
      aspect = (video.videoWidth || 16) / (video.videoHeight || 9)
    const width = p.videoFit === 'cover' ? w : Math.min(w, h * aspect),
      height = p.videoFit === 'cover' ? h : width / aspect
    box = { width, height, left: (w - width) / 2, top: (h - height) / 2 }
  }
  function sync(force = false) {
    if (!video || !p.media) return
    p.ms = video.currentTime * 1000
    if (Number.isFinite(video.duration) && video.duration > 0) p.duration = video.duration * 1000
    if (
      navigator.mediaSession?.setPositionState &&
      Number.isFinite(video.duration) &&
      video.duration > 0 &&
      (force || performance.now() - lastPosition >= 1000)
    ) {
      lastPosition = performance.now()
      navigator.mediaSession.setPositionState({
        duration: video.duration,
        playbackRate: video.playbackRate,
        position: Math.min(video.currentTime, video.duration),
      })
    }
    if (force || (p.playing && performance.now() - lastReport >= 250)) {
      lastReport = performance.now()
      p.remember()
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
    p.remember(true)
    cancelAnimationFrame(raf)
    activity()
    sync(true)
  }
  function loaded() {
    if (!video) return
    measure()
    audioOnly = !video.videoWidth
    for (const track of Array.from(video.textTracks)) {
      if (track.kind === 'captions' || track.kind === 'subtitles') track.mode = 'disabled'
    }
    pipAvailable = !audioOnly && supportsPiP(video)
    p.restoreVideo(video)
    sync(true)
  }
  function reportError() {
    if (!p.media || !video?.error) return
    if (mockMode && p.media.sourcePath?.startsWith('browser:')) {
      p.message(p.t('browserUnsupported'))
      return
    }
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
      pipAvailable = false
    }
  })
  $effect(() => {
    p.videoFit
    measure()
  })
  $effect(() => {
    const element = video
    if (!element) return
    const update = () => {
      pipActive = inPiP(element)
    }
    const events = [
      'enterpictureinpicture',
      'leavepictureinpicture',
      'webkitpresentationmodechanged',
    ]
    update()
    for (const event of events) element.addEventListener(event, update)
    return () => {
      for (const event of events) element.removeEventListener(event, update)
    }
  })
  let nativeCaptions: { video: HTMLVideoElement; track: TextTrack } | null = null
  $effect(() => {
    if (!video || !pipActive) {
      if (nativeCaptions) nativeCaptions.track.mode = 'disabled'
      return
    }
    if (nativeCaptions?.video !== video) {
      nativeCaptions = { video, track: video.addTextTrack('captions', p.t('subtitles')) }
    }
    nativeCaptions.track.mode = 'showing'
    updatePiPCaptions(nativeCaptions.track, subtitleTracks.pipCues(p.cues, p.settings))
  })
  function clickStage(e: MouseEvent) {
    if (!(e.target instanceof HTMLElement) || e.target.closest('button,input,select,.controls'))
      return
    clearTimeout(clickTimer)
    if (e.detail === 2) {
      void p.act(() => p.full(!p.fullscreen))
    } else if (e.detail === 1) clickTimer = window.setTimeout(() => p.toggle(), 260)
  }
  $effect(() => {
    if (navigator.mediaSession && typeof MediaMetadata !== 'undefined' && p.media)
      navigator.mediaSession.metadata = new MediaMetadata({ title: p.media.title })
  })
  $effect(() => {
    if (navigator.mediaSession)
      navigator.mediaSession.playbackState = p.playing ? 'playing' : 'paused'
  })
  function mediaKeys() {
    const session = navigator.mediaSession
    if (!session) return () => {}
    const actions: [MediaSessionAction, MediaSessionActionHandler][] = [
      [
        'play',
        () => {
          void p.video?.play().catch((e) => p.message(e))
        },
      ],
      ['pause', () => p.video?.pause()],
      [
        'nexttrack',
        () => {
          void p.next()
        },
      ],
      [
        'previoustrack',
        () => {
          void p.previous()
        },
      ],
      ['seekforward', ({ seekOffset }) => p.skip((seekOffset ?? 10) * 1000)],
      ['seekbackward', ({ seekOffset }) => p.skip(-(seekOffset ?? 10) * 1000)],
      [
        'seekto',
        ({ seekTime }) => {
          if (seekTime !== undefined) p.seek(seekTime * 1000)
        },
      ],
      [
        'stop',
        () => {
          void p.close()
        },
      ],
    ]
    const supported: MediaSessionAction[] = []
    for (const [name, handler] of actions) {
      try {
        session.setActionHandler(name, handler)
        supported.push(name)
      } catch {
        /* Native support varies per desktop webview and action. */
      }
    }
    return () => {
      for (const name of supported) session.setActionHandler(name, null)
      session.metadata = null
      session.playbackState = 'none'
      session.setPositionState?.()
    }
  }
  onMount(() => {
    const removeMediaKeys = mediaKeys()
    const size = new ResizeObserver(measure)
    size.observe(stage)
    return () => {
      removeMediaKeys()
      size.disconnect()
      cancelAnimationFrame(raf)
      clearTimeout(idleTimer)
      clearTimeout(clickTimer)
      p.video = null
    }
  })
</script>

<div
  bind:this={stage}
  bind:clientHeight={stageHeight}
  class="video-stage"
  class:hide-cursor={p.fullscreen && idle}
  role="presentation"
  onpointermove={activity}
  onpointerdown={activity}
  onfocusin={activity}
  onclick={clickStage}
>
  {#if p.media}
    {#key p.media.sessionId}
      {#if url}<video
          bind:this={video}
          src={url}
          style:object-fit={p.videoFit}
          data-session={p.media.sessionId}
          preload="auto"
          playsinline
          onloadedmetadata={loaded}
          onresize={measure}
          onplay={play}
          onpause={pause}
          onended={() => {
            pause()
            p.ended()
          }}
          onseeked={() => sync(true)}
          ontimeupdate={() => {
            if (!p.playing || document.hidden) sync()
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
    {#if p.status?.stage === 'needs-setup' && !url && subtitleTracks.selected === 'generated' && p.settings.subtitleMode !== 'off'}<button
        class="stage-chip"
        onclick={() => (p.dialog = 'setup')}
      >
        {p.t('needsSetup')} →
      </button>
    {:else if !idle && subtitleTracks.selected === 'generated' && p.settings.subtitleMode !== 'off' && p.status?.asrPendingSpans.some(([a, b]) => a <= p.ms && p.ms < b)}<span
        class="stage-chip"
      >
        {p.t('transcribing')}
      </span>{/if}
  {/if}
  <Controls {buffered} {pipAvailable} {pipActive} hidden={idle && p.playing} />
</div>

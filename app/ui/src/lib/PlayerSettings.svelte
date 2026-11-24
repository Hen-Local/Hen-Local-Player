<script lang="ts">
  import { tick, untrack } from 'svelte'
  import { backend, mockMode, playbackSpeedPresets, player as p } from './stores.svelte'
  import { clock } from './format'
  import Icon from './Icon.svelte'
  import { inPiP, supportsPiP, togglePiP } from './picture-in-picture'
  import SubtitlePanel from './SubtitlePanel.svelte'
  import { slidePanel } from './panel-transition'

  const tabs = ['subtitles', 'playback'] as const
  type Tab = (typeof tabs)[number]
  let { tab, onselect, onfocus, onclose }: {
    tab: Tab | null
    onselect: (tab: Tab) => void
    onfocus: () => void
    onclose: () => void
  } = $props()
  let playbackVisible = $derived(tab === 'playback')
  let time = $state('')
  let error = $state('')
  let start = $state<number | null>(null)
  let end = $state<number | null>(null)
  let chapters = $state<{ startMs: number; title: string }[]>([])
  let loadedSession = ''
  let session = $derived(p.media?.sessionId ?? '')
  let repeatLabel = $derived(
    p.t(p.repeat === 'all' ? 'repeatAll' : p.repeat === 'one' ? 'repeatOne' : 'repeatOff'),
  )
  let pipAvailable = $state(false)
  let pipActive = $state(false)
  let frameSize = $state('')

  function tabKey(e: KeyboardEvent) {
    const index = tabs.findIndex((item) => item === tab)
    const next = e.key === 'ArrowRight' ? (index + 1) % tabs.length
      : e.key === 'ArrowLeft' ? (index + tabs.length - 1) % tabs.length
      : e.key === 'Home' ? 0 : e.key === 'End' ? tabs.length - 1 : -1
    if (next < 0) return
    e.preventDefault()
    e.stopPropagation()
    onselect(tabs[next])
    const tablist = e.currentTarget as HTMLElement
    tablist.querySelector<HTMLButtonElement>('#tab-' + tabs[next])?.focus()
  }

  $effect(() => {
    const video = p.video
    if (!playbackVisible) return
    if (!video) {
      frameSize = ''
      pipAvailable = pipActive = false
      return
    }
    const sync = () => {
      frameSize = video.videoWidth ? video.videoWidth + ' × ' + video.videoHeight : ''
      pipAvailable = !!video.videoWidth && supportsPiP(video)
      pipActive = inPiP(video)
    }
    sync()
    const events = [
      'loadedmetadata',
      'resize',
      'enterpictureinpicture',
      'leavepictureinpicture',
      'webkitpresentationmodechanged',
    ]
    for (const event of events) video.addEventListener(event, sync)
    return () => {
      for (const event of events) video.removeEventListener(event, sync)
    }
  })

  $effect(() => {
    const media = session
    loadedSession = ''
    chapters = []
    start = end = null
    error = ''
    if (!media) time = ''
  })

  $effect(() => {
    const id = session
    if (playbackVisible && id)
      untrack(() => {
        void prepare(id)
      })
  })

  $effect(() => {
    if (start !== null && end !== null && p.playing && p.ms >= end && p.video && !p.video.seeking) {
      p.seek(start)
    }
  })

  async function saveFrame() {
    const video = p.video
    const media = p.media
    if (!media || !video?.videoWidth) return
    if (!mockMode) {
      if (
        await backend.call('save_video_frame', {
          sessionId: session,
          ms: Math.round(Math.min(p.ms, Math.max(0, p.duration - 50))),
        })
      ) {
        p.toast(p.t('frameSaved'))
      }
      return
    }
    const frame = document.createElement('canvas')
    frame.width = video.videoWidth
    frame.height = video.videoHeight
    frame.getContext('2d')?.drawImage(video, 0, 0)
    const png = await new Promise<Blob | null>((resolve) => frame.toBlob(resolve, 'image/png'))
    if (!png) throw new Error(p.t('captureFailed'))
    const url = URL.createObjectURL(png)
    const a = document.createElement('a')
    a.href = url
    a.download =
      media.title.replace(/\.[^.]+$/, '') + '-' + clock(p.ms).replaceAll(':', '-') + '.png'
    a.click()
    setTimeout(() => URL.revokeObjectURL(url), 1000)
  }

  async function prepare(id: string) {
    time = clock(p.ms)
    error = ''
    if (loadedSession === id) return
    try {
      const items = await backend.call('list_chapters', { sessionId: id })
      if (session === id) {
        chapters = items
        loadedSession = id
      }
    } catch (e) {
      if (session === id) error = String(e)
    }
  }

  function jump() {
    const value = time.trim()
    const parts = value.split(':').map(Number)
    if (
      !/^\d+(?::\d{1,2}){0,2}(?:\.\d{1,3})?$/.test(value) ||
      parts.slice(1).some((n) => n >= 60)
    ) {
      error = p.t('invalidTime')
      return
    }
    const ms = parts.reduce((sum, value) => sum * 60 + value, 0) * 1000
    if (ms > p.duration || !Number.isFinite(ms)) {
      error = p.t('invalidTime')
      return
    }
    p.seek(ms)
    onclose()
  }

  function markEnd() {
    if (start === null || p.ms <= start) {
      error = p.t('afterStart')
      return
    }
    end = p.ms
    error = ''
  }

  async function regenerate() {
    onselect('subtitles')
    const request = p.startTranscription({ regenerate: true })
    await tick()
    document.querySelector<HTMLElement>('#pane-subtitles .subtitle-workspace')?.focus({ preventScroll: true })
    await request
  }
</script>

{#snippet regenerateButton()}
  <button class="regenerate-button" aria-label={p.t('regenerateSubtitles')}
    title={p.t('regenerateSubtitles')} onclick={regenerate}>
    <Icon name="refresh" size={15}/>{p.t('regenerate')}
  </button>
{/snippet}

{#snippet playbackControls()}
  <div class="playback-sheet" role="group" aria-label={p.t('playbackOptions')}>
    <label class="option-row"
      >{p.t('speed')}
      <select bind:value={p.rate} aria-label={p.t('speed')}>
        {#if !playbackSpeedPresets.includes(p.rate)}<option value={p.rate}>{p.rate}×</option>{/if}
        {#each playbackSpeedPresets as speed}<option value={speed}>{speed}×</option>{/each}
      </select>
    </label>
    <div class="option-row" role="group" aria-label={p.t('playlist')}>
      <button
        class="repeat-mode"
        aria-pressed={p.repeat !== 'off'}
        onclick={() => p.toggleRepeat()}
      >
        <Icon name={p.repeat === 'one' ? 'repeat-one' : 'repeat'} size={15} />{repeatLabel}
      </button>
      <div class="option-field">
        <button
          class="nav-file"
          title={p.t('previousFile')}
          aria-label={p.t('previousFile')}
          onclick={() => p.previous()}><Icon name="previous" size={15} /></button
        >
        <button
          class="nav-file"
          title={p.t('nextFile')}
          aria-label={p.t('nextFile')}
          disabled={p.queueIndex + 1 >= p.queue.length && p.repeat !== 'all'}
          onclick={() => p.next()}><Icon name="next" size={15} /></button
        >
      </div>
    </div>
    <form
      onsubmit={(event) => {
        event.preventDefault()
        jump()
      }}
    >
      <label for="jump-time">{p.t('jumpTime')}</label>
      <div class="option-field">
        <input id="jump-time" bind:value={time} placeholder="00:00" autocomplete="off" />
        <button type="submit">{p.t('jumpGo')}</button>
      </div>
    </form>
    <div class="option-row">
      <span>{p.t('loopSelection')}</span>
      <div class="option-field">
        <button
          title={p.t('setStart')}
          onclick={() => {
            start = p.ms
            end = null
            error = ''
          }}>{start === null ? p.t('setStart') : 'A ' + clock(start)}</button
        >
        <button disabled={start === null} onclick={markEnd}
          >{end === null ? p.t('setEnd') : 'B ' + clock(end)}</button
        >
        {#if start !== null}<button
            class="icon"
            title={p.t('clearLoop')}
            aria-label={p.t('clearLoop')}
            onclick={() => {
              start = end = null
              error = ''
            }}><Icon name="close" size={13} /></button
          >{/if}
      </div>
    </div>
    {#if frameSize}
      <div class="option-row">
        <span>{p.t('videoSize')}</span>
        <div class="option-field">
          <button aria-pressed={p.videoFit === 'contain'} onclick={() => (p.videoFit = 'contain')}
            >{p.t('fit')}</button
          >
          <button aria-pressed={p.videoFit === 'cover'} onclick={() => (p.videoFit = 'cover')}
            >{p.t('fill')}</button
          >
        </div>
      </div>
    {/if}
    {#if p.media?.audioTracks.length}
      <label class="option-row"
        >{p.t('track')}
        <select
          aria-label={p.t('trackChange')}
          value={p.track}
          onchange={(event) => p.act(() => p.setTrack(+event.currentTarget.value))}
        >
          {#each p.media.audioTracks as track}<option value={track.order}
              >{track.order + 1} · {track.languageTag || track.codecName}</option
            >{/each}
        </select>
      </label>
    {/if}
    {#if chapters.length}
      <label class="option-row"
        >{p.t('chapters')}
        <select
          onchange={(e) => {
            if (e.currentTarget.value) {
              p.seek(Number(e.currentTarget.value))
              onclose()
            }
          }}
        >
          <option value="">{p.t('chapters')} ({chapters.length})</option>
          {#each chapters as c, i}<option value={c.startMs}
              >{c.title || p.t('chapter') + ' ' + (i + 1)} · {clock(c.startMs)}</option
            >{/each}
        </select>
      </label>
    {/if}
    {#if frameSize}
      <div class="option-row">
        <small>{frameSize}</small>
        <button onclick={() => p.act(saveFrame)}>{p.t('saveFrame')}</button>
      </div>
    {/if}
    {#if pipAvailable}
      <button
        class="pip-toggle"
        aria-pressed={pipActive}
        onclick={() =>
          p.act(async () => {
            if (p.video) await togglePiP(p.video)
          })}><Icon name="pip" size={16} />{p.t('pictureInPicture')}</button
      >
    {/if}
    {#if error}<small role="alert">{error}</small>{/if}
  </div>
{/snippet}

{#if tab}
  <aside id="player-settings" class="inspector right-inspector" onfocusin={onfocus} transition:slidePanel={'right'} aria-label={p.t('playerSettings')}>
    <div class="panel-title">
      <h2>{p.t('playerSettings')}</h2>
      <button class="icon panel-close" title={p.t('close')} aria-label={p.t('close')}
        onclick={onclose}><Icon name="close" size={16}/></button>
    </div>
    <div class="inspector-tabs" role="tablist" tabindex="-1" aria-label={p.t('playerSettings')} onkeydown={tabKey}>
      {#each tabs as item}
        <button role="tab" id={'tab-' + item} aria-controls={'pane-' + item} aria-selected={tab === item}
          tabindex={tab === item ? 0 : -1} onclick={() => onselect(item)}>
          {p.t(item === 'playback' ? 'quick' : item)}
        </button>
      {/each}
    </div>
    {#if tab === 'subtitles'}<div class="inspector-page" id="pane-subtitles" role="tabpanel" aria-labelledby="tab-subtitles"><SubtitlePanel /></div>
    {:else}<div class="inspector-page" id="pane-playback" role="tabpanel" aria-labelledby="tab-playback">
      {@render playbackControls()}
    </div>{/if}
    {#if p.canRegenerate}
      <div class="regenerate-footer">{@render regenerateButton()}</div>
    {/if}
  </aside>
{:else if p.canRegenerate}
  <div class="regenerate-corner">{@render regenerateButton()}</div>
{/if}

<style>
  .regenerate-footer {
    display: flex;
    flex: none;
    justify-content: flex-end;
    padding: 8px var(--inspector-inset);
    border-top: 1px solid var(--hairline);
  }
  .regenerate-corner {
    position: absolute;
    right: max(14px, env(safe-area-inset-right));
    bottom: max(clamp(18px, 4.6vh, 38px), env(safe-area-inset-bottom));
    z-index: 5;
  }
  .regenerate-button {
    gap: 6px;
    min-height: 32px;
    padding: 5px 11px;
    font-size: 12px;
    font-weight: 600;
  }
  .regenerate-corner .regenerate-button {
    color: #f2f3f6;
    background: rgb(24 27 34 / 70%);
    border: 1px solid rgb(255 255 255 / 27%);
    border-radius: 18px;
    box-shadow: inset 0 1px rgb(255 255 255 / 9%), 0 2px 8px rgb(0 0 0 / 22%);
    -webkit-backdrop-filter: blur(14px) saturate(145%);
    backdrop-filter: blur(14px) saturate(145%);
  }
  .regenerate-corner .regenerate-button:hover {
    background: rgb(47 51 62 / 85%);
  }
  @media (max-width: 900px) {
    .regenerate-corner {
      bottom: var(--bar-clearance);
    }
  }
  @media (prefers-reduced-transparency: reduce) {
    .regenerate-corner .regenerate-button { background: #202228; backdrop-filter: none; -webkit-backdrop-filter: none; }
  }
</style>

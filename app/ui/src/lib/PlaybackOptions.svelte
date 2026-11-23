<script lang="ts">
  import { onMount, tick } from 'svelte'
  import { backend, mockMode, player as p } from './stores.svelte'
  import { clock } from './format'
  import Icon from './Icon.svelte'

  let open = $state(false)
  let root: HTMLDivElement
  let trigger: HTMLButtonElement
  let input = $state<HTMLInputElement>()
  let time = $state('')
  let error = $state('')
  let start = $state<number | null>(null)
  let end = $state<number | null>(null)
  let chapters = $state<{ startMs: number; title: string }[]>([])
  let loadedSession = ''
  let session = $derived(p.media?.sessionId ?? '')

  $effect(() => {
    const media = session
    loadedSession = ''
    chapters = []
    start = end = null
    open = false
    error = ''
    if (!media) time = ''
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
      if (await backend.call('save_video_frame', { sessionId: session, ms: Math.round(Math.min(p.ms, Math.max(0, p.duration - 50))) })) {
        p.toast(p.t('frameSaved'))
      }
      return
    }
    const frame = document.createElement('canvas')
    frame.width = video.videoWidth
    frame.height = video.videoHeight
    frame.getContext('2d')?.drawImage(video, 0, 0)
    const png = await new Promise<Blob | null>(resolve => frame.toBlob(resolve, 'image/png'))
    if (!png) throw new Error(p.t('captureFailed'))
    const url = URL.createObjectURL(png)
    const a = document.createElement('a')
    a.href = url
    a.download = media.title.replace(/\.[^.]+$/, '') + '-' + clock(p.ms).replaceAll(':', '-') + '.png'
    a.click()
    setTimeout(() => URL.revokeObjectURL(url), 1000)
  }

  async function show() {
    if (!session) return
    open = true
    time = clock(p.ms)
    error = ''
    await tick()
    input?.focus()
    input?.select()
    if (loadedSession === session) return
    const id = session
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

  onMount(() => {
    const listener = () => { void show() }
    window.addEventListener('hlp:playback-options', listener)
    return () => window.removeEventListener('hlp:playback-options', listener)
  })

  function close() {
    open = false
    trigger?.focus()
  }

  function jump() {
    const value = time.trim()
    const parts = value.split(':').map(Number)
    if (!/^\d+(?::\d{1,2}){0,2}(?:\.\d{1,3})?$/.test(value) || parts.slice(1).some(n => n >= 60)) {
      error = p.t('invalidTime')
      return
    }
    const ms = parts.reduce((sum, value) => sum * 60 + value, 0) * 1000
    if (ms > p.duration || !Number.isFinite(ms)) {
      error = p.t('invalidTime')
      return
    }
    p.seek(ms)
    close()
  }

  function markEnd() {
    if (start === null || p.ms <= start) {
      error = p.t('afterStart')
      return
    }
    end = p.ms
    error = ''
  }

  function key(event: KeyboardEvent) {
    if (open && event.key === 'Escape') {
      event.preventDefault()
      event.stopImmediatePropagation()
      close()
    }
  }
</script>

<svelte:window onkeydowncapture={key}
  onpointerdown={(e) => { if (open && !root?.contains(e.target as Node)) open = false }} />
<div class="playback-options" bind:this={root}>
  <button class="playback-trigger" bind:this={trigger} aria-expanded={open} aria-haspopup="dialog"
    title={p.t('playbackOptions')} aria-label={p.t('playbackOptions')}
    onclick={() => open ? close() : void show()}><Icon name="settings" size={16} />
    <span>{p.t('quick')}</span></button>
  {#if open}
    <div class="playback-popover" role="dialog" aria-label={p.t('playbackOptions')}>
      <div class="popover-title"><strong>{p.t('playbackOptions')}</strong>
        <button class="icon" aria-label={p.t('close')} onclick={close}><Icon name="close" size={15}/></button></div>
      <form onsubmit={(event) => { event.preventDefault(); jump() }}>
        <label for="jump-time">{p.t('jumpTime')}</label>
        <div class="option-field"><input id="jump-time" bind:this={input} bind:value={time} placeholder="00:00" autocomplete="off" />
          <button type="submit">{p.t('jumpGo')}</button></div>
      </form>
      <div class="option-row"><span>{p.t('loopSelection')}</span><div class="option-field">
        <button title={p.t('setStart')} onclick={() => { start = p.ms; end = null; error = '' }}>{start === null ? p.t('setStart') : 'A ' + clock(start)}</button>
        <button disabled={start === null} onclick={markEnd}>{end === null ? p.t('setEnd') : 'B ' + clock(end)}</button>
        {#if start !== null}<button class="icon" title={p.t('clearLoop')} aria-label={p.t('clearLoop')}
          onclick={() => { start = end = null; error = '' }}><Icon name="close" size={13}/></button>{/if}
      </div></div>
      {#if p.video?.videoWidth}
        <div class="option-row"><span>{p.t('videoSize')}</span><div class="option-field">
          <button aria-pressed={p.videoFit === 'contain'} onclick={() => p.videoFit = 'contain'}>{p.t('fit')}</button>
          <button aria-pressed={p.videoFit === 'cover'} onclick={() => p.videoFit = 'cover'}>{p.t('fill')}</button>
        </div></div>
      {/if}
      {#if p.media?.audioTracks.length}
        <label class="option-row">{p.t('track')}
          <select aria-label={p.t('trackChange')} value={p.track} onchange={(event) => p.act(() => p.setTrack(+event.currentTarget.value))}>
            {#each p.media.audioTracks as track}<option value={track.order}>{track.order+1} · {track.languageTag || track.codecName}</option>{/each}
          </select>
        </label>
      {/if}
      {#if chapters.length}
        <label class="option-row">{p.t('chapters')}
          <select onchange={(e) => { if (e.currentTarget.value) { p.seek(Number(e.currentTarget.value)); close() } }}>
            <option value="">{p.t('chapters')} ({chapters.length})</option>
            {#each chapters as c,i}<option value={c.startMs}>{c.title || p.t('chapter') + ' ' + (i+1)} · {clock(c.startMs)}</option>{/each}
          </select>
        </label>
      {/if}
      {#if p.video?.videoWidth}
        <div class="option-row"><small>{p.video.videoWidth} × {p.video.videoHeight}</small>
          <button onclick={() => p.act(saveFrame)}>{p.t('saveFrame')}</button></div>
      {/if}
      {#if error}<small role="alert">{error}</small>{/if}
    </div>
  {/if}
</div>

<style>
  .playback-options { position: relative; display: inline-flex }
  .playback-trigger { display: inline-flex; align-items:center; gap:7px }
  .playback-popover { position:absolute; z-index:20; left:0; bottom:calc(100% + 6px); width:min(310px,calc(100vw - 30px)); max-height:60vh; overflow:auto; padding:14px; background:white; color:#101112; border:1px solid #101112; box-shadow:3px 3px 0 rgb(16 17 18 / 14%); text-align:left }
  .popover-title, .option-row, form { display:flex; gap:8px; align-items:center; justify-content:space-between }
  .popover-title { margin-bottom:6px; padding-bottom:8px; border-bottom:1px solid #a9adb1 }
  .option-row, form { min-height:40px; font-weight:650; font-size:12px; }
  .option-field { display:flex; align-items:stretch }
  .playback-popover button, .playback-popover select, .playback-popover input { background:#fff; color:#101112; border:1px solid #a9adb1; border-radius:0; min-height:28px; font:inherit; padding:3px 7px; }
  .option-field > :global(* + *) { margin-left:-1px }
  .playback-popover button:hover { background:#eceef1 }
  .playback-popover button[aria-pressed=true] { background:var(--accent-soft,#e9eaff); border-bottom:2px solid var(--accent,#0003fe) }
  .playback-popover input { width:82px; text-align:center; font-variant-numeric:tabular-nums }
  .playback-popover select { max-width:168px }
  .playback-popover [role=alert] { color:#9b2c28 }
</style>

<script lang="ts">
  import { onMount, tick } from 'svelte'
  import { player as p, mockMode, backend } from './lib/stores.svelte'
  import Icon from './lib/Icon.svelte'
  import Player from './lib/Player.svelte'
  import PlaybackOptions from './lib/PlaybackOptions.svelte'
  import TranscriptPanel from './lib/TranscriptPanel.svelte'
  import RecentList from './lib/RecentList.svelte'
  import PlaylistPanel from './lib/PlaylistPanel.svelte'
  import SubtitlePanel from './lib/SubtitlePanel.svelte'
  import OpenUrlDialog from './lib/OpenUrlDialog.svelte'
  import ExportMenu from './lib/ExportMenu.svelte'
  import SetupScreen from './lib/SetupScreen.svelte'
  import SettingsDialog from './lib/SettingsDialog.svelte'
  import PlayerMenu from './lib/PlayerMenu.svelte'
  import './styles.css'
  import './lib/player-panels.css'

  type SettingsTab = 'subtitles' | 'playback' | 'transcript'
  type Panel = 'library' | 'playlist' | SettingsTab
  const tabs = ['subtitles', 'playback', 'transcript'] as const
  let settingsTab: SettingsTab = 'subtitles'
  const mac = /Mac/.test(navigator.platform)
  const nativeMac = !mockMode && mac
  let shell: HTMLElement
  let samplePath = $state<string | null>(mockMode ? 'fixture_ja.mp4' : null)
  let drawer = $state<Panel | null | undefined>(undefined)
  let fullscreenDrawer = $state<Panel | null>(null)
  let active: Panel | null = $derived(
    p.fullscreen && p.media ? fullscreenDrawer :
      (drawer === undefined ? (p.media && p.settings.showTranscriptPanel ? 'transcript' : null) : drawer),
  )
  let menuOpen = $state(false)
  const rightOpen = $derived(active === 'subtitles' || active === 'playback' || active === 'transcript')
  const leftOpen = $derived(active === 'playlist' || active === 'library')
  let drawerTrigger: HTMLElement | null = null
  let wasPlayingFile = false

  function focusMore() {
    shell?.querySelector<HTMLButtonElement>('.more-trigger')?.focus({ preventScroll: true })
  }
  function placeOverlays() {
    const trigger = shell?.querySelector('.more-trigger')
    if (!trigger) return
    const button = trigger.getBoundingClientRect()
    const host = shell.getBoundingClientRect()
    shell.style.setProperty('--menu-right', Math.max(8, host.right - button.right) + 'px')
    shell.style.setProperty('--menu-top', button.bottom - host.top + 6 + 'px')
    const bar = shell.querySelector('[data-transport]')?.getBoundingClientRect()
    shell.style.setProperty('--bar-clearance', Math.max(8, host.bottom - (bar?.top ?? host.bottom) + 8) + 'px')
  }
  function toggleMenu() {
    placeOverlays()
    menuOpen = !menuOpen
  }
  async function setPanel(id: Panel | null) {
    if (!p.media && id !== 'library' && id !== null) return
    menuOpen = false
    if (id === 'subtitles' || id === 'playback' || id === 'transcript') settingsTab = id
    if (p.fullscreen && p.media) fullscreenDrawer = id
    else {
      drawer = id
      if (p.media && p.settings.showTranscriptPanel !== (id === 'transcript'))
        await p.act(() => p.update({ showTranscriptPanel: id === 'transcript' }))
      if (drawer === id && (id === 'transcript' || id === null)) drawer = undefined
    }
  }
  function togglePanel(id: Panel) {
    if (active === id) closePanel()
    else {
      drawerTrigger = document.activeElement instanceof HTMLElement ? document.activeElement : null
      void setPanel(id).then(tick).then(() => shell.querySelector<HTMLButtonElement>('.inspector:not([hidden]) .panel-close')?.focus({ preventScroll: true }))
    }
  }
  function transportPanel(id: Panel) {
    if (id === 'subtitles') {
      if (rightOpen) closePanel()
      else togglePanel(settingsTab)
    } else togglePanel(id)
  }
  function tabKey(e: KeyboardEvent) {
    const index = tabs.findIndex((tab) => tab === active)
    const next = e.key === 'ArrowRight' ? (index + 1) % tabs.length
      : e.key === 'ArrowLeft' ? (index + tabs.length - 1) % tabs.length
      : e.key === 'Home' ? 0 : e.key === 'End' ? tabs.length - 1 : -1
    if (next < 0) return
    e.preventDefault()
    e.stopPropagation()
    void setPanel(tabs[next]).then(tick).then(() => {
      if (active === tabs[next]) shell.querySelector<HTMLButtonElement>('#tab-' + tabs[next])?.focus()
    })
  }
  function closePanel() {
    void setPanel(null)
    if (drawerTrigger?.isConnected) drawerTrigger.focus({ preventScroll: true })
    else focusMore()
  }
  function menuAction(action: string) {
    // Return focus before a picker, dialog or drawer takes it.
    menuOpen = false
    focusMore()
    switch (action) {
      case 'open': void p.choose(); break
      case 'folder': void p.chooseFolder(); break
      case 'url': p.dialog = 'url'; break
      case 'settings': p.dialog = 'settings'; break
      case 'sample': if (samplePath) void p.open(samplePath); break
      case 'library': togglePanel('library'); break
      case 'close': void setPanel(null).then(() => p.close()); break
    }
  }

  function key(e: KeyboardEvent) {
    if (e.defaultPrevented || e.isComposing || p.dialog) return
    if (e.key === 'Escape' && menuOpen) {
      e.preventDefault()
      menuOpen = false
      focusMore()
      return
    }
    if (e.key === 'Escape' && active) {
      e.preventDefault()
      closePanel()
      return
    }
    if (menuOpen) return
    const cmd = e.metaKey || e.ctrlKey,
      k = e.key.toLowerCase()
    if (cmd && ['o', 'l', 'e'].includes(k)) {
      if (!mockMode) return // The desktop menu handles these accelerators.
      e.preventDefault()
      if (k === 'o') {
        if (e.shiftKey) void p.chooseFolder()
        else void p.choose()
      } else if (k === 'l') p.dialog = 'url'
      else if (p.media) p.dialog = 'export'
      return
    }
    const el = e.target as HTMLElement | null
    if (
      cmd ||
      e.altKey ||
      !p.media ||
      el?.closest('input,textarea,select,[contenteditable=true]')
    )
      return
    if (k === ' ' && el?.closest('button')) return
    switch (k) {
      case 'escape':
        if (p.fullscreen) void p.act(() => p.full(false))
        else return
        break
      case ' ':
        p.toggle()
        break
      case 'arrowleft':
      case 'arrowright':
        p.seek(
          p.ms + (k === 'arrowleft' ? -1 : 1) * (e.shiftKey ? 30000 : 5000),
        )
        break
      case 'arrowup':
      case 'arrowdown':
        p.volume =
          Math.round(
            Math.max(
              0,
              Math.min(1, p.volume + (k === 'arrowup' ? 0.05 : -0.05)),
            ) * 100,
          ) / 100
        break
      case ',':
      case '.':
        p.jump(k === ',' ? -1 : 1)
        break
      case 'm':
        p.mute()
        break
      case 'f':
        void p.act(() => p.full(!p.fullscreen))
        break
      case 's':
        void p.act(() => p.cycle())
        break
      case 't':
        togglePanel('transcript')
        break
      case 'c':
        togglePanel('subtitles')
        break
      case 'l':
        togglePanel('playlist')
        break
      case 'p':
      case 'n':
        if (!e.shiftKey) return
        if (k === 'p') void p.previous()
        else void p.next()
        break
      case '[':
      case ']':
        p.rate = Math.max(
          0.25,
          Math.min(4, p.rate + (k === '[' ? -0.25 : 0.25)),
        )
        break
      default:
        return
    }
    e.preventDefault()
  }
  onMount(() => {
    document.documentElement.classList.toggle('native-mac', nativeMac)
    void p.act(() => p.init())
    if (!mockMode) void p.act(async () => { samplePath = await backend.call('get_sample_media') })
    function optionsFromMenu() {
      menuOpen = false
      if (p.media) void setPanel('playback')
    }
    window.addEventListener('hlp:playback-options', optionsFromMenu)
    function fromMenu(e: Event) {
      const id = (e as CustomEvent).detail
      if (['library', 'playlist', 'subtitles', 'transcript'].includes(id))
        setPanel(id)
    }
    window.addEventListener('player:panel', fromMenu)
    return () => {
      window.removeEventListener('player:panel', fromMenu)
      window.removeEventListener('hlp:playback-options', optionsFromMenu)
      p.dispose()
    }
  })
  $effect(() => {
    // Observe the dock itself because opening a sidebar changes the video width
    // without resizing the application window.
    p.media?.sessionId
    const observer = new ResizeObserver(placeOverlays)
    let mounted = true
    void tick().then(() => {
      if (!mounted) return
      observer.observe(shell)
      const bar = shell.querySelector('[data-transport]')
      if (bar) observer.observe(bar)
      const viewport = shell.querySelector('.movie-viewport')
      if (viewport) observer.observe(viewport)
      placeOverlays()
    })
    return () => { mounted = false; observer.disconnect() }
  })
  $effect(() => {
    document.documentElement.lang = p.locale
    document.documentElement.dataset.accentTheme = p.settings.accentTheme
    document.title = p.media ? p.media.title + ' · ' + p.t('app') : p.t('app')
    if (!!p.media !== wasPlayingFile) {
      wasPlayingFile = !!p.media
      drawer = undefined
      menuOpen = false
    }
    if (!p.fullscreen) fullscreenDrawer = null
    if (p.dialog) menuOpen = false
  })
</script>

<svelte:window onkeydown={key} />
<main bind:this={shell} class="app" class:watching={!!p.media}
  class:left-dock={leftOpen && !!p.media} class:right-dock={rightOpen} class:fullscreen={p.fullscreen}>
  {#if p.media}
    <div class="movie-viewport">
      <Player onpanel={transportPanel} onmore={toggleMenu} keepControls={menuOpen || !!active}
        subtitlesOpen={rightOpen} playlistOpen={active === 'playlist'} />
    </div>
  {:else}
    <section class="idle-stage" aria-label={p.t('player')}>
      <div class="welcome" class:has-history={p.recent.length > 0}>
        <div class="welcome-heading">
          <div class="idle-brand">
            <span class="hen-mark" aria-hidden="true"></span>
            <h1>{p.t('app')}</h1>
          </div>
          <div class="start-actions">
            <button class="open-file" onclick={() => p.choose()} title={p.t('open') + (mac ? ' (⌘O)' : ' (Ctrl+O)')}>
              <Icon name="folder" size={15}/>{p.t('open')}
            </button>
            <button class="open-file open-url" onclick={() => (p.dialog = 'url')} title={p.t('openUrlAction') + (mac ? ' (⌘L)' : ' (Ctrl+L)')}>
              <Icon name="link" size={15}/>{p.t('openUrlAction')}
            </button>
          </div>
        </div>
        {#if p.recent.length}
          <nav class="welcome-recent" aria-label={p.t('recent')}>
            {#each p.recent.slice(0, 4) as file (file.path)}
              <button title={file.path} onclick={() => p.open(file.path)}>
                <Icon name="film" size={15}/><span>{file.path === samplePath ? p.t('demo') : file.title}</span>
              </button>
            {/each}
          </nav>
        {/if}
      </div>
    </section>
  {/if}

  {#if !p.fullscreen}
    <header class="window-chrome" data-tauri-drag-region>
      {#if p.media}
        <span data-tauri-drag-region class="window-title">{p.media.sourcePath === samplePath ? p.t('demo') : p.media.title}</span>
      {:else}
        <button class="more-trigger window-more" aria-label={p.t('more')} title={p.t('more')}
          aria-haspopup="menu" aria-expanded={menuOpen} onclick={toggleMenu}><Icon name="more" size={19}/></button>
      {/if}
    </header>
  {/if}

  {#if leftOpen && active}
    <aside class="inspector left-inspector" aria-label={p.t(active === 'library' ? 'recent' : 'playlist')}>
      <div class="panel-title">
        <h2>{p.t(active === 'library' ? 'recent' : 'playlist')}</h2>
        <button class="icon panel-close" title={p.t('close')} aria-label={p.t('close')}
          onclick={closePanel}><Icon name="close" size={16}/></button>
      </div>
      {#if active === 'playlist'}<PlaylistPanel />
      {:else}<RecentList {samplePath} onselect={closePanel} />{/if}
    </aside>
  {/if}
  {#if p.media}
    <aside class="inspector right-inspector" hidden={!rightOpen} aria-label={p.t('playerSettings')}>
      <div class="panel-title">
        <h2>{p.t('playerSettings')}</h2>
        <button class="icon panel-close" title={p.t('close')} aria-label={p.t('close')}
          onclick={closePanel}><Icon name="close" size={16}/></button>
      </div>
      <div class="inspector-tabs" role="tablist" tabindex="-1" aria-label={p.t('playerSettings')} onkeydown={tabKey}>
        {#each tabs as tab}
          <button role="tab" id={'tab-' + tab} aria-controls={'pane-' + tab} aria-selected={active === tab}
            tabindex={active === tab ? 0 : -1} onclick={() => setPanel(tab)}>
            {p.t(tab === 'playback' ? 'quick' : tab)}
          </button>
        {/each}
      </div>
      {#if active === 'subtitles'}<div class="inspector-page" id="pane-subtitles" role="tabpanel" aria-labelledby="tab-subtitles"><SubtitlePanel /></div>{/if}
      <div class="inspector-page" hidden={active !== 'playback'} id="pane-playback" role="tabpanel" aria-labelledby="tab-playback">
        <PlaybackOptions open={active === 'playback'} onreturnfocus={closePanel} />
      </div>
      {#if active === 'transcript'}<div class="inspector-page" id="pane-transcript" role="tabpanel" aria-labelledby="tab-transcript"><TranscriptPanel /></div>{/if}
    </aside>
  {/if}
  {#if menuOpen}
    <PlayerMenu hasMedia={!!p.media} canSample={!!samplePath} onaction={menuAction}
      ondismiss={(restore) => { menuOpen = false; if (restore) focusMore() }} />
  {/if}

  {#if (p.error || p.notice) && !p.dialog}<div
      class="toast"
      role={p.error ? 'alert' : 'status'}
    >
      <Icon name={p.error ? 'alert' : 'check'} size={16} />
      <span>{p.error || p.notice}</span>
      <button
        class="icon"
        aria-label={p.t('dismiss')}
        onclick={() => {
          p.error = ''
          p.notice = ''
        }}><Icon name="close" size={14} /></button
      >
    </div>{/if}
  {#if p.dialog === 'url'}<OpenUrlDialog
    />{:else if p.dialog === 'settings'}<SettingsDialog />
  {:else if p.dialog === 'setup'}<SetupScreen
    />{:else if p.dialog === 'export'}<ExportMenu />{/if}
</main>

<script lang="ts">
  import { onMount, tick } from 'svelte'
  import { getCurrentWindow } from '@tauri-apps/api/window'
  import { enableWindowDragging } from './lib/window-drag'
  import { player as p, mockMode, backend } from './lib/stores.svelte'
  import Icon from './lib/Icon.svelte'
  import Player from './lib/Player.svelte'
  import PlayerSettings from './lib/PlayerSettings.svelte'
  import { slidePanel } from './lib/panel-transition'
  import RecentList from './lib/RecentList.svelte'
  import PlaylistPanel from './lib/PlaylistPanel.svelte'
  import OpenUrlDialog from './lib/OpenUrlDialog.svelte'
  import OnlineSubtitleDialog from './lib/OnlineSubtitleDialog.svelte'
  import ExportMenu from './lib/ExportMenu.svelte'
  import SetupScreen from './lib/SetupScreen.svelte'
  import SettingsDialog from './lib/SettingsDialog.svelte'
  import PlayerMenu from './lib/PlayerMenu.svelte'
  import './styles.css'
  import './lib/player-panels.css'

  type SettingsTab = 'subtitles' | 'playback'
  type LeftPanel = 'library' | 'playlist'
  type Panel = LeftPanel | SettingsTab | 'transcript'
  type Side = 'left' | 'right'
  const mac = /Mac/.test(navigator.platform)
  const nativeMac = !mockMode && mac
  let shell: HTMLElement
  let samplePath = $state<string | null>(mockMode ? 'fixture_ja.mp4' : null)
  let leftPanel = $state<LeftPanel | null>(null)
  let lastSettingsTab: SettingsTab = 'subtitles'
  let playbackOpen = $state(false)
  let subtitleOverride = $state<boolean | null>(null)
  let rightTab: SettingsTab | null = $derived(!p.media ? null :
    (subtitleOverride ?? p.settings.showTranscriptPanel) ? 'subtitles' : playbackOpen ? 'playback' : null)
  let preferenceWrite = Promise.resolve()
  let preferenceRevision = 0
  let menuOpen = $state(false)
  let lastSide = $state<Side>('right')
  let dismissSide: Side = $derived(leftPanel && (!rightTab || lastSide === 'left') ? 'left' : 'right')
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
  function setRight(tab: SettingsTab | null) {
    if (tab) lastSettingsTab = tab
    playbackOpen = tab === 'playback'
    const showSubtitles = tab === 'subtitles'
    subtitleOverride = showSubtitles
    // Keep the visible tab responsive while serializing preference changes.
    const revision = ++preferenceRevision
    preferenceWrite = preferenceWrite.then(async () => {
      if (p.settings.showTranscriptPanel !== showSubtitles)
        await p.act(() => p.update({ showTranscriptPanel: showSubtitles }))
      if (revision === preferenceRevision) subtitleOverride = null
    })
  }
  function showPanel(id: Panel, toggle = false) {
    if (!p.media && id !== 'library') return
    menuOpen = false
    const left = id === 'library' || id === 'playlist'
    const side = left ? 'left' : 'right'
    const tab = id === 'transcript' ? 'subtitles' : id
    if (toggle && (left ? leftPanel === id : rightTab === tab)) {
      closePanel(side)
      return
    }
    lastSide = side
    if (left) leftPanel = id
    else setRight(id === 'transcript' ? 'subtitles' : id)
    void tick().then(() => {
      const selector = id === 'transcript'
        ? '.right-inspector .subtitle-workspace'
        : '.' + side + '-inspector .panel-close'
      const target = shell.querySelector<HTMLElement>(selector)
        ?? shell.querySelector<HTMLElement>('.' + side + '-inspector .panel-close')
      target?.focus({ preventScroll: true })
    })
  }
  function transportPanel(id: Panel) {
    if (id === 'subtitles') {
      if (rightTab) closePanel('right')
      else showPanel(lastSettingsTab)
    } else showPanel(id, true)
  }
  function closePanel(side: Side) {
    if (side === 'left') leftPanel = null
    else setRight(null)
    const selector = side === 'right' ? '.settings-trigger' : p.media ? '.playlist-trigger' : '.more-trigger'
    shell?.querySelector<HTMLButtonElement>(selector)?.focus({ preventScroll: true })
    lastSide = side === 'left' ? 'right' : 'left'
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
      case 'library': showPanel('library', true); break
      case 'close': void p.close(); break
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
    if (e.key === 'Escape' && (leftPanel || rightTab)) {
      e.preventDefault()
      const focused = (e.target as Element | null)?.closest?.('.inspector')
      const side = focused?.classList.contains('left-inspector') ? 'left'
        : focused?.classList.contains('right-inspector') ? 'right'
        : dismissSide
      closePanel(side)
      return
    }
    if (menuOpen) return
    const cmd = e.metaKey || e.ctrlKey,
      k = e.key.toLowerCase()
    if (cmd && (['o', 'l', 'e'].includes(k) || (k === 'd' && e.shiftKey))) {
      if (!mockMode) return // The desktop menu handles these accelerators.
      e.preventDefault()
      if (k === 'o') {
        if (e.shiftKey) void p.chooseFolder()
        else void p.choose()
      } else if (k === 'l') p.dialog = 'url'
      else if (p.media) p.dialog = k === 'd' ? 'online-subtitles' : 'export'
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
        if (!e.repeat) void p.act(() => p.full(!p.fullscreen))
        break
      case 's':
        void p.act(() => p.cycle())
        break
      case 't':
        showPanel('transcript', true)
        break
      case 'c':
        showPanel('subtitles', true)
        break
      case 'l':
        showPanel('playlist', true)
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
          Math.min(5, p.rate + (k === '[' ? -0.25 : 0.25)),
        )
        break
      default:
        return
    }
    e.preventDefault()
  }
  onMount(() => {
    document.documentElement.classList.toggle('native-mac', nativeMac)
    const stopDragging = !mockMode && enableWindowDragging(shell,
      () => !p.fullscreen && !p.dialog && !menuOpen,
      () => void p.act(() => getCurrentWindow().startDragging()))
    void p.act(() => p.init())
    if (!mockMode) void p.act(async () => { samplePath = await backend.call('get_sample_media') })
    function optionsFromMenu() {
      menuOpen = false
      showPanel('playback')
    }
    window.addEventListener('hlp:playback-options', optionsFromMenu)
    function fromMenu(e: Event) {
      const id = (e as CustomEvent).detail
      if (['library', 'playlist', 'subtitles', 'transcript'].includes(id))
        showPanel(id)
    }
    window.addEventListener('player:panel', fromMenu)
    return () => {
      if (stopDragging) stopDragging()
      window.removeEventListener('player:panel', fromMenu)
      window.removeEventListener('hlp:playback-options', optionsFromMenu)
      p.dispose()
    }
  })
  $effect(() => {
    // Keep the menu and subtitle clearance aligned to the transport after resize.
    p.media?.sessionId
    const observer = new ResizeObserver(placeOverlays)
    let mounted = true
    void tick().then(() => {
      if (!mounted) return
      observer.observe(shell)
      const bar = shell.querySelector('[data-transport]')
      if (bar) observer.observe(bar)
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
      if (!p.media) {
        leftPanel = null
        playbackOpen = false
      }
      menuOpen = false
    }
    if (p.dialog) menuOpen = false
  })
</script>

<svelte:window onkeydown={key} />
<main bind:this={shell} class="app" class:desktop={!mockMode} class:watching={!!p.media}
  class:fullscreen={p.fullscreen}>
  {#if p.media}
    <Player onpanel={transportPanel} onmore={toggleMenu} keepControls={menuOpen || !!leftPanel || !!rightTab}
      subtitlesOpen={!!rightTab} playlistOpen={leftPanel === 'playlist'} />
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

  {#if leftPanel}
    <aside id="playlist-sidebar" class="inspector left-inspector" transition:slidePanel={'left'} onfocusin={() => (lastSide = 'left')} aria-label={p.t(leftPanel === 'library' ? 'recent' : 'playlist')}>
      <div class="panel-title">
        <h2>{p.t(leftPanel === 'library' ? 'recent' : 'playlist')}</h2>
        <button class="icon panel-close" title={p.t('close')} aria-label={p.t('close')}
          onclick={() => closePanel('left')}><Icon name="close" size={16}/></button>
      </div>
      {#if leftPanel === 'playlist'}<PlaylistPanel />
      {:else}<RecentList {samplePath} onselect={() => closePanel('left')} />{/if}
    </aside>
  {/if}
  {#if p.media}
    {#if leftPanel || rightTab}
      <button class="panel-dismiss-surface" tabindex="-1" aria-label={p.t('close')}
        aria-controls={dismissSide === 'left' ? 'playlist-sidebar' : 'player-settings'}
        onclick={() => closePanel(dismissSide)}></button>
    {/if}
    <PlayerSettings tab={rightTab} onselect={setRight} onfocus={() => (lastSide = 'right')}
      onclose={() => closePanel('right')} />
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
  {:else if p.dialog === 'online-subtitles'}<OnlineSubtitleDialog />
  {:else if p.dialog === 'setup'}<SetupScreen
    />{:else if p.dialog === 'export'}<ExportMenu />{/if}
</main>

<style>
  .desktop :global(:is(.idle-stage, .video-stage, .panel-title)) {
    -webkit-user-select: none;
    user-select: none;
  }
</style>

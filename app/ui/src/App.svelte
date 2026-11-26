<script lang="ts">
  import { onMount, tick } from 'svelte'
  import { MediaQuery } from 'svelte/reactivity'
  import { getCurrentWindow } from '@tauri-apps/api/window'
  import { enableWindowDragging } from './lib/window-drag'
  import { player as p, mockMode, appUpdates, backend } from './lib/stores.svelte'
  import { observeSystemAccent } from './lib/system-accent'
  import { updateInterval } from './lib/app-updates.svelte'
  import { applyNativeMenu } from './lib/i18n/native-menu'
  import { loadMessages, resolveUiLanguage } from './lib/i18n'
  import Icon from './lib/Icon.svelte'
  import Player from './lib/Player.svelte'
  import PlayerSettings from './lib/PlayerSettings.svelte'
  import { finishPanelSlide } from './lib/panel-transition'
  import RecentList from './lib/RecentList.svelte'
  import WelcomeHistory from './lib/WelcomeHistory.svelte'
  import PlaylistPanel from './lib/PlaylistPanel.svelte'
  import OpenUrlDialog from './lib/OpenUrlDialog.svelte'
  import OnlineSubtitleDialog from './lib/OnlineSubtitleDialog.svelte'
  import ExportMenu from './lib/ExportMenu.svelte'
  import Dialog from './lib/Dialog.svelte'
  import SetupScreen from './lib/SetupScreen.svelte'
  import SettingsDialog from './lib/SettingsDialog.svelte'
  import './styles.css'
  import './lib/player-panels.css'

  type LeftPanel = 'library' | 'playlist'
  type Panel = LeftPanel | 'subtitles' | 'transcript'
  type Side = 'left' | 'right'
  async function deleteSubtitles() {
    if (await p.deleteGeneratedSubtitles()) {
      await tick()
      document.querySelector<HTMLElement>('#player-settings .subtitle-workspace')?.focus({ preventScroll: true })
    }
  }
  function preferredLanguages() {
    return navigator.languages.length ? [...navigator.languages] : [navigator.language]
  }
  let systemLanguages = $state.raw(preferredLanguages())
  $effect(() => {
    const locale = resolveUiLanguage(p.settings.uiLanguage, systemLanguages)
    let current = true
    void loadMessages(locale).then(messages => {
      if (current) p.interfaceCopy = { locale, messages }
    }).catch(error => { if (current) p.message(error) })
    return () => { current = false }
  })

  const mac = /Mac/.test(navigator.platform)
  const nativeMac = !mockMode && mac
  let initialized = $state(false)
  let shell: HTMLElement
  let welcomeHistory = $state<ReturnType<typeof WelcomeHistory>>()
  let leftPanel = $state<LeftPanel | null>(null)
  let subtitleOverride = $state<boolean | null>(null)
  let subtitlesOpen = $derived(!!p.media && (subtitleOverride ?? p.settings.showTranscriptPanel))
  const compactPlayer = new MediaQuery('(max-width: 890px), (max-height: 405px)')
  let panelOpen = $derived(!!leftPanel || subtitlesOpen)
  let preferenceWrite = Promise.resolve()
  let preferenceRevision = 0
  let lastSide = $state<Side>('right')
  let dismissSide: Side = $derived(leftPanel && (!subtitlesOpen || lastSide === 'left') ? 'left' : 'right')
  let wasPlayingFile = false

  function placeOverlays() {
    const bar = shell.querySelector('[data-transport-stack]')?.getBoundingClientRect()
    const host = shell.getBoundingClientRect()
    shell.style.setProperty('--bar-clearance', Math.max(8, host.bottom - (bar?.top ?? host.bottom) + 8) + 'px')
  }
  function setSubtitlesOpen(showSubtitles: boolean) {
    subtitleOverride = showSubtitles
    // Reflect the toggle immediately while serializing saved preferences.
    const revision = ++preferenceRevision
    preferenceWrite = preferenceWrite.then(async () => {
      if (p.settings.showTranscriptPanel !== showSubtitles)
        await p.act(() => p.update({ showTranscriptPanel: showSubtitles }))
      if (revision === preferenceRevision) subtitleOverride = null
    })
  }
  function showPanel(id: Panel, toggle = false) {
    if (!p.media) {
      if (id === 'library') void welcomeHistory?.showAll()
      return
    }
    const left = id === 'playlist' || id === 'library'
    const side = left ? 'left' : 'right'
    if (toggle && (left ? leftPanel === id : subtitlesOpen)) {
      closePanel(side)
      return
    }
    lastSide = side
    if (left) {
      if (compactPlayer.current && subtitlesOpen) setSubtitlesOpen(false)
      leftPanel = id
    } else {
      if (compactPlayer.current) leftPanel = null
      setSubtitlesOpen(true)
    }
    void tick().then(() => {
      const selector = id === 'transcript'
        ? '.right-inspector .subtitle-workspace'
        : '.' + side + '-inspector .panel-close'
      const target = shell.querySelector<HTMLElement>(selector)
        ?? shell.querySelector<HTMLElement>('.' + side + '-inspector .panel-close')
      target?.focus({ preventScroll: true })
    })
  }
  $effect(() => {
    if (compactPlayer.current && leftPanel && subtitlesOpen) {
      if (lastSide === 'right') leftPanel = null
      else setSubtitlesOpen(false)
    }
  })
  function focusPanelButton(side: Side) {
    const selector = side === 'right' ? '.settings-trigger' : '.playlist-trigger'
    shell?.querySelector<HTMLButtonElement>(selector)?.focus({ preventScroll: true })
  }
  function dismissPanels() {
    const restore = dismissSide
    leftPanel = null
    if (subtitlesOpen) setSubtitlesOpen(false)
    focusPanelButton(restore)
  }
  function trackPanelInteraction(event: PointerEvent) {
    if (!leftPanel || !subtitlesOpen) return
    const panel = event.target instanceof Element ? event.target.closest('.inspector') : null
    if (panel) lastSide = panel.classList.contains('left-inspector') ? 'left' : 'right'
  }
  function closePanel(side: Side) {
    if (side === 'left') leftPanel = null
    else setSubtitlesOpen(false)
    focusPanelButton(side)
    lastSide = side === 'left' ? 'right' : 'left'
  }
  function key(e: KeyboardEvent) {
    if (e.defaultPrevented || e.isComposing || p.dialog) return
    if (e.key === 'Escape' && (leftPanel || subtitlesOpen)) {
      e.preventDefault()
      closePanel(dismissSide)
      return
    }
    const cmd = e.metaKey || e.ctrlKey,
      k = e.key.toLowerCase()
    if (cmd && e.shiftKey && !e.altKey && k === 'd') {
      // Some webviews forward the keystroke instead of invoking the native menu.
      e.preventDefault()
      p.findOnlineSubtitles()
      return
    }
    if (cmd && ['o', 'l', 'e', ',', 'w'].includes(k)) {
      if (!mockMode) return // The desktop menu handles these accelerators.
      e.preventDefault()
      if (k === 'o') {
        if (e.shiftKey) void p.chooseFolder()
        else void p.chooseMedia()
      } else if (k === ',') p.dialog = 'settings'
      else if (k === 'w') { if (p.media) void p.close() }
      else if (k === 'l') p.dialog = 'url'
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
    const stopAccent = mac ? observeSystemAccent(backend) : undefined
    const stopDragging = !mockMode && enableWindowDragging(shell,
      () => !p.fullscreen && !p.dialog,
      () => void p.act(() => getCurrentWindow().startDragging()))
    void p.act(() => p.init()).then(() => { initialized = true })
    function fromMenu(e: Event) {
      const id = (e as CustomEvent).detail
      if (['library', 'playlist', 'subtitles', 'transcript'].includes(id))
        showPanel(id)
    }
    window.addEventListener('player:panel', fromMenu)
    return () => {
      if (stopDragging) stopDragging()
      stopAccent?.()
      window.removeEventListener('player:panel', fromMenu)
      p.dispose()
    }
  })
  $effect(() => {
    if (!mockMode && initialized) void applyNativeMenu(p.strings, nativeMac && p.fullscreen, !!p.media).catch(p.message.bind(p))
  })
  $effect(() => {
    if (!p.settings.autoDownloadUpdates || p.media) return
    const eligible = () => p.settings.autoDownloadUpdates && !p.media
    const check = async () => {
      const version = await appUpdates.automatic(eligible)
      if (version && eligible() && !p.dialog) p.toast(p.t('readyToRestartNotice').replace('{version}', version))
    }
    let daily: number
    const initial = window.setTimeout(() => {
      void check()
      daily = window.setInterval(check, updateInterval)
    }, 2500)
    return () => { window.clearTimeout(initial); window.clearInterval(daily) }
  })
  $effect(() => {
    // Keep subtitle clearance aligned to the transport after resize.
    p.media?.sessionId
    const observer = new ResizeObserver(placeOverlays)
    let mounted = true
    void tick().then(() => {
      if (!mounted) return
      observer.observe(shell)
      const bar = shell.querySelector('[data-transport-stack]')
      if (bar) observer.observe(bar)
      placeOverlays()
    })
    return () => { mounted = false; observer.disconnect() }
  })
  $effect(() => {
    document.documentElement.lang = p.locale === 'tl' ? 'fil' : p.locale
    document.documentElement.dir = ['ar', 'fa'].includes(p.locale) ? 'rtl' : 'ltr'
    document.documentElement.dataset.accentTheme = p.settings.accentTheme
    document.title = p.media ? p.media.title + ' · ' + p.t('app') : p.t('app')
    if (!!p.media !== wasPlayingFile) {
      wasPlayingFile = !!p.media
      if (!p.media) leftPanel = null
    }
  })
</script>

<svelte:window onpointerdowncapture={trackPanelInteraction} onkeydown={key} onlanguagechange={() => systemLanguages = preferredLanguages()} />
<main bind:this={shell} class="app" class:desktop={!mockMode} class:watching={!!p.media}
  class:fullscreen={p.fullscreen} class:compact-player={compactPlayer.current}
  class:panel-open={panelOpen} class:left-open={!!leftPanel} class:right-open={subtitlesOpen}>
  {#if p.media}
    <Player {panelOpen}>
      <button class="float-button panel-toggle playlist-trigger" title={p.t('playlist')} aria-label={p.t('playlist')}
        aria-expanded={leftPanel === 'playlist'} aria-controls={leftPanel === 'playlist' ? 'playlist-sidebar' : undefined}
        onclick={() => showPanel('playlist', true)}><Icon name="sidebar" size={15} /><span>{p.t('playlist')}</span></button>
      <button class="float-button panel-toggle settings-trigger" title={p.t('subtitles')} aria-label={p.t('subtitles')}
        aria-expanded={subtitlesOpen} aria-controls={subtitlesOpen ? 'player-settings' : undefined}
        onclick={() => showPanel('subtitles', true)}><Icon name="cc" size={15} /><span>{p.t('subtitles')}</span></button>
    </Player>
  {:else}
    <section class="idle-stage" aria-label={p.t('player')}>
      <div class="welcome">
        <div class="welcome-heading">
          <div class="idle-brand">
            <span class="hen-mark" aria-hidden="true"></span>
            <h1>{p.t('app')}</h1>
          </div>
          <div class="start-actions">
            <button class="open-file" onclick={() => p.chooseMedia()} title={p.t('open') + ' / ' + p.t('openFolder') + (mac ? ' (⌘O)' : '')}>
              <Icon name="folder" size={15}/><span>{p.t('open')} / {p.t('openFolder')}</span>
            </button>
            <button class="open-file open-url" onclick={() => (p.dialog = 'url')} title={p.t('openUrlAction') + (mac ? ' (⌘L)' : ' (Ctrl+L)')}>
              <Icon name="link" size={15}/>{p.t('openUrlAction')}
            </button>
          </div>
        </div>
        <WelcomeHistory bind:this={welcomeHistory} />
      </div>
    </section>
  {/if}

  <header class="window-chrome" data-tauri-drag-region>
    {#if p.media}
      <span data-tauri-drag-region class="window-title" dir="auto">{p.media.title}</span>
    {:else}
      <div class="welcome-tools" dir="ltr">
        <button class="welcome-settings" aria-label={p.t('settings')} title={p.t('settings')}
          onclick={() => (p.dialog = 'settings')}><Icon name="gear" size={19}/></button>
      </div>
    {/if}
  </header>

  {#if leftPanel}
    <aside id="playlist-sidebar" class="inspector left-inspector" out:finishPanelSlide onfocusin={() => (lastSide = 'left')} aria-label={p.t(leftPanel === 'library' ? 'recent' : 'playlist')}>
      <div class="panel-title">
        <h2>{p.t(leftPanel === 'library' ? 'recent' : 'playlist')}</h2>
        <button class="icon panel-close" title={p.t('close')} aria-label={p.t('close')}
          onclick={() => closePanel('left')}><Icon name="close" size={16}/></button>
      </div>
      {#if leftPanel === 'playlist'}<PlaylistPanel onselect={() => { if (compactPlayer.current) closePanel('left') }} />
      {:else}<RecentList onselect={() => closePanel('left')} />{/if}
    </aside>
  {/if}
  {#if p.media}
    {#if panelOpen}
      <button class="panel-dismiss-surface" data-drag-surface tabindex="-1" aria-label={p.t('close')}
        onclick={dismissPanels}></button>
    {/if}
    <PlayerSettings open={subtitlesOpen} onfocus={() => (lastSide = 'right')}
      onclose={() => closePanel('right')} />
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
  {#if p.dialog === 'open-media'}
    <Dialog title={p.t('open') + ' / ' + p.t('openFolder')} icon="folder">
      <div class="dialog-actions">
        <button onclick={() => { p.dialog = null; void p.choose() }}>{p.t('open')}</button>
        <button onclick={() => { p.dialog = null; void p.chooseFolder() }}>{p.t('openFolder')}</button>
      </div>
    </Dialog>
  {:else if p.dialog === 'url'}<OpenUrlDialog
    />{:else if p.dialog === 'settings'}<SettingsDialog systemAccentAvailable={mac} />
  {:else if p.dialog === 'online-subtitles'}<OnlineSubtitleDialog />
  {:else if p.dialog === 'setup'}<SetupScreen
    />{:else if p.dialog === 'export'}<ExportMenu />
  {:else if p.dialog && typeof p.dialog === 'object' && p.dialog.kind === 'delete-subtitles'}
    <Dialog title={p.t('deleteGeneratedSubtitles')} icon="trash">
      <p class="help">{p.t('deleteSubtitlesPrompt')}</p>
      <div class="dialog-actions">
        <button disabled={p.deletingSubtitles} onclick={() => (p.dialog = null)}>{p.t('cancel')}</button>
        <button class="confirm-subtitle-deletion" disabled={p.deletingSubtitles}
          onclick={deleteSubtitles}>{p.t('deleteAction')}</button>
      </div>
    </Dialog>
  {/if}
</main>

<style>
  .confirm-subtitle-deletion { color: #ffb0aa; }
  .desktop :global(:is(.idle-stage, .video-stage, .panel-title)) {
    -webkit-user-select: none;
    user-select: none;
  }
</style>

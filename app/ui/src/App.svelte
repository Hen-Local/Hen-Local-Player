<script lang="ts">
  import { onMount, tick } from 'svelte'
  import { player as p, mockMode } from './lib/stores.svelte'
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
  import './styles.css'

  type Panel = 'library' | 'playlist' | 'subtitles' | 'transcript'
  const tools = [
    { id: 'subtitles', icon: 'cc' },
    { id: 'transcript', icon: 'transcript' },
    { id: 'playlist', icon: 'list' },
  ] as const
  let drawer = $state<Exclude<Panel, 'transcript'> | null>(null)
  let fullscreenDrawer = $state<Panel | null>(null)
  let active: Panel | null = $derived(
    p.media
      ? p.fullscreen
        ? fullscreenDrawer
        : (drawer ?? (p.settings.showTranscriptPanel ? 'transcript' : null))
      : null,
  )
  let drawerTrigger: HTMLElement | null = null
  let closeButton = $state<HTMLButtonElement>()

  async function setPanel(id: Panel | null) {
    if (!p.media) return
    if (p.fullscreen) {
      fullscreenDrawer = id
      return
    }
    drawer = id === 'transcript' ? null : id
    if (p.settings.showTranscriptPanel !== (id === 'transcript'))
      await p.act(() => p.update({ showTranscriptPanel: id === 'transcript' }))
  }
  function togglePanel(id: Panel) {
    if (active === id) {
      closePanel()
    } else {
      drawerTrigger =
        document.activeElement instanceof HTMLElement
          ? document.activeElement
          : null
      void setPanel(id)
        .then(tick)
        .then(() => closeButton?.focus({ preventScroll: true }))
    }
  }
  function closePanel() {
    setPanel(null)
    drawerTrigger?.focus({ preventScroll: true })
  }
  async function closeFile() {
    await setPanel(null)
    await p.close()
  }

  function key(e: KeyboardEvent) {
    if (e.defaultPrevented || e.isComposing || p.dialog) return
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
        if (active) closePanel()
        else if (p.fullscreen) void p.act(() => p.full(false))
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
    void p.act(() => p.init())
    function fromMenu(e: Event) {
      const id = (e as CustomEvent).detail
      if (['library', 'playlist', 'subtitles', 'transcript'].includes(id))
        setPanel(id)
    }
    window.addEventListener('player:panel', fromMenu)
    return () => {
      window.removeEventListener('player:panel', fromMenu)
      p.dispose()
    }
  })
  $effect(() => {
    document.documentElement.lang = p.locale
    document.documentElement.dataset.accentTheme = p.settings.accentTheme
    document.title = p.media ? p.media.title + ' · ' + p.t('app') : p.t('app')
    if (!p.media) drawer = null
    if (!p.fullscreen) fullscreenDrawer = null
  })
</script>

<svelte:window onkeydown={key} />
<main class="app" class:fullscreen={p.fullscreen}>
  <header class="top">
    <div class="brand">
      <span class="brand-mark" aria-hidden="true"></span>
      <div class="brand-copy">
        <h1>{p.t('app')}</h1>
        <p>
          {mockMode && p.queue[p.queueIndex]?.path === 'fixture_ja.mp4'
            ? p.t('demo')
            : (p.media?.title ?? p.t('local'))}
        </p>
      </div>
    </div>
    <nav class="top-actions" aria-label={p.t('player')}>
      <button
        class="outline primary"
        title={p.t('open')}
        aria-label={p.t('open')}
        onclick={() => p.choose()}
      >
        <Icon name="folder" size={16} /><span>{p.t('open')}</span>
      </button>
      <button
        class="outline"
        title={p.t('link')}
        aria-label={p.t('link')}
        onclick={() => (p.dialog = 'url')}
      >
        <Icon name="link" size={16} /><span>{p.t('link')}</span>
      </button>
      <button
        class="square settings-button"
        title={p.t('settings')}
        aria-label={p.t('settings')}
        onclick={() => (p.dialog = 'settings')}
      >
        <Icon name="settings" size={20} />
      </button>
    </nav>
  </header>

  {#if p.media}
    <section
      class="playback-workspace"
      class:with-drawer={active !== null}
      aria-label={p.t('player')}
    >
      <Player />
      {#if active}
        <aside class="inspector" aria-label={p.t(active)}>
          <div class="panel-title">
            <h2>{p.t(active)}</h2>
            <button
              bind:this={closeButton}
              class="icon"
              title={p.t('close')}
              aria-label={p.t('close')}
              onclick={closePanel}><Icon name="close" size={17} /></button
            >
          </div>
          {#if active === 'transcript'}<TranscriptPanel />
          {:else if active === 'playlist'}<PlaylistPanel />
          {:else if active === 'subtitles'}<SubtitlePanel />
          {:else}
            <RecentList onselect={closePanel} />
            <div class="drawer-footer">
              <button class="outline" onclick={() => p.chooseFolder()}
                ><Icon name="folder" size={15} />{p.t('openFolder')}</button
              >
              <button class="link-action" onclick={closeFile}
                >{p.t('closeFile')}</button
              >
            </div>
          {/if}
        </aside>
      {/if}
    </section>
    <footer class="playback-bar">
      <button
        class="bar-action library-button"
        title={p.t('library')}
        aria-label={p.t('library')}
        aria-pressed={active === 'library'}
        onclick={() => togglePanel('library')}
      >
        <Icon name="clock" /><span>{p.t('library')}</span>
      </button>
      <PlaybackOptions />
      <span class="spacer"></span>
      <div class="view-tools">
        {#each tools as tool}
          <button
            class="bar-action"
            aria-label={p.t(tool.id)}
            title={p.t(tool.id)}
            aria-pressed={active === tool.id}
            onclick={() => togglePanel(tool.id)}
          >
            <Icon name={tool.icon} size={17} /><span>{p.t(tool.id)}</span>
          </button>
        {/each}
      </div>
    </footer>
  {:else}
    <div class="library-home">
      <RecentList home />
    </div>
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

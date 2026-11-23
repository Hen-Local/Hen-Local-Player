<script lang="ts">
  import { onMount } from 'svelte'
  import { player as p, modes } from './lib/stores.svelte'
  import Icon from './lib/Icon.svelte'
  import Player from './lib/Player.svelte'
  import LanguageControls from './lib/LanguageControls.svelte'
  import TranscriptPanel from './lib/TranscriptPanel.svelte'
  import RecentList from './lib/RecentList.svelte'
  import OpenUrlDialog from './lib/OpenUrlDialog.svelte'
  import ExportMenu from './lib/ExportMenu.svelte'
  import SetupScreen from './lib/SetupScreen.svelte'
  import SettingsDialog from './lib/SettingsDialog.svelte'
  const logo = '/hen-local.png'
  import './styles.css'
  function key(e: KeyboardEvent) {
    if (e.defaultPrevented || e.isComposing) return
    if (e.key === 'Escape') {
      if (p.dialog) return
      if (p.fullscreen) {
        e.preventDefault()
        void p.act(() => p.full(false))
      }
      return
    }
    const cmd = e.metaKey || e.ctrlKey,
      k = e.key.toLowerCase()
    if (p.dialog) return
    if (cmd && ['o', 'l', 'e'].includes(k)) {
      e.preventDefault()
      if (k === 'o') void p.choose()
      else if (k === 'l') p.dialog = 'url'
      else if (p.media) p.dialog = 'export'
      return
    }
    const el = e.target as HTMLElement | null
    if (cmd || e.altKey || el?.closest('input,textarea,select,[contenteditable=true]')) return
    if (k === ' ' && el?.closest('button')) return
    switch (k) {
      case ' ':
        p.toggle()
        break
      case 'arrowleft':
      case 'arrowright':
        p.seek(p.ms + (k === 'arrowleft' ? -1 : 1) * (e.shiftKey ? 30000 : 5000))
        break
      case 'arrowup':
      case 'arrowdown':
        p.volume =
          Math.round(Math.max(0, Math.min(1, p.volume + (k === 'arrowup' ? 0.05 : -0.05))) * 100) /
          100
        break
      case ',':
      case '.':
        p.jump(k === ',' ? -1 : 1)
        break
      case 'f':
        void p.act(() => p.full(!p.fullscreen))
        break
      case 's':
        void p.act(() => p.cycle())
        break
      case 't':
        void p.act(() => p.update({ showTranscriptPanel: !p.settings.showTranscriptPanel }))
        break
      case '[':
      case ']':
        p.rate = Math.max(0.5, Math.min(2, p.rate + (k === '[' ? -0.25 : 0.25)))
        break
      default:
        return
    }
    e.preventDefault()
  }
  onMount(() => {
    void p.act(() => p.init())
    return () => p.dispose()
  })
  $effect(() => {
    document.documentElement.lang = p.locale
    document.title = p.media ? p.media.title + ' · ' + p.t('app') : p.t('app')
  })
</script>

<svelte:window onkeydown={key} />
<main class="app" class:fullscreen={p.fullscreen}>
  <header class="top">
    <div class="brand">
      <img src={logo} alt="" />
      <b>Hen Local</b>
      <span class="slash">/</span>
      <span>{p.t('player')}</span>
    </div>
    <div class="top-actions">
      <button class="primary mini" onclick={() => p.choose()}>
        <Icon name="folder" size={16} />{p.t('open')}
      </button>
      <button onclick={() => (p.dialog = 'url')}>
        <Icon name="link" size={15} />{p.t('link')}
      </button>
    </div>
    <span class="spacer"></span>
    <button class="export-open" disabled={!p.media} onclick={() => (p.dialog = 'export')}>
      <Icon name="export" size={16} />{p.t('export')}
    </button>
    <span class="separator"></span>
    <button
      class="icon"
      title={p.t('settings')}
      aria-label={p.t('settings')}
      onclick={() => (p.dialog = 'settings')}
    >
      <Icon name="settings" />
    </button>
  </header>
  <section class="workspace" aria-label={p.t('player')}>
    <div class="workspace-header">
      <div class="file-heading">
        <span class="file-kind"><Icon name="film" size={17} /></span>
        <h1>{p.media?.title ?? p.t('empty')}</h1>
        {#if p.media && p.media.audioTracks.length > 1}<select
            title={p.t('trackChange')}
            aria-label={p.t('trackChange')}
            value={p.track}
            onchange={(e) => p.act(() => p.setTrack(+e.currentTarget.value))}
          >
            {#each p.media.audioTracks as a}<option value={a.order}>
                {p.t('track')}
                {a.order + 1} · {a.languageTag ?? a.codecName}
              </option>{/each}
          </select>{/if}
      </div>
      <button
        class="panel-button"
        aria-pressed={p.settings.showTranscriptPanel}
        title={p.t('panel')}
        aria-label={p.t('panel')}
        onclick={() =>
          p.act(() => p.update({ showTranscriptPanel: !p.settings.showTranscriptPanel }))}
      >
        <Icon name="transcript" size={17} />
        <span>{p.t('transcript')}</span>
      </button>
    </div>
    <div class="work-grid" class:single={!p.settings.showTranscriptPanel}>
      <div class="player-col">
        <Player />
        <div class="subtitle-toolbar">
          <LanguageControls />
          <div class="mode-control">
            <span>{p.t('display')}</span>
            <div class="mode-group">
              {#each modes as mode}<button
                  class:selected={p.settings.subtitleMode === mode}
                  aria-pressed={p.settings.subtitleMode === mode}
                  onclick={() => p.act(() => p.update({ subtitleMode: mode }))}
                >
                  {p.t(mode)}
                </button>{/each}
            </div>
          </div>
        </div>
      </div>
      {#if p.settings.showTranscriptPanel}{#if p.media}<TranscriptPanel />{:else}<RecentList
          />{/if}{/if}
    </div>
  </section>
  <footer class="statusbar">
    <span class="dot"></span>
    {p.t('local')}
    <span class="spacer"></span>
    <button class="footer-link" onclick={() => (p.dialog = 'setup')}>
      <Icon name="download" size={13} />{p.t('models')}
    </button>
  </footer>
  {#if p.error || p.notice}<div class="toast" role={p.error ? 'alert' : 'status'}>
      <Icon name={p.error ? 'alert' : 'check'} size={16} />
      <span>{p.error || p.notice}</span>
      <button
        class="icon"
        aria-label={p.t('dismiss')}
        onclick={() => {
          p.error = ''
          p.notice = ''
        }}
      >
        <Icon name="close" size={14} />
      </button>
    </div>{/if}
  {#if p.dialog === 'url'}<OpenUrlDialog />{:else if p.dialog === 'settings'}<SettingsDialog
    />{:else if p.dialog === 'setup'}<SetupScreen />{:else if p.dialog === 'export'}<ExportMenu
    />{/if}
</main>

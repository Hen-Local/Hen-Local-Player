<script lang="ts">
  import { onMount, tick } from 'svelte'
  import Dialog from './Dialog.svelte'
  import SelectField from './SelectField.svelte'
  import Icon from './Icon.svelte'
  import SetupScreen from './SetupScreen.svelte'
  import SoftwareUpdate from './SoftwareUpdate.svelte'
  import repository from '../../../release-repository.txt?raw'
  import { invoke } from '@tauri-apps/api/core'
  import { player as p, backend, mockMode } from './stores.svelte'
  import { interfaceLanguages } from './i18n'
  import { languageName } from './languages'
  import { size } from './format'
  import type { Settings } from './types'
  let { systemAccentAvailable }: { systemAccentAvailable: boolean } = $props()
  // A release without a creator handle has no funding link or external requests.
  const coffeeHandle = (import.meta.env.VITE_BUY_ME_A_COFFEE_USERNAME ?? '').trim()
  const coffeeUrl = /^[a-z0-9_-]+$/i.test(coffeeHandle) ? `https://buymeacoffee.com/${coffeeHandle}` : ''
  const themes = [
    {id:'neon-blue', label:'themeBlue'},
    {id:'neon-orange', label:'themeOrange'},
    {id:'neon-pink', label:'themePink'},
    {id:'neon-green', label:'themeGreen'},
  ] as const
  const projectUrl = 'https://github.com/' + repository.trim()
  const categories = { general: 'gear', processing: 'audio', storage: 'folder', about: 'info' } as const
  function openLink(event: MouseEvent & { currentTarget: HTMLAnchorElement }) {
    if (mockMode) return
    event.preventDefault()
    const url = event.currentTarget.href
    void p.act(() => invoke('plugin:opener|open_url', { url }))
  }
  let tab = $state<keyof typeof categories>('general'),
    confirm = $state(false),
    updating = $state(false)
  let showingSetup = $state(false)
  let setupButton = $state<HTMLButtonElement>()
  async function returnToSettings() {
    showingSetup = false
    await tick()
    setupButton?.focus()
  }
  function set<K extends keyof Settings>(k: K, value: Settings[K]) {
    void p.act(() => p.update({ [k]: value }))
  }
  onMount(() => {
    void p.act(async () => { p.components = await backend.call('get_setup_status') })
  })
  $effect(() => {
    if (tab === 'storage') void p.act(async () => { p.info = await backend.call('app_info') })
  })
  async function update() {
    updating = true
    try {
      await backend.call('update_ytdlp')
      p.toast(p.t('updated'))
    } catch (e) {
      p.message(e)
    } finally {
      updating = false
    }
  }
</script>

{#if showingSetup}
  <SetupScreen back={returnToSettings} />
{:else}
<Dialog title={p.t('settings')} icon="settings">
  {#snippet navigation()}
    <nav class="categories" aria-label={p.t('settings')}>
      {#each Object.entries(categories) as [key, icon]}
        {@const category = key as keyof typeof categories}
        <button aria-current={tab === category ? 'page' : undefined} onclick={() => (tab = category)}>
          <Icon name={icon} size={16}/><span>{p.t(category)}</span>
        </button>
      {/each}
    </nav>
  {/snippet}
  {#if tab === 'general'}
    <SoftwareUpdate />
    <div class="group choices">
      <label class="field-row">
        <span>{p.t('uiLang')}</span>
        <SelectField>
          <select value={p.settings.uiLanguage} onchange={(e) => set('uiLanguage', e.currentTarget.value)}>
            <option value="system">{p.t('system')}</option>
            {#each interfaceLanguages as language (language.code)}
              <option value={language.code} lang={language.code} dir="auto">{languageName(language.code, language.code)}</option>
            {/each}
          </select>
        </SelectField>
      </label>
      <label class="field-row">
        <span>{p.t('accentColor')}</span>
        <SelectField swatch="var(--accent)">
          <select value={p.settings.accentTheme === 'system' && !systemAccentAvailable ? 'neon-green' : p.settings.accentTheme}
            onchange={(e) => set('accentTheme', e.currentTarget.value as Settings['accentTheme'])}>
            {#if systemAccentAvailable}<option value="system">{p.t('followSystemColor')}</option>{/if}
            {#each themes as theme}<option value={theme.id}>{p.t(theme.label)}</option>{/each}
          </select>
        </SelectField>
      </label>
    </div>
    <div class="group resources">
      <div class="resource-row">
        <div class="resource-detail">
          <strong>{p.t('models')}</strong>
          {#if p.components.length}
            <small>{p.components.filter((c) => c.ready).length}/{p.components.length} {p.t('installed')}</small>
          {/if}
        </div>
        <button bind:this={setupButton} onclick={() => (showingSetup = true)}
          title={p.t('defaultModels') + ' · Qwen3-ASR ' + (p.settings.asrModel.startsWith('sherpa') ? '0.6B' : '1.7B') + ' + Hy-MT2 1.8B'}>{p.t('setup')}</button>
      </div>
      <div class="resource-row">
        <span title={p.t('linkTools')}>yt-dlp</span>
        <button disabled={updating} onclick={update} title={p.t('linkTools')}>
          {updating ? p.t('busy') : p.t('updateTool')}
        </button>
      </div>
      {#if coffeeUrl}
        <div class="resource-row">
          <span>{p.t('supportProject')}</span>
          <a href={coffeeUrl} target="_blank" rel="noreferrer" onclick={openLink}>{p.t('buyMeACoffee')}</a>
        </div>
      {/if}
    </div>
  {:else if tab === 'processing'}
    <div class="section-fields">
    <label>
      {p.t('model')}
      <SelectField>
        <select value={p.settings.asrModel} onchange={(e) => set('asrModel', e.currentTarget.value)}>
          {#if p.settings.asrModel.startsWith('sherpa')}
            <option value="sherpa-qwen3-asr-0.6b">{p.t('winModel')}</option>
          {:else}
            <option value="qwen3-asr-1.7b">{p.t('fullModel')}</option>
            <option value="qwen3-asr-0.6b">{p.t('lightModel')}</option>
          {/if}
        </select>
      </SelectField>
    </label>
    <label>
      {p.t('lookahead')}
      <input
        type="number"
        min="1"
        max="86400"
        aria-describedby="lookahead-help"
        value={p.settings.lookaheadSeconds}
        onchange={(e) => {
          if (e.currentTarget.validity.valid) set('lookaheadSeconds', +e.currentTarget.value)
        }}
      />
    </label>
    <p id="lookahead-help" class="help field-hint">{p.t('lookaheadHelp')}</p>
    {#each [['processWholeFile', 'whole'], ['dialogueChannelFocus', 'dialogue'], ['translationContext', 'context']] as [key, label]}<label
        class="toggle"
      >
        <input
          type="checkbox"
          checked={Boolean(p.settings[key as keyof Settings])}
          onchange={(e) => set(key as 'processWholeFile', e.currentTarget.checked)}
        />
        {p.t(label as 'whole')}
      </label>{/each}
    </div>
  {:else if tab === 'storage'}
    <div class="section-fields">
    <label>
      {p.t('downloadDir')}
      <input
        value={p.settings.downloadDir}
        placeholder={p.t('path')}
        onchange={(e) => set('downloadDir', e.currentTarget.value)}
      />
    </label>
    <label>
      {p.t('cacheLimit')}
      <input
        type="number"
        min="1"
        value={p.settings.cacheLimitGb}
        onchange={(e) => {
          if (e.currentTarget.validity.valid) set('cacheLimitGb', +e.currentTarget.value)
        }}
      />
    </label>
    <div class="setting-block">
      <div>
        <strong>{p.t('cache')}</strong>
        {#if p.info.logDir}<small>{size(p.info.cacheBytes)}</small>{/if}
      </div>
      <button
        onclick={() => {
          if (!confirm) {
            confirm = true
            return
          }
          void p.act(async () => {
            await backend.call('clear_cache')
            p.info = await backend.call('app_info')
            confirm = false
            p.toast(p.t('cacheCleared'))
          })
        }}
      >
        {p.t('clearCache')}
      </button>
    </div>
    {#if confirm}<p class="warn">{p.t('confirmClear')}</p>{/if}
    <button disabled={!p.info.logDir} onclick={() => p.act(() => backend.call('reveal_in_folder', { path: p.info.logDir }))}>
      {p.t('logs')}
    </button>
    </div>
  {:else}
    <div class="about-project">
      <h3>{p.t('app')}</h3>
      <a href={projectUrl} target="_blank" rel="noopener noreferrer" onclick={openLink}>
        <Icon name="link" size={15} />{p.t('githubProject')}
      </a>
    </div>
  {/if}
</Dialog>
{/if}

<style>
  .categories {
    display: flex;
    flex-direction: column;
    align-items: stretch;
    gap: 4px;
    padding: 12px 9px;
    background: rgb(0 0 0 / 12%);
    overflow: auto;
  }
  :global(.app .player-dialog) .categories button {
    min-height: 36px;
    justify-content: flex-start;
    padding: 7px 10px;
    text-align: start;
    font-size: 13px;
    font-weight: 530;
    background: transparent;
  }
  .categories :global(svg) { flex: none; }
  :global(.app .player-dialog) .categories button[aria-current="page"] {
    background: var(--surface-selection);
    font-weight: 650;
  }
  @media (forced-colors: active) {
    .categories button[aria-current="page"] { outline: 1px solid Highlight; outline-offset: -2px; }
  }
  .group {
    display: grid;
    gap: 12px;
    padding-block: 18px;
    border-top: 1px solid var(--hairline);
  }
  .group:last-child { padding-bottom: 0; }
  .field-row, .resource-row {
    display: grid;
    grid-template-columns: minmax(0, 1fr) minmax(0, 46%);
    gap: 14px;
    align-items: center;
    line-height: 1.35;
    font-size: 13px;
    font-weight: 550;
    overflow-wrap: anywhere;
  }
  .resource-row { grid-template-columns: minmax(0, 1fr) fit-content(49%); }
  .resource-row button, .resource-row a { font-size: 12px; justify-self: end; }
  .resource-detail { display: grid; gap: 3px; }
  .resource-detail strong { font-size: 13px; font-weight: 600; }
  .resource-detail small { font-size: 12px; font-weight: 400; }
  .section-fields { display: grid; gap: 14px; }
  .section-fields > label { display: grid; gap: 6px; font-size: 13px; font-weight: 550; }
  .section-fields .toggle { display: flex; }
  .field-hint { margin: -8px 0 4px; }
  .section-fields > button { justify-self: start; }
  .about-project h3 { margin: 0 0 12px; }
  .about-project a { display: inline-flex; align-items: center; gap: 7px; color: var(--glass-accent); text-underline-offset: 3px; }
  .about-project :global(svg) { flex: none; }
  @media (max-width: 575px) {
    .categories { flex-direction: row; gap: 3px; padding: 3px 10px 6px; background: transparent; }
    :global(.app .player-dialog) .categories button { flex: 1; justify-content: center; padding-inline: 3px; }
    .categories :global(svg) { display: none; }
    .group { gap: 10px; padding-block: 12px; }
  }
</style>

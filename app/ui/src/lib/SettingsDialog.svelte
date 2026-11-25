<script lang="ts">
  import { onMount, tick } from 'svelte'
  import Dialog from './Dialog.svelte'
  import SetupScreen from './SetupScreen.svelte'
  import SoftwareUpdate from './SoftwareUpdate.svelte'
  import { player as p, backend } from './stores.svelte'
  import { size } from './format'
  import type { Settings } from './types'
  const themes = [
    {id:'neon-blue', label:'themeBlue', color:'#0003fe'},
    {id:'neon-orange', label:'themeOrange', color:'#ff5705'},
    {id:'neon-pink', label:'themePink', color:'#ff0073'},
    {id:'neon-green', label:'themeGreen', color:'#51f91b'},
  ] as const
  const tabs = ['general', 'processing', 'storage'] as const
  let tab = $state<(typeof tabs)[number]>('general'),
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
    void p.act(async () => { p.info = await backend.call('app_info') })
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
  <div class="tabs settings-tabs" style:--selected-tab={tabs.indexOf(tab)}>
    {#each tabs as k, index}<button
        style:grid-column={index + 1}
        class:selected={tab === k}
        aria-pressed={tab === k}
        onclick={() => (tab = k)}
      >
        {p.t(k)}
      </button>{/each}
  </div>
  {#if tab === 'general'}
    <SoftwareUpdate />
    <label>
      {p.t('uiLang')}
      <select
        value={p.settings.uiLanguage}
        onchange={(e) => set('uiLanguage', e.currentTarget.value)}
      >
        <option value="system">{p.t('system')}</option>
        <option value="en">English</option>
        <option value="zh">中文</option>
      </select>
    </label>
    <fieldset class="theme-picker">
      <legend>{p.t('accentColor')}</legend>
      <div>{#each themes as theme}
        <button aria-pressed={p.settings.accentTheme === theme.id} onclick={() => set('accentTheme', theme.id)}>
          <i aria-hidden="true" style:background-color={theme.color}></i>{p.t(theme.label)}
        </button>
      {/each}</div>
    </fieldset>
    <div class="setting-block">
      <div>
        <strong>{p.t('models')}</strong>
        {#if p.components.length}
          <small>{p.components.filter((c) => c.ready).length}/{p.components.length} {p.t('installed')}</small>
        {/if}
        <small>{p.t('defaultModels')} · Qwen3-ASR {p.settings.asrModel.startsWith('sherpa') ? '0.6B' : '1.7B'} + Hy-MT2 1.8B</small>
      </div>
      <button bind:this={setupButton} onclick={() => (showingSetup = true)}>{p.t('setup')}</button>
    </div>
    <div class="setting-block">
      <div>
        <strong>yt-dlp</strong>
        <small>{p.t('linkTools')}</small>
      </div>
      <button disabled={updating} onclick={update}>
        {updating ? p.t('busy') : p.t('updateTool')}
      </button>
    </div>
  {:else if tab === 'processing'}
    <label>
      {p.t('model')}
      <select value={p.settings.asrModel} onchange={(e) => set('asrModel', e.currentTarget.value)}>
        {#if p.settings.asrModel.startsWith('sherpa')}<option value="sherpa-qwen3-asr-0.6b">
            {p.t('winModel')}
          </option>{:else}<option value="qwen3-asr-1.7b">{p.t('fullModel')}</option>
          <option value="qwen3-asr-0.6b">{p.t('lightModel')}</option>{/if}
      </select>
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
  {:else}
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
    </button>{/if}
</Dialog>
{/if}

<style>
  .settings-tabs {
    --tab-gap: 2px;
    isolation: isolate;
  }
  .settings-tabs::before {
    content: '';
    grid-area: 1 / 1;
    pointer-events: none;
    border-radius: 7px;
    background: linear-gradient(155deg, rgb(255 255 255 / 7%), transparent 68%), var(--surface-selection);
    box-shadow: 0 1px 4px rgb(0 0 0 / 15%);
    transform: translateX(calc(var(--selected-tab) * (100% + var(--tab-gap))));
    transition: transform 200ms cubic-bezier(.2, .8, .2, 1);
  }
  .settings-tabs button {
    grid-row: 1;
    position: relative;
  }
  @media (prefers-reduced-motion: reduce) {
    .settings-tabs::before { transition: none; }
  }
  @media (forced-colors: active) {
    .settings-tabs::before { background: Highlight; forced-color-adjust: none; }
    .settings-tabs button[aria-pressed="true"] { color: HighlightText; forced-color-adjust: none; }
  }
  .field-hint { margin: -8px 0 16px; }
  .theme-picker { padding:0; border:0; margin:0 0 16px; min-width:0; }
  .theme-picker legend { margin-bottom:8px; padding:0; font-size:12px; font-weight:630; }
  .theme-picker > div { display:grid; grid-template-columns:repeat(4,minmax(0,1fr)); gap:4px; }
  .theme-picker button { gap:5px; padding:6px 2px; font-size:12px; }
  .theme-picker i {
    display: block;
    width: 11px;
    height: 11px;
    border-radius: 50%;
    background-image: linear-gradient(145deg, rgb(255 255 255 / 35%), transparent 65%);
  }
  @media (max-width: 399px) {
    .theme-picker > div { grid-template-columns:repeat(2,minmax(0,1fr)); }
  }
  :global(:root[data-accent-theme="neon-orange"]) { --accent:#ff5705; --accent-soft:#fff0e9; --accent-text:#9c3500; }
  :global(:root[data-accent-theme="neon-pink"]) { --accent:#ff0073; --accent-soft:#ffe8f2; --accent-text:#a8004c; }
  :global(:root[data-accent-theme="neon-green"]) { --accent:#51f91b; --accent-soft:#ecffe6; --accent-text:#176d09; }
</style>

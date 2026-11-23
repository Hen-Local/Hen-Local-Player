<script lang="ts">
  import { onMount } from 'svelte'
  import Dialog from './Dialog.svelte'
  import SubtitleAppearance from './SubtitleAppearance.svelte'
  import { player as p, backend } from './stores.svelte'
  import { size } from './format'
  import { languages, languageName } from './languages'
  import type { Settings } from './types'
  const themes = [
    {id:'neon-blue', label:'themeBlue', color:'#0003fe'},
    {id:'neon-orange', label:'themeOrange', color:'#ff5705'},
    {id:'neon-pink', label:'themePink', color:'#ff0073'},
    {id:'neon-green', label:'themeGreen', color:'#51f91b'},
  ] as const
  let tab = $state<'general' | 'subtitles' | 'processing' | 'storage'>('general'),
    confirm = $state(false),
    updating = $state(false)
  function set<K extends keyof Settings>(k: K, value: Settings[K]) {
    void p.act(() => p.update({ [k]: value }))
  }
  onMount(
    () =>
      void p.act(async () => {
        p.info = await backend.call('app_info')
        p.components = await backend.call('get_setup_status')
      }),
  )
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

<Dialog title={p.t('settings')} icon="settings">
  <div class="tabs">
    {#each ['general', 'subtitles', 'processing', 'storage'] as k}<button
        class:selected={tab === k}
        aria-pressed={tab === k}
        onclick={() => (tab = k as typeof tab)}
      >
        {p.t(k as typeof tab)}
      </button>{/each}
  </div>
  {#if tab === 'general'}
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
          <i aria-hidden="true" style:background={theme.color}></i>{p.t(theme.label)}
        </button>
      {/each}</div>
    </fieldset>
    <label class="toggle">
      <input
        type="checkbox"
        checked={p.settings.showTranscriptPanel}
        onchange={(e) => set('showTranscriptPanel', e.currentTarget.checked)}
      />
      {p.t('transcriptSetting')}
    </label>
    <div class="setting-block">
      <div>
        <strong>{p.t('models')}</strong>
        <small>
          {p.components.filter((c) => c.ready).length}/{p.components.length}
          {p.t('installed')}
        </small>
      </div>
      <button onclick={() => (p.dialog = 'setup')}>{p.t('setup')}</button>
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
  {:else if tab === 'subtitles'}
    <label>
      {p.t('target')}
      <select
        value={p.settings.targetLanguage}
        onchange={(e) => p.act(() => p.setTarget(e.currentTarget.value))}
      >
        <option value="none">{p.t('none')}</option>
        {#each languages.filter((l) => l.target) as l}<option value={l.code}>
            {languageName(l.code, p.locale, true)}
          </option>{/each}
      </select>
    </label>
    <label>
      {p.t('order')}
      <select
        value={p.settings.bilingualOrder}
        onchange={(e) => set('bilingualOrder', e.currentTarget.value as Settings['bilingualOrder'])}
      >
        <option value="translation-first">{p.t('trFirst')}</option>
        <option value="original-first">{p.t('origFirst')}</option>
      </select>
    </label>
    <SubtitleAppearance />
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
        value={p.settings.lookaheadSeconds}
        onchange={(e) => {
          if (e.currentTarget.validity.valid) set('lookaheadSeconds', +e.currentTarget.value)
        }}
      />
    </label>
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
        <small>{size(p.info.cacheBytes)}</small>
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
    <button onclick={() => p.act(() => backend.call('reveal_in_folder', { path: p.info.logDir }))}>
      {p.t('logs')}
    </button>{/if}
</Dialog>

<style>
  .theme-picker { padding:0; border:0; margin:0 0 16px; min-width:0; }
  .theme-picker legend { margin-bottom:8px; padding:0; font-size:12px; font-weight:630; }
  .theme-picker > div { display:grid; grid-template-columns:repeat(4,minmax(0,1fr)); gap:4px; }
  .theme-picker button { gap:5px; padding:6px 2px; font-size:12px; }
  .theme-picker i { display:block; width:11px; height:11px; border:1px solid #ffffff55; border-radius:50%; }
  @media (max-width: 399px) {
    .theme-picker > div { grid-template-columns:repeat(2,minmax(0,1fr)); }
  }
  :global(:root[data-accent-theme="neon-orange"]) { --accent:#ff5705; --accent-soft:#fff0e9; --accent-text:#9c3500; }
  :global(:root[data-accent-theme="neon-pink"]) { --accent:#ff0073; --accent-soft:#ffe8f2; --accent-text:#a8004c; }
  :global(:root[data-accent-theme="neon-green"]) { --accent:#51f91b; --accent-soft:#ecffe6; --accent-text:#176d09; }
</style>

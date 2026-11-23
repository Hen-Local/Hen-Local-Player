<script lang="ts">
  import { onMount } from 'svelte'
  import Dialog from './Dialog.svelte'
  import { player as p, backend } from './stores.svelte'
  import { size } from './format'
  import { languages, languageName } from './languages'
  import type { Settings } from './types'
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
    {#each [{ key: 'subtitleScale', label: 'scale', min: 0.6, max: 2, step: 0.1 }, { key: 'subtitleBackground', label: 'background', min: 0, max: 0.8, step: 0.05 }, { key: 'subtitleBottomOffset', label: 'offset', min: 0, max: 0.4, step: 0.01 }] as opt}
      <label class="slider-setting">
        <span>
          {p.t(opt.label as 'scale')}
          <b>{Math.round(Number(p.settings[opt.key as keyof Settings]) * 100)}%</b>
        </span>
        <input
          type="range"
          min={opt.min}
          max={opt.max}
          step={opt.step}
          value={Number(p.settings[opt.key as keyof Settings])}
          onchange={(e) => set(opt.key as 'subtitleScale', +e.currentTarget.value)}
        />
      </label>{/each}
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

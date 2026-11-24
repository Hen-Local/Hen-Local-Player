<script lang="ts">
  import { player as p } from './stores.svelte'
  import { displaySourceLanguage, subtitleTracks as tracks } from './subtitle-tracks.svelte'
  import { languageName } from './languages'
  import LanguageControls from './LanguageControls.svelte'
  import SubtitleAppearance from './SubtitleAppearance.svelte'
  import Icon from './Icon.svelte'

  const source = $derived(displaySourceLanguage(p.settings, p.language))
  const sourceName = $derived(source ? languageName(source, p.locale) : p.t('originalLanguage'))
  const target = $derived(p.settings.targetLanguage)
  const targetName = $derived(languageName(target, p.locale))
  const twoLanguages = $derived(target !== 'none' && source !== target)
  const displayMode = $derived(twoLanguages ? p.settings.subtitleMode : 'original')

  function delay(seconds: number) {
    if (Number.isFinite(seconds) && Number.isSafeInteger(Math.round(seconds * 1000)))
      tracks.setDelay(Math.round(seconds * 1000))
  }
</script>

<details class="track-settings">
  <summary class="settings-disclosure"><Icon name="right" size={14} />{p.t('subtitleSettings')}</summary>
  <div class="expanded-tools">
    {#if tracks.selected === 'generated'}
      <label class="track-field subtitle-language-choice">
        <span>{p.t('displayLanguage')}</span>
        <select aria-label={p.t('displayLanguage')} value={displayMode} onchange={(e) => p.act(() => tracks.setMode(e.currentTarget.value))}>
          <option value="original">{sourceName}</option>
          {#if twoLanguages}
            <option value="translation">{targetName}</option>
            <option value="both">{p.t('bothLanguages')} · {sourceName} + {targetName}</option>
          {/if}
        </select>
      </label>
    {/if}
    <SubtitleAppearance />
    {#if tracks.selected === 'generated'}
      <LanguageControls />
      {#if twoLanguages && displayMode === 'both'}
        <label class="track-field">
          <span>{p.t('order')}</span>
          <select aria-label={p.t('order')} value={p.settings.bilingualOrder}
            onchange={(e) => p.act(() => p.update({ bilingualOrder: e.currentTarget.value as 'translation-first' | 'original-first' }))}>
            <option value="translation-first">{targetName} · {p.t('onTop')}</option>
            <option value="original-first">{sourceName} · {p.t('onTop')}</option>
          </select>
        </label>
      {/if}
    {/if}
    <div class="track-field">
      <label for="track-delay-seconds">{p.t('subtitleDelay')}</label>
      <div class="delay-stepper">
        <button type="button" aria-label={p.t('earlier')} title={p.t('earlier')}
          onclick={() => delay((tracks.delayMs - 100) / 1000)}>−</button>
        <input id="track-delay-seconds" type="number" step="0.1" value={tracks.delayMs / 1000}
          onchange={(e) => delay(e.currentTarget.valueAsNumber)} />
        <button type="button" aria-label={p.t('later')} title={p.t('later')}
          onclick={() => delay((tracks.delayMs + 100) / 1000)}>+</button>
      </div>
    </div>
    <div class="subtitle-file-actions">
      {#if tracks.discoveryError}<button onclick={() => tracks.discover(p.media?.sessionId ?? '')}>{p.t('retryTracks')}</button>{/if}
      {#if tracks.selected === 'generated' && p.cues.length}
        <button onclick={() => (p.dialog = 'export')}><Icon name="export" size={14} />{p.t('exportSubs')}</button>
      {/if}
      <button onclick={() => (p.dialog = 'setup')}><Icon name="download" size={14} />{p.t('models')}</button>
    </div>
  </div>
</details>

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
  const languagePair = $derived(sourceName + ' + ' + targetName)
  const displayName = $derived(displayMode === 'both' ? languagePair : displayMode === 'translation' ? targetName : sourceName)

  function delay(seconds: number) {
    if (Number.isFinite(seconds) && Number.isSafeInteger(Math.round(seconds * 1000)))
      tracks.setDelay(Math.round(seconds * 1000))
  }
</script>

{#if tracks.selected === 'generated'}
  <details class="track-settings">
    <summary class="settings-disclosure"><Icon name="right" size={14} />{p.t('subtitleLanguages')}</summary>
    <div class="expanded-tools">
      <LanguageControls />
      <label class="track-field display-field">
        <span>{p.t('displayLanguage')}</span>
        <span class="select-box display-choice">
          <span class="selected-languages" aria-hidden="true">{displayName}</span>
          <select aria-label={p.t('displayLanguage')} value={displayMode}
            onchange={(e) => p.act(() => tracks.setMode(e.currentTarget.value))}>
            <option value="original">{sourceName}</option>
            {#if twoLanguages}
              <option value="translation">{targetName}</option>
              <option value="both">{languagePair}</option>
            {/if}
          </select>
          <Icon name="down" size={13} />
        </span>
      </label>
      {#if twoLanguages && displayMode === 'both'}
        <label class="track-field">
          <span>{p.t('order')}</span>
          <span class="select-box">
            <select aria-label={p.t('order')} value={p.settings.bilingualOrder}
              onchange={(e) => p.act(() => p.update({ bilingualOrder: e.currentTarget.value as 'translation-first' | 'original-first' }))}>
              <option value="translation-first">{targetName} · {p.t('onTop')}</option>
              <option value="original-first">{sourceName} · {p.t('onTop')}</option>
            </select>
            <Icon name="down" size={13} />
          </span>
        </label>
      {/if}
      <button class="models-action" onclick={() => (p.dialog = 'setup')}>
        <Icon name="download" size={14} />{p.t('models')}
      </button>
    </div>
  </details>
{/if}
<details class="track-settings">
  <summary class="settings-disclosure"><Icon name="right" size={14} />{p.t('subtitleAppearance')}</summary>
  <div class="expanded-tools">
    <SubtitleAppearance />
    <div class="delay-setting">
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
  </div>
</details>

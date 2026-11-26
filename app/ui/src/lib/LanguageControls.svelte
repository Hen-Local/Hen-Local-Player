<script lang="ts">
  import SelectField from './SelectField.svelte'
  import { player as p } from './stores.svelte'
  import { languages, languageName } from './languages'
</script>

<label class="track-field">
  <span>{p.t('source')}</span>
  <SelectField>
    <select
      aria-label={p.t('source')}
      value={p.settings.sourceLanguage}
      onchange={(e) => void p.act(() => p.setSource(e.currentTarget.value))}
    >
      <option value="auto">
        {p.t('auto')}{p.language?.detected
          ? ' · ' +
            languageName(p.language.detected, p.locale) +
            ' ' +
            Math.round(p.language.confidence * 100) +
            '%'
          : ''}
      </option>
      <option value="auto-per-line">{p.t('perLine')}</option>
      {#each languages.filter((l) => l.spoken) as l}<option value={l.code}>
          {languageName(l.code, p.locale, true)}
        </option>{/each}
    </select>
  </SelectField>
</label>
<label class="track-field">
  <span>{p.t('target')}</span>
  <SelectField>
    <select
      aria-label={p.t('target')}
      value={p.settings.targetLanguage}
      onchange={(e) => void p.act(() => p.setTarget(e.currentTarget.value))}
    >
      <option value="none">{p.t('none')}</option>
      {#each languages.filter((l) => l.target) as l}<option value={l.code}>
          {languageName(l.code, p.locale, true)}
        </option>{/each}
    </select>
  </SelectField>
</label>
{#if p.language?.unverifiedTranslation}<span class="warn">{p.t('unverified')}</span>{/if}

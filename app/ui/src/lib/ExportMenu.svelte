<script lang="ts">
  import SelectField from './SelectField.svelte'
  import Dialog from './Dialog.svelte'
  import { backend, player as p } from './stores.svelte'
  import type { SubtitleFormat, SubtitleVariant } from './types'
  let content = $state<'original' | 'translation' | 'bilingual'>(
      p.settings.targetLanguage === 'none' || p.settings.targetLanguage === p.language?.detected
        ? 'original'
        : 'bilingual',
    ),
    format = $state<SubtitleFormat>('srt'),
    partial = $state(false),
    all = $state(false),
    busy = $state(false)
  let availableTranslation = $derived(
    p.settings.targetLanguage !== 'none' && p.settings.targetLanguage !== p.language?.detected,
  )
  function variant(mode: typeof content): SubtitleVariant {
    return mode === 'bilingual' ? { bilingual: p.settings.bilingualOrder } : mode
  }
  async function perform() {
    const jobs: { f: SubtitleFormat; v: SubtitleVariant; path: string }[] = []
    const formats: SubtitleFormat[] = all ? ['srt', 'vtt', 'ass'] : [format]
    const variants: SubtitleVariant[] =
      all && availableTranslation
        ? ['original', 'translation', variant('bilingual')]
        : [all || !availableTranslation ? 'original' : variant(content)]
    for (const f of formats)
      for (const v of variants) {
        const suggested = await backend.call('suggest_export_paths', { variant: v, format: f })
        const path = all ? suggested : await backend.saveFile(suggested)
        if (path) jobs.push({ f, v, path })
      }
    for (const j of jobs)
      await backend.call('export_subtitles', { format: j.f, variant: j.v, path: j.path })
    if (jobs.length) p.toast(p.t('saved'))
  }
  async function run(finish: boolean) {
    busy = true
    try {
      if (finish) {
        await p.update({ processWholeFile: true })
        if (p.status?.stage === 'partial') return
        if (p.status?.stage !== 'complete') {
          p.scheduleExport(perform)
          p.toast(p.t('queued'))
          p.dialog = null
          return
        }
      }
      await perform()
      p.dialog = null
    } catch (e) {
      p.message(e)
    } finally {
      busy = false
    }
  }
  function choose(multiple: boolean) {
    all = multiple
    const stage = p.status?.stage
    if (stage === 'complete' || (stage === 'partial' && (!availableTranslation || (!all && content === 'original'))))
      void run(false)
    else partial = true
  }
</script>

<Dialog title={p.t('exportSubs')} icon="export">
  <div class="export-fields">
    <label for="subtitle-export-content">{p.t('variant')}</label>
    <SelectField>
      <select id="subtitle-export-content" bind:value={content}>
        <option value="original">{p.t('original')}</option>
        <option value="translation" disabled={!availableTranslation}>{p.t('translation')}</option>
        <option value="bilingual" disabled={!availableTranslation}>{p.t('bilingual')}</option>
      </select>
    </SelectField>
    <label for="subtitle-export-format">{p.t('format')}</label>
    <SelectField>
      <select id="subtitle-export-format" bind:value={format}>
        <option value="srt">SubRip (.srt)</option>
        <option value="vtt">WebVTT (.vtt)</option>
        <option value="ass">Advanced SubStation Alpha (.ass)</option>
      </select>
    </SelectField>
  </div>
  {#if partial}
    <p class="help" role="status">{p.status?.stage === 'partial' ? p.t('partialExport') : Math.round(p.progress * 100) + '% ' + p.t('incomplete')}</p>
  {/if}
  <div class="dialog-actions export-actions">
    {#if partial}
      {#if p.status?.stage !== 'partial'}
        <button disabled={busy} onclick={() => run(true)}>{p.t('finish')}</button>
      {/if}
      <button class="primary" disabled={busy} onclick={() => run(false)}>{p.t('exportNow')}</button>
    {:else}
      <button disabled={busy} onclick={() => choose(true)}>{p.t('exportAll')}</button>
      <button class="primary" disabled={busy} onclick={() => choose(false)}>{p.t('saveAs')}</button>
    {/if}
  </div>
</Dialog>

<style>
  .export-fields {
    display: grid;
    grid-template-columns: fit-content(40%) minmax(0, 1fr);
    align-items: center;
    gap: 10px 20px;
  }
  .export-fields label {
    color: var(--ink);
    font-size: 13px;
    font-weight: 560;
    overflow-wrap: anywhere;
  }
  .export-actions { padding-block-start: 20px; }
  .export-actions button { min-block-size: 32px; }
  .export-actions .primary { min-inline-size: 94px; }
  @media (max-width: 380px) {
    .export-fields { grid-template-columns: minmax(0, 1fr); gap: 6px; }
    .export-fields label:not(:first-child) { margin-block-start: 5px; }
    .export-actions button { flex: 1 1 auto; }
  }
</style>

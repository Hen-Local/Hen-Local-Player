<script lang="ts">
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
    if (p.status?.stage !== 'complete') partial = true
    else void run(false)
  }
</script>

<Dialog title={p.t('exportSubs')} icon="export">
  <label>
    {p.t('variant')}
    <select bind:value={content}>
      <option value="original">{p.t('original')}</option>
      <option value="translation" disabled={!availableTranslation}>{p.t('translation')}</option>
      <option value="bilingual" disabled={!availableTranslation}>{p.t('bilingual')}</option>
    </select>
  </label>
  <label>
    {p.t('format')}
    <select bind:value={format}>
      <option value="srt">SubRip (.srt)</option>
      <option value="vtt">WebVTT (.vtt)</option>
      <option value="ass">Advanced SubStation Alpha (.ass)</option>
    </select>
  </label>
  {#if partial}<p class="help">{Math.round(p.progress * 100)}% {p.t('incomplete')}</p>
    <div class="dialog-actions">
      <button disabled={busy} onclick={() => run(true)}>{p.t('finish')}</button>
      <button class="primary" disabled={busy} onclick={() => run(false)}>{p.t('exportNow')}</button>
    </div>
  {:else}<div class="dialog-actions">
      <button disabled={busy} onclick={() => choose(true)}>{p.t('exportAll')}</button>
      <button class="primary" disabled={busy} onclick={() => choose(false)}>{p.t('saveAs')}</button>
    </div>{/if}
</Dialog>

<script lang="ts">
  import { player as p } from './stores.svelte'

  let { cached }: { cached: boolean } = $props()
  let status = $derived(p.status)
  let stage = $derived(status?.stage)
  let paused = $derived(cached || stage === 'paused')
  let failed = $derived(stage === 'error' || !!p.subtitleError)
  let audio = $derived(status?.vadComplete ? 100 : percent(status?.extractedUntilMs ?? 0, p.duration))
  let transcribed = $derived(Math.round(p.progress * 100))
  let translated = $derived(status?.trPendingSpans.length
    ? percent(status.translatedCount + status.uttNoSpeech, status.uttTotal) : transcribed)
  let translate = $derived(p.settings.targetLanguage !== 'none')
  let progress = $derived(status?.uttTotal ? Math.min(audio, transcribed, translate ? translated : 100) : undefined)

  function percent(value: number, whole: number) {
    return whole ? Math.min(100, Math.round(100 * value / whole)) : 0
  }
</script>

<section class="coverage-summary" aria-label={p.t('subtitleProgress')}>
  <div class="status-line">
    <span role="status">{failed ? p.t('errors') : paused ? p.t('generationPaused') : p.t('creatingSubtitles')}</span>
    {#if progress !== undefined}<span class="progress-value">{progress}%</span>{/if}
  </div>
  {#if status && (stage !== 'idle' || cached)}
    <dl class="progress-stages">
      <div><dt>{p.t('audio')}</dt><dd>{audio}%</dd></div>
      <div><dt>{p.t('asr')}</dt><dd>{transcribed}%</dd></div>
      {#if translate}<div><dt>{p.t('translated')}</dt><dd>{translated}%</dd></div>{/if}
    </dl>
    {#if status.asrRtf > 0}<small>{p.t('transcriptionSpeed')} · {Number(status.asrRtf.toFixed(1))}×</small>{/if}
  {/if}
  {#if progress === undefined}
    <progress max="100" aria-label={p.t('subtitleProgress')}></progress>
  {:else}
    <progress max="100" value={progress} aria-label={p.t('subtitleProgress')}></progress>
  {/if}
</section>

<style>
  @media (max-height: 350px) {
    :global(.app .right-inspector) .progress-stages,
    small { display: none; }
  }
</style>

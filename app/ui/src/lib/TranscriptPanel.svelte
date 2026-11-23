<script lang="ts">
  import { tick } from 'svelte'
  import { player as p, backend } from './stores.svelte'
  import Icon from './Icon.svelte'
  import { clock } from './format'
  import { subtitleTracks } from './subtitle-tracks.svelte'
  import { visibleIndices } from './subtitle-tracks'
  let scroller = $state<HTMLDivElement>(),
    scroll = $state(0),
    height = $state(500),
    manualUntil = 0
  let authored = $derived(subtitleTracks.authored)
  let rows = $derived(authored ? authored.cues.map((cue, i) => ({
    id: 'authored-'+i, startMs: cue.startMs, original: cue.text, translation: null, stale: false,
  })) : p.cues)
  let activeIndices = $derived(authored ? visibleIndices(authored, p.ms) : [p.active])
  let activeRow = $derived(activeIndices.at(-1) ?? -1)
  const rowH = 88
  let virtual = $derived(rows.length > 500),
    start = $derived(virtual ? Math.max(0, Math.floor(scroll / rowH) - 4) : 0)
  let end = $derived(
    virtual ? Math.min(rows.length, start + Math.ceil(height / rowH) + 9) : rows.length,
  )
  let pending = $derived(authored ? [] : p.status?.asrPendingSpans ?? [])
  let status = $derived(authored ? null : p.status)
  $effect(() => {
    const index = activeRow
    if (index < 0 || !scroller || Date.now() < manualUntil) return
    if (virtual) {
      if (
        index * rowH < scroller.scrollTop ||
        (index + 1) * rowH > scroller.scrollTop + scroller.clientHeight
      )
        scroller.scrollTop = Math.max(0, index * rowH - scroller.clientHeight / 3)
    } else
      void tick().then(() =>
        scroller?.querySelector('[aria-current="true"]')?.scrollIntoView({ block: 'nearest' }),
      )
  })
  function manual() {
    manualUntil = Date.now() + 5000
  }
</script>

<div class="side-panel transcript">
  {#if status}<div class="coverage-summary">
      <div class="status-line">
        <span class="dot"></span>
        {status.stage === 'complete'
          ? p.t('complete')
          : status.stage === 'error' ? p.t('errors') : status.stage === 'paused' ? p.t('paused') : status.stage === 'needs-setup'
            ? p.t('needsSetup')
            : p.t('processing')}
        <span class="spacer"></span>
        <b>{Math.round(p.progress * 100)}%</b>
      </div>
      <div class="micro-rail"><i style:width={p.progress * 100 + '%'}></i></div>
      <small>
        {p.t('audio')}
        {status.vadComplete ? 100 : Math.min(100, Math.round((status.extractedUntilMs / (p.duration || 1)) * 100))}% · {p.t(
          'asr',
        )}
        {Math.round(p.progress * 100)}%{#if p.settings.targetLanguage !== 'none'}
          · {p.t('translated')}
          {Math.round((status.translatedCount / (status.uttTotal || 1)) * 100)}%{/if}
      </small>
      {#if status.asrRtf > 0}<small>
          ASR {Number(status.asrRtf.toFixed(1))}× {p.t('realtime')}
        </small>{/if}
    </div>{/if}
  <div
    role="region"
    aria-label={p.t('lines')}
    class="cue-list"
    bind:this={scroller}
    bind:clientHeight={height}
    onscroll={(e) => (scroll = e.currentTarget.scrollTop)}
    onwheel={manual}
    ontouchstart={manual}
    onpointerdown={manual}
  >
    {#if virtual}<div style:height={start * rowH + 'px'}></div>{/if}
    {#each rows.slice(start, end) as cue, i (cue.id)}
      <button
        class="cue-row"
        class:active={activeIndices.includes(i + start)}
        class:virtual
        aria-current={activeIndices.includes(i + start)}
        onclick={() => p.seek(authored ? cue.startMs : cue.startMs - 100)}
      >
        <time>{clock(cue.startMs)}</time>
        <span>
          <strong dir="auto" style:white-space={authored ? 'pre-line' : undefined}>{cue.original}</strong>
          {#if cue.translation}<span dir="auto">{cue.translation}</span>{/if}{#if cue.stale}<small>
              {p.t('stale')}
            </small>{/if}
        </span>
      </button>
    {/each}
    {#if virtual}<div style:height={(rows.length - end) * rowH + 'px'}></div>{/if}
    {#each pending.slice(0, 4) as span}<button
        class="cue-row pending"
        onclick={() => p.seek(span[0])}
      >
        <time>{clock(span[0])}</time>
        <span>{p.t('transcribing')}</span>
      </button>{/each}
    {#if !rows.length && !pending.length}<p class="empty-note subtle">
        {status?.stage === 'complete' ? p.t('noSpeech') : p.t('waiting')}
      </p>{/if}
    {#if status && (status.uttFailed > 0 || status.stage === 'error')}<button
        class="retry"
        onclick={() => p.act(() => backend.call('retry_failed'))}
      >
        {p.t('retryFailed')}{status.uttFailed ? ' (' + status.uttFailed + ')' : ''}
      </button>{/if}
  </div>
  <div class="panel-footer">
    <Icon name="clock" size={13} />
    <span>{rows.length} {p.t('lines')}</span>
    <span class="spacer"></span>
    <kbd>,</kbd>
    <kbd>.</kbd>
  </div>
</div>

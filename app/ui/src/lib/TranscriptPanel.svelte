<script lang="ts">
  import { tick } from 'svelte'
  import { player as p, cueAt } from './stores.svelte'
  import Icon from './Icon.svelte'
  import { clock } from './format'
  import { generatedLines, subtitleTracks } from './subtitle-tracks.svelte'
  import { visibleIndices } from './subtitle-tracks'
  let scroller = $state<HTMLDivElement>(),
    scroll = $state(0),
    height = $state(500),
    manualUntil = 0
  let authored = $derived(subtitleTracks.authored)
  let rows = $derived(
    authored
      ? authored.cues.map((cue, i) => ({
          id: 'authored-' + i,
          startMs: cue.startMs,
          original: cue.text,
          translation: null,
          stale: false,
        }))
      : p.cues,
  )
  let activeIndices = $derived(
    authored ? visibleIndices(authored, p.ms) : [cueAt(p.cues, p.ms - subtitleTracks.delayMs)],
  )
  let activeRow = $derived(activeIndices.at(-1) ?? -1)
  const rowH = 88
  let virtual = $derived(rows.length > 500),
    start = $derived(virtual ? Math.max(0, Math.floor(scroll / rowH) - 4) : 0)
  let end = $derived(
    virtual ? Math.min(rows.length, start + Math.ceil(height / rowH) + 9) : rows.length,
  )

  let status = $derived(
    !authored &&
      p.status &&
      p.status.stage !== 'idle' &&
      p.status.stage !== 'needs-setup' &&
      (p.transcriptionRequested || p.status.stage === 'complete')
      ? p.status
      : null,
  )
  let pending = $derived(status?.asrPendingSpans ?? [])
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
      void tick().then(() => {
        const row = scroller?.querySelector('[aria-current="true"]')
        if (!row || !scroller) return
        const list = scroller.getBoundingClientRect(),
          current = row.getBoundingClientRect()
        if (current.top < list.top) scroller.scrollTop += current.top - list.top
        else if (current.bottom > list.bottom) scroller.scrollTop += current.bottom - list.bottom
      })
  })
  function manual() {
    manualUntil = Date.now() + 5000
  }
</script>

<div class="side-panel transcript">
  {#if status && status.stage !== 'complete'}<details class="coverage-summary">
      <summary class="status-line">
        <span class="dot"></span>
        {status.stage === 'complete'
          ? p.t('complete')
          : status.stage === 'error'
            ? p.t('errors')
            : status.stage === 'paused'
              ? p.t('paused')
              : status.stage === 'needs-setup'
                ? p.t('needsSetup')
                : p.t('processing')}
        <span class="spacer"></span>
        <b>{Math.round(p.progress * 100)}%</b>
      </summary>
      <div class="micro-rail"><i style:width={p.progress * 100 + '%'}></i></div>
      <small>
        {p.t('audio')}
        {status.vadComplete
          ? 100
          : Math.min(100, Math.round((status.extractedUntilMs / (p.duration || 1)) * 100))}% · {p.t(
          'asr',
        )}
        {Math.round(p.progress * 100)}%{#if p.settings.targetLanguage !== 'none'}
          · {p.t('translated')}
          {Math.round((status.translatedCount / (status.uttTotal || 1)) * 100)}%{/if}
      </small>
      {#if status.asrRtf > 0}<small>
          ASR {Number(status.asrRtf.toFixed(1))}× {p.t('realtime')}
        </small>{/if}
    </details>{/if}
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
      {@const text = authored ? [cue.original] : generatedLines(cue, p.settings)}
      <button
        class="cue-row"
        class:active={activeIndices.includes(i + start)}
        class:virtual
        aria-current={activeIndices.includes(i + start)}
        onclick={() =>
          p.seek(authored ? cue.startMs : Math.max(0, cue.startMs + subtitleTracks.delayMs))}
      >
        <time
          >{clock(authored ? cue.startMs : Math.max(0, cue.startMs + subtitleTracks.delayMs))}</time
        >
        <span>
          <strong dir="auto" style:white-space={authored ? 'pre-line' : undefined}
            >{text[0] ?? p.t('translating')}</strong
          >
          {#if text[1]}<span dir="auto">{text[1]}</span>{/if}{#if cue.stale}<small>
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
    {#if !rows.length && !pending.length && status && ['processing', 'complete'].includes(status.stage)}<p
        class="empty-note subtle"
      >
        {status?.stage === 'complete' ? p.t('noSpeech') : p.t('waiting')}
      </p>{/if}
  </div>
  <div class="panel-footer">
    <Icon name="clock" size={13} />
    <span>{rows.length} {p.t('lines')}</span>
    <span class="spacer"></span>
    {#if status?.stage === 'complete'}<span>{p.t('complete')}</span>{/if}
    <kbd>,</kbd>
    <kbd>.</kbd>
  </div>
</div>

<style>
  .transcript {
    min-height: clamp(116px, 32vh, 166px);
  }
  .transcript .coverage-summary {
    padding: 7px 12px;
  }
  .transcript .coverage-summary summary {
    cursor: pointer;
  }
</style>

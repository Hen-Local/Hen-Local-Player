<script lang="ts">
  import { tick } from 'svelte'
  import { player as p, cueAt } from './stores.svelte'
  import Icon from './Icon.svelte'
  import { clock } from './format'
  import { generatedLines, subtitleTracks } from './subtitle-tracks.svelte'
  import { visibleIndices } from './subtitle-tracks'

  let expanded = $state(false)
  let scroller = $state<HTMLDivElement>(),
    scroll = $state(0),
    height = $state(500),
    manualUntil = 0
  let authored = $derived(subtitleTracks.authored)
  let count = $derived(authored?.cues.length ?? p.cues.length)
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

  let pending = $derived(!authored && p.transcriptionRequested ? p.status?.asrPendingSpans ?? [] : [])
  $effect(() => {
    if (!expanded || !scroller || Date.now() < manualUntil) return
    const index = activeRow
    if (index < 0) return
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

<div class="transcript" class:expanded>
  <button
    class="subtitle-list-toggle"
    aria-expanded={expanded}
    aria-controls="subtitle-list"
    onclick={() => { expanded = !expanded }}
  >
    <Icon name={expanded ? 'down' : 'right'} size={14} />
    {p.t('subtitleList')}
    <span class="row-count" aria-label={count + ' ' + p.t('lines')}>{count}</span>
  </button>
  <div id="subtitle-list" hidden={!expanded}>
    {#if expanded}
      <div
        role="region"
        aria-label={p.t('subtitleList')}
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
        {#if !rows.length && !pending.length}<p class="empty-note">
          {p.t('waiting')}
        </p>{/if}
      </div>
    {/if}
  </div>
</div>

<style>
  .transcript {
    flex: none;
    min-width: 0;
  }
  .transcript.expanded,
  #subtitle-list:not([hidden]) {
    display: flex;
    flex: 1 0 auto;
    flex-direction: column;
  }
  .cue-list {
    min-height: 3rem;
    max-height: 54dvh;
    overscroll-behavior: none contain;
  }
  .row-count {
    margin-left: auto;
    color: var(--muted);
    font-size: 12px;
    font-variant-numeric: tabular-nums;
  }
</style>

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
  // Generated lists grow during playback. Use the same row geometry from the
  // first batch onward so the scrollbar stays anchored as captions arrive.
  const rowH = 88
  let virtual = $derived(!authored || count > 32)
  let start = $derived(virtual ? Math.min(count, Math.max(0, Math.floor(scroll / rowH) - 4)) : 0)
  let end = $derived(virtual ? Math.min(count, start + Math.ceil(height / rowH) + 9) : count)
  let rows = $derived.by(() =>
    authored
      ? authored.cues.slice(start, end).map((cue, i) => ({
          id: 'authored-' + (start + i),
          startMs: cue.startMs,
          original: cue.text,
          translation: null,
          stale: false,
        }))
      : p.cues.slice(start, end),
  )
  let activeAuthored = $derived(expanded && authored ? visibleIndices(authored, p.ms) : [])
  let activeRow = $derived(!expanded ? -1 : authored ? activeAuthored.at(-1) ?? -1
    : cueAt(p.cues, p.ms - subtitleTracks.delayMs))

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
    onclick={() => {
      scroll = 0
      manualUntil = 0
      expanded = !expanded
    }}
  >
    <Icon name={expanded ? 'down' : 'right'} size={14} />
    {p.t('subtitleList')}
    <span class="row-count" aria-label={count + ' ' + p.t('lines')}>{count}</span>
  </button>
  <div id="subtitle-list" hidden={!expanded}>
    {#if expanded}
      {#if !authored && p.cues.length}
        <div class="list-tools">
          <button onclick={() => (p.dialog = 'export')}><Icon name="export" size={14} />{p.t('exportSubs')}</button>
        </div>
      {/if}
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
        {#each rows as cue, i (cue.id)}
          {@const active = authored ? activeAuthored.includes(start + i) : activeRow === start + i}
          {@const text = authored ? [cue.original] : generatedLines(cue, p.settings)}
          <button
            class="cue-row"
            class:active
            class:virtual
            aria-current={active}
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
        {#if virtual}<div style:height={(count - end) * rowH + 'px'}></div>{/if}
        {#each pending.slice(0, 4) as span}<button
            class="cue-row pending"
            onclick={() => p.seek(span[0])}
          >
            <time>{clock(span[0])}</time>
            <span>{p.t('transcribing')}</span>
          </button>{/each}
        {#if !count && !pending.length}<p class="empty-note">
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
  .transcript.expanded {
    display: flex;
    flex: 1 0 9rem;
    flex-direction: column;
    min-height: 0;
  }
  .subtitle-list-toggle { flex: none; }
  #subtitle-list:not([hidden]) {
    display: flex;
    flex: 1;
    flex-direction: column;
    min-height: 0;
  }
  .cue-list {
    min-height: 0;
    overflow-anchor: none;
    overscroll-behavior: none contain;
  }
  .row-count {
    margin-left: auto;
    color: var(--muted);
    font-size: 12px;
    font-variant-numeric: tabular-nums;
  }
</style>

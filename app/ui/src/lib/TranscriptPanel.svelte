<script lang="ts">
  import { onDestroy, tick, untrack } from 'svelte'
  import { SvelteSet } from 'svelte/reactivity'
  import Disclosure from './Disclosure.svelte'
  import { player as p, cueAt } from './stores.svelte'
  import Icon from './Icon.svelte'
  import { clock } from './format'
  import { generatedLines, subtitleTracks } from './subtitle-tracks.svelte'
  import { visibleIndices } from './subtitle-tracks'

  let { viewport, browsing = $bindable(false) }: {
    viewport: HTMLDivElement | undefined
    browsing: boolean
  } = $props()
  let expanded = $state(false)
  let list = $state<HTMLDivElement>()
  let authored = $derived(subtitleTracks.authored)
  let count = $derived(authored?.cues.length ?? p.cues.length)
  let activeAuthored = $derived(expanded && authored ? visibleIndices(authored, p.ms) : [])
  let activeRow = $derived(!expanded ? -1 : authored ? activeAuthored.at(-1) ?? -1
    : cueAt(p.cues, p.ms - subtitleTracks.delayMs))
  let pending = $derived(!authored && p.transcriptionRequested ? p.status?.asrPendingSpans ?? [] : [])

  // Unseen groups reserve space in the panel's own scroll flow. Once read,
  // reserve their measured height, including every wrapped line of text.
  const groupSize = 16
  const nearby = new SvelteSet<number>()
  const heights = new Map<number, number>()
  let observer: IntersectionObserver | undefined
  function observeGroup(node: HTMLDivElement, index: number) {
    observer ??= new IntersectionObserver(entries => {
      if (!viewport || !list || !expanded) return
      const atEnd = viewport.scrollTop > 0 && viewport.scrollHeight - viewport.scrollTop - viewport.clientHeight <= 1
      const panel = viewport.getBoundingClientRect()
      const topRow = document.elementFromPoint(panel.left + panel.width / 2, panel.top + 1)
        ?.closest<HTMLElement>('[data-row], [data-group]')
      const anchor = topRow && list.contains(topRow) ? topRow : null
      const anchorTop = anchor?.getBoundingClientRect().top
      for (const entry of entries) {
        const group = entry.target as HTMLDivElement
        const i = Number(group.dataset.group)
        if (entry.isIntersecting) {
          nearby.add(i)
        } else if (!group.contains(document.activeElement)) {
          if (group.firstElementChild) heights.set(i, group.getBoundingClientRect().height)
          nearby.delete(i)
        }
      }
      // WebKit does not consistently anchor scrolling when a preceding group
      // grows. Keep the reader on the same line, including an explicit End.
      void tick().then(() => {
        if (!viewport || !expanded) return
        if (atEnd) viewport.scrollTop = viewport.scrollHeight
        else if (anchor?.isConnected && anchorTop !== undefined) {
          const delta = anchor.getBoundingClientRect().top - anchorTop
          if (Math.abs(delta) >= 1) viewport.scrollTop += delta
        }
      })
    }, { root: viewport, rootMargin: '100% 0px' })
    observer.observe(node)
    return { destroy() { observer?.unobserve(node); nearby.delete(index) } }
  }
  onDestroy(() => observer?.disconnect())

  function rows(start: number) {
    return authored
      ? authored.cues.slice(start, start + groupSize).map((cue, i) => ({
          id: 'authored-' + (start + i), startMs: cue.startMs,
          original: cue.text, translation: null, stale: false,
        }))
      : p.cues.slice(start, start + groupSize)
  }

  function followCurrent() {
    if (!expanded || !viewport || !list || browsing || activeRow < 0) return
    const row = list.querySelector<HTMLElement>(`[data-row="${activeRow}"]`)
    if (!row) return
    const panel = viewport.getBoundingClientRect(), current = row.getBoundingClientRect()
    if (current.top < panel.top || current.bottom > panel.bottom) {
      const margin = Math.max(0, Math.min(panel.height / 3, panel.height - current.height))
      viewport.scrollTop += current.top - panel.top - margin
    }
  }

  $effect(() => {
    void p.track
    void p.seekRevision
    // A seek can resume following in the transcript. Leave the options
    // under the reader's pointer if they have scrolled back to settings.
    untrack(() => {
      if (viewport && list && list.getBoundingClientRect().top <= viewport.getBoundingClientRect().top) {
        browsing = false
      }
    })
  })
  $effect(() => {
    if (!expanded || !list || activeRow < 0 || browsing) return
    const frame = requestAnimationFrame(followCurrent)
    return () => cancelAnimationFrame(frame)
  })
</script>

<Disclosure label={p.t('subtitleList')} bind:expanded onrevealed={followCurrent}
  ontoggle={open => { if (open) browsing = false }}>
  {#snippet trailing()}
    <span class="row-count" aria-label={count + ' ' + p.t('lines')}>{count}</span>
  {/snippet}
  {#if !authored && p.cues.length}
    <div class="list-tools">
      <button onclick={() => (p.dialog = 'export')}><Icon name="export" size={14} />{p.t('exportSubs')}</button>
    </div>
  {/if}
  <div bind:this={list} role="region" aria-label={p.t('subtitleList')} class="cue-list">
    {#each Array(Math.ceil(count / groupSize)) as _, group}
      {@const start = group * groupSize}
      {@const visible = nearby.has(group) || (!browsing && activeRow >= start && activeRow < start + groupSize)}
      <div data-group={group} use:observeGroup={group}
        style:height={visible ? undefined : (heights.get(group) ?? Math.min(count - start, groupSize) * 54) + 'px'}>
        {#if visible}
          {#each rows(start) as cue, i (cue.id)}
            {@const index = start + i}
            {@const active = authored ? activeAuthored.includes(index) : activeRow === index}
            {@const text = authored ? [cue.original] : generatedLines(cue, p.settings)}
            {@const time = authored ? cue.startMs : Math.max(0, cue.startMs + subtitleTracks.delayMs)}
            <button class="cue-row" class:active data-row={index} aria-current={active}
              onclick={() => { browsing = false; p.seek(time) }}>
              <time>{clock(time)}</time>
              <span>
                <strong dir="auto" style:white-space={authored ? 'pre-line' : undefined}>
                  {text[0] ?? p.t('translating')}</strong>
                {#if text[1]}<span dir="auto">{text[1]}</span>{/if}
                {#if cue.stale}<small>{p.t('stale')}</small>{/if}
              </span>
            </button>
          {/each}
        {/if}
      </div>
    {/each}
    {#each pending.slice(0, 4) as span}
      <button class="cue-row pending" onclick={() => p.seek(span[0])}>
        <time>{clock(span[0])}</time><span>{p.t('transcribing')}</span>
      </button>
    {/each}
    {#if !count && !pending.length}<p class="empty-note">{p.t('waiting')}</p>{/if}
  </div>
</Disclosure>

<style>
  .row-count {
    margin-inline-start: auto;
    color: var(--muted);
    font-size: 12px;
    font-variant-numeric: tabular-nums;
  }
</style>

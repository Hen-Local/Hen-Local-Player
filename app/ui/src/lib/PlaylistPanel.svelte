<script module lang="ts">
  import { backend } from './stores.svelte'
  // Opening and closing the drawer should not repeatedly probe the same movie.
  let chapterLookup: { id: string; result: Promise<{ startMs: number; title: string }[]> } | undefined

  function chaptersFor(id: string) {
    if (chapterLookup?.id === id) return chapterLookup.result
    const result = backend.call('list_chapters', { sessionId: id }).catch(() => {
      if (chapterLookup?.id === id) chapterLookup = undefined
      return []
    })
    chapterLookup = { id, result }
    return result
  }
</script>

<script lang="ts">
  import SelectField from './SelectField.svelte'
  import { player as p } from './stores.svelte'
  import { mediaKind, type PlaylistSort } from './playlist'
  import { clock } from './format'
  import Icon from './Icon.svelte'
  let { onselect }: { onselect: () => void } = $props()
  const sortLabels = {
    original: 'sortOriginal', nameAsc: 'sortNameAsc', nameDesc: 'sortNameDesc',
    newest: 'sortNewest', oldest: 'sortOldest',
  } as const
  let list: HTMLDivElement
  let chapterRequest = $derived(p.media && chaptersFor(p.media.sessionId))
  let repeatLabel = $derived(p.t(p.repeat === 'all' ? 'repeatAll' : p.repeat === 'one' ? 'repeatOne' : 'repeatOff'))

  $effect(() => {
    const index = p.queueIndex
    const row = list?.children[index]
    if (!row) return
    const bounds = row.getBoundingClientRect()
    const viewport = list.getBoundingClientRect()
    if (bounds.top < viewport.top) list.scrollTop += bounds.top - viewport.top
    else if (bounds.bottom > viewport.bottom) list.scrollTop += bounds.bottom - viewport.bottom
  })
</script>

<div class="queue-toolbar">
  {#if p.queueFolder}
    <div class="queue-folder" title={p.queueFolder}>
      <Icon name="folder" size={14} />
      <span>{p.queueFolder.split(/[\\/]/).filter(Boolean).at(-1) || p.queueFolder}</span>
    </div>
  {/if}
  <div class="sort-control" class:sorted={p.queueSort !== 'original'} title={p.t('playlistSort') + ' · ' + p.t(sortLabels[p.queueSort])}>
    <SelectField compact displayLabel={p.t('playlistSort')}>
      <select aria-label={p.t('playlistSort')} value={p.queueSort}
        onchange={e => p.setQueueSort(e.currentTarget.value as PlaylistSort)}>
        {#each Object.entries(sortLabels) as [order, label]}
          <option value={order}>{p.t(label)}</option>
        {/each}
      </select>
    </SelectField>
  </div>
</div>
<div class="queue-list" bind:this={list}>
  {#each p.queue as item, index (item.path)}
    {@const kind = mediaKind(item.title)}
    <div class="queue-row" class:current={index === p.queueIndex}>
      <button
        class="queue-file"
        aria-current={index === p.queueIndex ? 'true' : undefined}
        title={item.path}
        onclick={() => { onselect(); void p.openQueueIndex(index) }}
      >
        <span class="queue-kind" role="img" aria-label={p.t(kind)}><Icon name={kind === 'audio' ? 'audio' : 'film'} size={15} /></span>
        <span dir="auto">{item.title}</span>
      </button>
      <button
        class="icon"
        aria-label={p.t('removeFromPlaylist') + ' · ' + item.title}
        title={p.t('removeFromPlaylist')}
        onclick={() => p.removeQueueItem(index)}
      >
        <Icon name="close" size={14} />
      </button>
    </div>
  {:else}
    <p class="empty-note">{p.t('emptyPlaylist')}</p>
  {/each}
</div>

{#if chapterRequest}
  {#await chapterRequest then chapters}
    {#if chapters.length}
      <div class="chapter-choice">
        <SelectField>
          <select aria-label={p.t('chapters')} onchange={(e) => {
            const choice = e.currentTarget
            if (choice.value) p.seek(Number(choice.value))
            choice.value = ''
          }}>
            <option value="">{p.t('chapters')} ({chapters.length})</option>
            {#each chapters as chapter, i}
              <option value={chapter.startMs}>{chapter.title || p.t('chapter') + ' ' + (i + 1)} · {clock(chapter.startMs)}</option>
            {/each}
          </select>
        </SelectField>
      </div>
    {/if}
  {/await}
{/if}
<div class="drawer-footer playlist-actions">
  <button class="outline add-files" aria-label={p.t('addFiles')} title={p.t('addFiles')} onclick={() => p.addToQueue()}>
    <Icon name="folder" size={15} /><span>{p.t('addFiles')}</span>
  </button>
  <div class="playlist-navigation" role="group" aria-label={p.t('playlist')}>
    <button class="repeat-mode" aria-pressed={p.repeat !== 'off'} aria-label={repeatLabel} title={repeatLabel}
      onclick={() => p.toggleRepeat()}>
      <Icon name={p.repeat === 'one' ? 'repeat-one' : 'repeat'} size={16} />
    </button>
  </div>
</div>

<style>
  .queue-toolbar {
    display: flex; align-items: center; justify-content: space-between; gap: 8px;
    flex: none; min-width: 0; padding: 5px var(--inspector-inset, 14px);
    color: var(--muted); font-size: 12px;
  }
  .queue-folder { display: flex; align-items: center; gap: 7px; min-width: 0; }
  .queue-folder :global(svg) { flex: none; }
  .queue-folder span { white-space: nowrap; text-overflow: ellipsis; overflow: hidden; }
  .sort-control { margin-inline-start: auto; max-width: 66%; min-width: 0; }
  .sort-control.sorted { --ink: var(--accent-text); }
  .queue-kind { display: grid; place-items: center; flex: none; width: 22px; color: var(--sub); }
  .current .queue-kind { color: var(--indicator-accent); filter: var(--indicator-shadow); }
  @media (forced-colors: active) {
    .current .queue-kind { color: Highlight; }
  }
</style>

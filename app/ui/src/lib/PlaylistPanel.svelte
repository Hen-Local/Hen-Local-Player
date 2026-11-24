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
  import { player as p } from './stores.svelte'
  import { clock } from './format'
  import Icon from './Icon.svelte'
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

{#if p.queueFolder}
  <div class="queue-folder" title={p.queueFolder}>
    <Icon name="folder" size={14} />
    <span>{p.queueFolder.split(/[\\/]/).filter(Boolean).at(-1) || p.queueFolder}</span>
  </div>
{/if}
<div class="queue-list" bind:this={list}>
  {#each p.queue as item, index (item.path)}
    <div class="queue-row" class:current={index === p.queueIndex}>
      <button
        class="queue-file"
        aria-current={index === p.queueIndex ? 'true' : undefined}
        title={item.path}
        onclick={() => p.openQueueIndex(index)}
      >
        <span class="queue-number"
          >{#if index === p.queueIndex}<Icon
              name="play"
              size={13}
            />{:else}{index + 1}{/if}</span
        >
        <span>{item.title}</span>
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
        <span class="select-box">
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
          <Icon name="down" size={13} />
        </span>
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
    <button title={p.t('previousFile')} aria-label={p.t('previousFile')} disabled={!p.media}
      onclick={() => p.previous()}><Icon name="previous" size={15} /></button>
    <button title={p.t('nextFile')} aria-label={p.t('nextFile')}
      disabled={!p.media || (p.queueIndex + 1 >= p.queue.length && p.repeat !== 'all')}
      onclick={() => p.next()}><Icon name="next" size={15} /></button>
  </div>
</div>

<style>
  .queue-folder { display: flex; align-items: center; flex: none; gap: 7px; min-width: 0; padding: 7px var(--inspector-inset, 14px); color: var(--muted); font-size: 12px; }
  .queue-folder span { white-space: nowrap; text-overflow: ellipsis; overflow: hidden; }
</style>

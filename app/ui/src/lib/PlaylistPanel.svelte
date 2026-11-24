<script lang="ts">
  import { player as p } from './stores.svelte'
  import Icon from './Icon.svelte'
  let list: HTMLDivElement

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

<style>
  .queue-folder { display: flex; align-items: center; flex: none; gap: 7px; min-width: 0; padding: 7px var(--inspector-inset, 14px); color: var(--muted); font-size: 12px; }
  .queue-folder span { white-space: nowrap; text-overflow: ellipsis; overflow: hidden; }
</style>
<div class="drawer-footer">
  <button class="outline" onclick={() => p.addToQueue()}
    ><Icon name="folder" size={15} />{p.t('addFiles')}</button
  >
</div>

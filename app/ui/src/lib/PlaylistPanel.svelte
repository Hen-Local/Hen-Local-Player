<script lang="ts">
  import { player as p } from './stores.svelte'
  import Icon from './Icon.svelte'
</script>

<div class="queue-list">
  {#each p.queue as item, index}
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
        aria-label={p.t('remove') + ' ' + item.title}
        title={p.t('remove')}
        onclick={() => p.removeQueueItem(index)}
      >
        <Icon name="close" size={14} />
      </button>
    </div>
  {:else}
    <p class="empty-note">{p.t('emptyPlaylist')}</p>
  {/each}
</div>
<div class="drawer-footer">
  <button class="outline" onclick={() => p.addToQueue()}
    ><Icon name="folder" size={15} />{p.t('addFiles')}</button
  >
</div>

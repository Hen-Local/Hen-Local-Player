<script lang="ts">
  import { player as p } from './stores.svelte'
  import { clock } from './format'
  import Icon from './Icon.svelte'

  let { onselect }: { onselect: () => void } = $props()
  async function open(path: string) {
    await p.open(path)
    if (!p.error) onselect()
  }
</script>

<div class="recent-shelf in-drawer">
  <div class="recent-list">
    {#each p.recent as file (file.path)}
      <button class="recent-row" title={file.path} onclick={() => open(file.path)}>
        <span class="file-icon"><Icon name="film" size={18}/></span>
        <span class="recent-content">
          <strong>{file.title}</strong>
          <small>{clock(file.durationMs)}</small>
        </span>
        <Icon name="right" size={14}/>
      </button>
    {:else}
      <p class="recent-empty">{p.t('noRecent')}</p>
    {/each}
  </div>
</div>

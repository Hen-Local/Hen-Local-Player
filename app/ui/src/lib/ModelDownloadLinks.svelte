<script lang="ts">
  import { invoke } from '@tauri-apps/api/core'
  import { player as p, mockMode } from './stores.svelte'
  import { componentNames } from './component-names'

  type Model = { id: string; files: { name?: string; url: string; mirror?: string }[] }
  let models = $state<Model[]>([])
  let selected = $state('')
  let model = $derived(models.find(item => item.id === selected))
  async function load() {
    if (models.length) return
    const { default: catalog } = await import('../../../../crates/hlp-models/src/components.json')
    models = catalog.filter(item => item.kind === 'model' && p.components.some(c => c.id === item.id))
    selected = models[0]?.id ?? ''
  }
  function follow(event: MouseEvent, url: string) {
    if (mockMode) return
    event.preventDefault()
    void p.act(() => invoke('plugin:opener|open_url', { url }))
  }
</script>

{#if p.components.length}
  <details class="model-links" ontoggle={(e) => { if (e.currentTarget.open) void p.act(load) }}>
    <summary>{p.t('manualModelLinks')}</summary>
    {#if models.length}
      <select bind:value={selected} aria-label={p.t('model')}>
        {#each models as item}<option value={item.id}>{componentNames[item.id]}</option>{/each}
      </select>
      <ul>
        {#each model?.files ?? [] as file}
          <li>
            <span>{file.name || file.url.split('/').pop()}</span>
            <a href={file.url} target="_blank" rel="noreferrer" onclick={(e) => follow(e, file.url)}>{p.t('international')}</a>
            {#if file.mirror}<a href={file.mirror} target="_blank" rel="noreferrer" onclick={(e) => follow(e, file.mirror!)}>{p.t('mainlandChina')}</a>{/if}
          </li>
        {/each}
      </ul>
    {/if}
  </details>
{/if}

<style>
  .model-links { margin: 10px 0 16px; font-size: 12px; }
  summary { cursor: pointer; }
  select { margin-block: 8px; width: 100%; }
  ul { list-style: none; padding: 0; margin: 0; }
  li { display: flex; align-items: baseline; gap: 10px; padding-block: 4px; }
  li span { flex: 1; min-width: 0; overflow-wrap: anywhere; }
  a { color: inherit; text-underline-offset: 2px; }
</style>

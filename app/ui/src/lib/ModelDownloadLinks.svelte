<script lang="ts">
  import SelectField from './SelectField.svelte'
  import Disclosure from './Disclosure.svelte'
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
  <div class="model-links">
    <Disclosure label={p.t('manualModelLinks')} ontoggle={open => { if (open) void p.act(load) }}>
      {#if models.length}
        <SelectField>
          <select bind:value={selected} aria-label={p.t('model')}>
            {#each models as item}<option value={item.id}>{componentNames[item.id]}</option>{/each}
          </select>
        </SelectField>
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
    </Disclosure>
  </div>
{/if}

<style>
  .model-links { margin: 10px 0 16px; font-size: 12px; }
  ul { list-style: none; padding: 0; margin: 8px 0 0; }
  li { display: flex; align-items: baseline; gap: 10px; padding-block: 4px; }
  li span { flex: 1; min-width: 0; overflow-wrap: anywhere; }
  a { color: inherit; text-underline-offset: 2px; }
</style>

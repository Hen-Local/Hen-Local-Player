<script lang="ts">
  import { onMount } from 'svelte'
  import Dialog from './Dialog.svelte'
  import { backend, player as p } from './stores.svelte'
  import { size } from './format'
  import type { Settings } from './types'
  import { componentNames } from './component-names'
  import Icon from './Icon.svelte'
  import ModelDownloadLinks from './ModelDownloadLinks.svelte'
  let { back }: { back?: () => void } = $props()
  let checking = $state(true), savingSource = $state(false)
  async function setSource(source: Settings['modelDownloadSource']) {
    savingSource = true
    try { await p.act(() => p.update({ modelDownloadSource: source })) }
    finally { savingSource = false }
  }
  let include = $state(false),
    busy = $state(false),
    error = $state('')
  let missing = $derived(
    p.components.filter(
      (c) =>
        !c.ready &&
        (c.id !== 'asr-qwen3-0.6b-mlx' || p.settings.asrModel === 'qwen3-asr-0.6b') &&
        (c.id !== 'asr-qwen3-1.7b-mlx' || p.settings.asrModel !== 'qwen3-asr-0.6b') &&
        (include || !['deno', 'yt-dlp'].includes(c.id)),
    ),
  )
  let running = $derived(
    busy || p.components.some((c) => c.state === 'downloading' || c.state === 'verifying'),
  )
  async function start(ids: string[]) {
    busy = true
    error = ''
    try {
      await backend.call('start_setup', { includeUrlTools: include, ids })
      p.components = await backend.call('get_setup_status')
    } catch (e) {
      error = String(e)
    } finally {
      busy = false
    }
  }
  onMount(
    () =>
      void p.act(async () => {
        try {
          p.components = await backend.call('get_setup_status')
        } finally {
          checking = false
        }
      }),
  )
</script>

<Dialog title={p.t('setup')} icon="download" {back}>
  <p class="help">{p.t('setupHint')}</p>
  <label>
    {p.t('modelDownloadSource')}
    <select disabled={running || savingSource} value={p.settings.modelDownloadSource}
      onchange={(e) => void setSource(e.currentTarget.value as Settings['modelDownloadSource'])}>
      <option value="auto">{p.t('autoModelSource')}</option>
      <option value="china">{p.t('chinaModelSource')}</option>
      <option value="global">{p.t('globalModelSource')}</option>
    </select>
  </label>
  <div class="component-list">
    {#each p.components as c (c.id)}
      <div class="component-row">
        <div class:good={c.ready} class="component-icon">
          <Icon name={c.ready ? 'check' : 'download'} size={16} />
        </div>
        <div class="component-name">
          <strong>{componentNames[c.id] ?? c.id}</strong>
          <small>
            {#if c.id === 'asr-qwen3-1.7b-mlx' || c.id === 'asr-qwen3-0.6b-sherpa'}{p.t('defaultAsr')} ·
            {:else if c.id === 'mt-hy-mt2-1.8b'}{p.t('defaultTranslation')} · {/if}
            {size(c.bytesTotal)} · {typeof c.state === 'object' ? p.t('failed') : p.t(c.state)}
          </small>
          {#if c.state === 'downloading' || c.state === 'verifying'}<progress
              value={c.bytesDone}
              max={c.bytesTotal || 1}
            ></progress>{:else if typeof c.state === 'object'}<small class="form-error">
              {c.state.failed}
            </small>{/if}
        </div>
        {#if !c.ready}<button
            class="icon"
            aria-label={p.t('downloadOne') + ' ' + c.id}
            title={p.t('retry')}
            disabled={running || savingSource}
            onclick={() => start([c.id])}
          >
            <Icon name="download" size={16} />
          </button>{/if}
      </div>
    {:else}<p role="status">{p.t(checking ? 'checkingModels' : 'noModel')}</p>{/each}
  </div>
  <ModelDownloadLinks />
  <label class="toggle">
    <input type="checkbox" bind:checked={include} />
    {p.t('includeTools')}
  </label>
  {#if error}<p class="form-error">{error}</p>{/if}
  <div class="dialog-actions">
    {#if running}<button onclick={() => p.act(() => backend.call('cancel_setup'))}>
        {p.t('cancel')}
      </button>{/if}
    <button
      class="primary"
      disabled={running || savingSource || !missing.length}
      onclick={() => start(missing.map((c) => c.id))}
    >
      {p.t('downloadAll')} ({size(missing.reduce((sum, c) => sum + c.bytesTotal - c.bytesDone, 0))})
    </button>
  </div>
</Dialog>

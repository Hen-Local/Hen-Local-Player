<script lang="ts">
  import { onMount } from 'svelte'
  import Dialog from './Dialog.svelte'
  import { backend, player as p } from './stores.svelte'
  import { size } from './format'
  import Icon from './Icon.svelte'
  let include = $state(false),
    busy = $state(false),
    error = $state('')
  const names: Record<string, string> = {
    'asr-qwen3-1.7b-mlx': 'Qwen3-ASR · 1.7B',
    'asr-qwen3-0.6b-mlx': 'Qwen3-ASR · 0.6B',
    'asr-qwen3-0.6b-sherpa': 'Qwen3-ASR · 0.6B',
    'vad-silero': 'Silero VAD',
    'mt-hy-mt2-1.8b': 'Hy-MT2 · 1.8B',
    'llama-runtime-b11429': 'llama.cpp',
    ffmpeg: 'FFmpeg',
    deno: 'Deno',
    'yt-dlp': 'yt-dlp',
  }
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
        p.components = await backend.call('get_setup_status')
      }),
  )
</script>

<Dialog title={p.t('setup')} icon="download">
  <p class="help">{p.t('setupHint')}</p>
  <div class="component-list">
    {#each p.components as c (c.id)}
      <div class="component-row">
        <div class:good={c.ready} class="component-icon">
          <Icon name={c.ready ? 'check' : 'download'} size={16} />
        </div>
        <div class="component-name">
          <strong>{names[c.id] ?? c.id}</strong>
          <small>
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
            disabled={running}
            onclick={() => start([c.id])}
          >
            <Icon name="download" size={16} />
          </button>{/if}
      </div>
    {:else}<p>{p.t('noModel')}</p>{/each}
  </div>
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
      disabled={running || !missing.length}
      onclick={() => start(missing.map((c) => c.id))}
    >
      {p.t('downloadAll')} ({size(missing.reduce((sum, c) => sum + c.bytesTotal - c.bytesDone, 0))})
    </button>
  </div>
</Dialog>

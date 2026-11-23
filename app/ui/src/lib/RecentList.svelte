<script lang="ts">
  import { onMount } from 'svelte'
  import { player as p, mockMode, backend } from './stores.svelte'
  import { clock } from './format'
  import Icon from './Icon.svelte'

  let { onselect, home = false }: { onselect?: () => void; home?: boolean } =
    $props()
  let samplePath = $state<string | null>(mockMode ? 'fixture_ja.mp4' : null)
  onMount(() => {
    if (!mockMode)
      void p.act(async () => {
        samplePath = await backend.call('get_sample_media')
      })
  })
  const latest = $derived(p.recent[0])
  const featured = $derived(latest?.path ?? samplePath)
  const isSample = $derived(!!samplePath && featured === samplePath)
  const featuredTitle = $derived(
    isSample ? p.t('demo') : (latest?.title ?? p.t('empty')),
  )
  const featuredFormat = $derived(
    featured?.split('.').pop()?.toUpperCase() ?? '',
  )
  const featuredDuration = $derived(
    latest?.durationMs ?? (isSample ? 22_000 : 0),
  )

  async function open(path: string) {
    await p.open(path)
    if (!p.error) onselect?.()
  }
</script>

{#if home}
  <section
    class="standby"
    class:sample-cover={isSample}
    aria-label={p.t('player')}
  >
    <div class="standby-art" aria-hidden="true">
      {#if isSample}<img src="/demo/poster.webp" alt="" />{:else}<div
          class="disc"
        >
          <div class="disc-hub"></div>
        </div>{/if}
    </div>
    <div class="standby-copy">
      {#if featured}
        <div class="standby-meta">
          <span>{featuredFormat}</span><i></i><time
            >{clock(featuredDuration)}</time
          >
        </div>
      {/if}
      <h2>{featuredTitle}</h2>
      {#if featured}
        <button class="screen-play" onclick={() => featured && open(featured)}>
          <Icon name="play" size={15} />{isSample ? p.t('sample') : p.t('play')}
        </button>
      {:else}
        <button class="screen-play" onclick={() => p.choose()}>
          <Icon name="folder" size={16} />{p.t('open')}
        </button>
      {/if}
    </div>
  </section>
{/if}

<section
  class="recent-shelf"
  class:in-drawer={!home}
  aria-label={p.t('recent')}
>
  <div class="shelf-heading">
    <h2>{p.t('recent')}</h2>
    <div class="shelf-actions">
      {#if samplePath && (!home || !isSample)}<button
          class="link-action"
          onclick={() => samplePath && open(samplePath)}
          ><Icon name="play" size={13} />{p.t('sample')}</button
        >{/if}
      {#if home}<button class="link-action" onclick={() => p.chooseFolder()}>
          <Icon name="folder" size={16} />{p.t('openFolder')}
        </button>{/if}
    </div>
  </div>
  <div class="recent-list">
    {#each p.recent as file (file.path)}
      <button
        class="recent-row"
        title={file.path}
        onclick={() => open(file.path)}
      >
        <span class="file-icon"><Icon name="film" size={18} /></span>
        <span class="recent-content">
          <strong>{file.path === samplePath ? p.t('demo') : file.title}</strong>
          <small>{clock(file.durationMs)}</small>
        </span>
        <Icon name="right" size={14} />
      </button>
    {/each}
    {#if !p.recent.length}
      <p class="recent-empty">{p.t('noRecent')}</p>
    {/if}
  </div>
</section>

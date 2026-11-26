<script lang="ts">
  import { tick } from 'svelte'
  import { backend, mockMode, player as p } from './stores.svelte'
  import { mediaKind } from './playlist'
  import { clock } from './format'
  import Icon from './Icon.svelte'

  const previewCount = 4
  const revealLabel = /Mac/.test(navigator.platform) ? 'openInFinder' : 'showInFolder'
  let expanded = $state(false)
  let list = $state<HTMLDivElement>()
  const visibleItems = $derived(expanded ? p.recent : p.recent.slice(0, previewCount))
  export async function showAll() {
    if (!p.recent.length) { p.toast(p.t('noRecent')); return }
    expanded = p.recent.length > previewCount
    await tick()
    if (list) list.scrollTop = 0
    list?.querySelector('button')?.focus({ preventScroll: true })
  }
  function toggle() {
    expanded = !expanded
    if (list) list.scrollTop = 0
  }
</script>

{#if p.recent.length}
  <nav class="welcome-history" aria-labelledby="recent-title">
    <h2 id="recent-title">{p.t('recent')}</h2>
    <div id="recent-files" class="welcome-recent" class:expanded bind:this={list}>
      {#each visibleItems as file, index (file.path)}
        <div class="welcome-history-row">
          <button class="recent-play" title={file.path} onclick={() => p.open(file.path)}>
            <Icon name={mediaKind(file.title) === 'audio' ? 'audio' : 'film'} size={15}/>
            <span id={"recent-name-" + index} dir="auto">{file.title}</span>
          </button>
          {#if !mockMode}
            <button class="recent-reveal" aria-describedby={"recent-name-" + index} title={p.t(revealLabel)}
              onclick={() => p.act(() => backend.call('reveal_in_folder', { path: file.path }))}>
              <span>{p.t(revealLabel)}</span>
            </button>
          {/if}
          <span class="recent-duration" dir="ltr">{file.durationMs > 0 ? clock(file.durationMs) : '–:––'}</span>
        </div>
      {/each}
    </div>
    {#if p.recent.length > previewCount}
      <button class="history-toggle" aria-controls="recent-files" aria-expanded={expanded} onclick={toggle}>
        {p.t(expanded ? 'showLess' : 'showMore')}<Icon name="down" size={14}/>
      </button>
    {/if}
  </nav>
{/if}

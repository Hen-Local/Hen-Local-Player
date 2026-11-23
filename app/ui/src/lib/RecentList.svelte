<script lang="ts">
  import { player as p, mockMode } from './stores.svelte'
  import { clock } from './format'
  import Icon from './Icon.svelte'
</script>

<aside class="side-panel">
  <div class="panel-title">
    <Icon name="clock" />
    <h2>{p.t('recent')}</h2>
  </div>
  {#if !p.recent.length && !mockMode}<p class="subtle empty-note">{p.t('noRecent')}</p>{/if}
  {#each p.recent as r (r.path)}<button class="recent-row" onclick={() => p.open(r.path)}>
      <span class="file-icon"><Icon name="film" /></span>
      <span class="recent-content">
        <strong>{r.title}</strong>
        <small>{clock(r.durationMs)} · {Math.round(r.progress * 100)}%</small>
      </span>
      <Icon name="right" size={14} />
    </button>{/each}
  {#if mockMode && !p.recent.some((r) => r.path === 'fixture_ja.mp4')}<button
      class="recent-row"
      onclick={() => p.open('fixture_ja.mp4')}
    >
      <span class="file-icon"><Icon name="film" /></span>
      <span class="recent-content">
        <strong>{p.t('demo')}</strong>
        <small>00:22 · MP4</small>
      </span>
      <Icon name="play" size={13} />
    </button>{/if}
</aside>

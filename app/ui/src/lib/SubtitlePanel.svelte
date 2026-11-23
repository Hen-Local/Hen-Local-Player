<script lang="ts">
  import { player as p, modes } from './stores.svelte'
  import Icon from './Icon.svelte'
  import LanguageControls from './LanguageControls.svelte'
  import SubtitleTracks from './SubtitleTracks.svelte'
  import { subtitleTracks } from './subtitle-tracks.svelte'
</script>

<div class="inspector-body">
  <SubtitleTracks />
  {#if subtitleTracks.selected === 'generated'}
    <LanguageControls />
    <div class="mode-control">
      <span class="field-label">{p.t('display')}</span>
      <div class="mode-group">
        {#each modes as mode}
          <button
            class:selected={p.settings.subtitleMode === mode}
            aria-pressed={p.settings.subtitleMode === mode}
            onclick={() => p.act(() => subtitleTracks.setMode(mode))}
            >{p.t(mode)}</button
          >
        {/each}
      </div>
    </div>
    <button class="outline wide" onclick={() => (p.dialog = 'export')}
      ><Icon name="export" size={16} />{p.t('exportSubs')}</button
    >
  {/if}
  <button class="link-action" onclick={() => (p.dialog = 'setup')}
    ><Icon name="download" size={15} />{p.t(
      p.status?.stage === 'needs-setup' &&
        subtitleTracks.selected === 'generated'
        ? 'needsSetup'
        : 'models',
    )}</button
  >
</div>

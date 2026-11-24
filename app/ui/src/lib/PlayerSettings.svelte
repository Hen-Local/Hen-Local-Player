<script lang="ts">
  import { tick } from 'svelte'
  import { player as p } from './stores.svelte'
  import Icon from './Icon.svelte'
  import SubtitlePanel from './SubtitlePanel.svelte'
  import { slidePanel } from './panel-transition'

  let { open, onfocus, onclose }: {
    open: boolean
    onfocus: () => void
    onclose: () => void
  } = $props()

  async function regenerate() {
    const request = p.startTranscription({ regenerate: true })
    await tick()
    document.querySelector<HTMLElement>('#player-settings .subtitle-workspace')?.focus({ preventScroll: true })
    await request
  }
</script>

{#if open}
  <aside id="player-settings" class="inspector right-inspector" aria-labelledby="subtitle-panel-title"
    onfocusin={onfocus} transition:slidePanel={'right'}>
    <div class="panel-title">
      <h2 id="subtitle-panel-title">{p.t('subtitles')}</h2>
      <button class="icon panel-close" title={p.t('close')} aria-label={p.t('close')}
        onclick={onclose}><Icon name="close" size={16} /></button>
    </div>
    <SubtitlePanel />
    {#if p.canRegenerate}
      <div class="regenerate-footer">
        <button class="regenerate-button" onclick={regenerate}>
          <Icon name="refresh" size={15} />{p.t('regenerateSubtitles')}
        </button>
      </div>
    {/if}
  </aside>
{/if}

<style>
  .regenerate-footer {
    display: flex;
    flex: none;
    justify-content: center;
    padding: 8px var(--inspector-inset);
    border-top: 1px solid var(--hairline);
  }
  .regenerate-button {
    gap: 6px;
    min-height: 32px;
    padding: 5px 11px;
    font-size: 12px;
    font-weight: 600;
  }
</style>

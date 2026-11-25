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
    {#if p.canDeleteSubtitles}
      <div class="regenerate-footer">
        {#if p.canRegenerate}
          <button class="regenerate-button" aria-label={p.t('regenerateSubtitles')} title={p.t('regenerateSubtitles')}
            disabled={p.deletingSubtitles} onclick={regenerate}>
            <Icon name="refresh" size={15} />{p.t('regenerate')}
          </button>
        {/if}
        <button class="delete-button" aria-label={p.t('deleteGeneratedSubtitles')} title={p.t('deleteGeneratedSubtitles')}
          disabled={p.deletingSubtitles} aria-haspopup="dialog" onclick={() => p.confirmSubtitleDeletion()}>
          <Icon name="trash" size={15} />{p.t('deleteAction')}
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
    gap: 6px;
    padding: 8px var(--inspector-inset);
  }
  .regenerate-footer button {
    gap: 4px;
    min-height: 32px;
    padding: 5px 8px;
    font-size: 12px;
    font-weight: 600;
  }
</style>

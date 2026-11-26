<script lang="ts">
  import type { Snippet } from 'svelte'
  import { cubicOut } from 'svelte/easing'
  import Icon from './Icon.svelte'

  let { label, children, trailing, expanded = $bindable(false), disabled = false, ontoggle, onrevealed }: {
    label: string
    children: Snippet
    trailing?: Snippet
    expanded?: boolean
    disabled?: boolean
    onrevealed?: () => void
    ontoggle?: (expanded: boolean) => void
  } = $props()
  const id = $props.id()

  function reveal(_node: HTMLElement) {
    return {
      duration: matchMedia('(prefers-reduced-motion: reduce)').matches ? 0 : 155,
      easing: cubicOut,
      css: (t: number) => `grid-template-rows: minmax(0, ${t}fr); overflow: clip`,
    }
  }
</script>

<div class="disclosure">
  <button type="button" class="disclosure-trigger" {disabled} aria-expanded={expanded}
    aria-controls={expanded ? id : undefined} onclick={() => {
      expanded = !expanded
      ontoggle?.(expanded)
    }}>
    <Icon name="right" size={15} />
    <span>{label}</span>
    {#if trailing}{@render trailing()}{/if}
  </button>
  {#if expanded}
    <div class="disclosure-content" id={id} transition:reveal onintroend={onrevealed}>
      <div class="disclosure-body">{@render children()}</div>
    </div>
  {/if}
</div>

<style>
  .disclosure { min-width: 0; flex: none; }
  .disclosure-content {
    display: grid;
    grid-template-rows: minmax(0, 1fr);
  }
  .disclosure-body { min-height: 0; align-self: start; }
  :global(.app :is(.inspector, .player-dialog)) .disclosure-trigger {
    width: 100%; min-height: 36px; flex: none; gap: 7px;
    justify-content: flex-start; padding: 6px 8px;
    border-radius: 8px; background: transparent;
    font-size: 13px; font-weight: 600; line-height: 1.35; text-align: start;
  }
  :global(.inspector) .disclosure-trigger {
    margin-inline: 6px; max-width: calc(100% - 12px);
    margin-block: 2px;
  }
  :global(.app :is(.inspector, .player-dialog)) .disclosure-trigger:hover:not(:disabled) {
    background: var(--hover);
  }
  :global(.app :is(.inspector, .player-dialog)) .disclosure-trigger:focus-visible { outline-offset: -2px; }
  .disclosure-trigger :global(svg) {
    flex: none;
    color: var(--sub);
    transition: transform 155ms cubic-bezier(.333333, 1, .666667, 1);
  }
  .disclosure-trigger:dir(rtl) :global(svg) { transform: rotate(180deg); }
  .disclosure-trigger[aria-expanded='true'] :global(svg) { transform: rotate(90deg); }
  @media (prefers-reduced-motion: reduce) {
    .disclosure-trigger :global(svg) { transition: none; }
  }
</style>

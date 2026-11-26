<script lang="ts">
  import type { Snippet } from 'svelte'
  import Icon from './Icon.svelte'

  let { children, swatch, displayLabel, compact = false }: {
    children: Snippet
    swatch?: string
    displayLabel?: string
    compact?: boolean
  } = $props()
</script>

<span class="select-field-control" class:compact class:with-swatch={!!swatch} class:with-label={displayLabel !== undefined}>
  {@render children()}
  {#if swatch}<i aria-hidden="true" style:background-color={swatch}></i>{/if}
  {#if displayLabel !== undefined}<span class="display-label" aria-hidden="true">{displayLabel}</span>{/if}
  <Icon name="down" size={15} />
</span>

<style>
  .select-field-control {
    position: relative;
    display: grid;
    align-items: center;
    min-inline-size: 0;
    min-block-size: var(--field-height, 36px);
    border-radius: 8px;
    background: var(--control-fill);
    color: var(--ink);
    box-shadow: inset 0 1px 0 rgb(255 255 255 / 5%);
  }
  .select-field-control:has(:global(select:not(:disabled))):hover { background: var(--hover); }
  .select-field-control:has(:global(select:focus-visible)) {
    outline: 2px solid var(--glass-focus);
    outline-offset: 2px;
  }
  .select-field-control:has(:global(select:disabled)) { opacity: .52; }
  .select-field-control :global(select), .display-label {
    grid-area: 1 / 1;
    box-sizing: border-box;
    inline-size: 100%;
    min-inline-size: 0;
    min-block-size: var(--field-height, 36px);
    padding-block: 0;
    padding-inline: 11px 31px;
    font: inherit;
    font-size: 13px;
    font-weight: 500;
    line-height: 1.35;
  }
  .select-field-control :global(select) {
    appearance: none;
    -webkit-appearance: none;
    display: block;
    border: 0;
    border-radius: inherit;
    color: inherit;
    background: transparent;
    text-overflow: ellipsis;
  }
  .select-field-control :global(select:focus-visible) { outline: none; }
  .select-field-control :global(select:disabled) { opacity: 1; }
  .select-field-control :global(svg) {
    pointer-events: none;
    position: absolute;
    inset-inline-end: 9px;
  }
  .with-swatch i {
    position: absolute;
    pointer-events: none;
    inset-inline-start: 11px;
    inline-size: 11px;
    block-size: 11px;
    border-radius: 50%;
    box-shadow: inset 0 1px 1px rgb(255 255 255 / 30%);
  }
  .with-swatch :global(select) { padding-inline-start: 30px; }
  .with-label :global(select) { position: absolute; inset: 0; block-size: 100%; opacity: 0; }
  .display-label {
    display: flex;
    align-items: center;
    padding-block: 5px;
    overflow-wrap: anywhere;
    pointer-events: none;
  }
  .compact { inline-size: fit-content; }
  .compact .display-label { white-space: nowrap; overflow: hidden; display: block; align-content: center; }
  @media (forced-colors: active) {
    .select-field-control { border: 1px solid ButtonText; }
    .select-field-control:has(:global(select:focus-visible)) { outline-color: Highlight; }
  }
</style>

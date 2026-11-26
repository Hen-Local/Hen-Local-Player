<script lang="ts">
  import { onMount, type Snippet } from 'svelte'
  import { player as p } from './stores.svelte'
  import Icon from './Icon.svelte'
  let {
    title,
    icon,
    children,
    back,
    navigation,
    close = () => {
      p.dialog = null
    },
  }: { title: string; icon: string; children: Snippet; navigation?: Snippet; back?: () => void; close?: () => void } = $props()
  let dialog: HTMLDialogElement
  let heading: HTMLHeadingElement
  const headingId = $props.id()
  const headingText = $derived(title.replace(/(?:…|\.{3})$/, '').trimEnd())
  onMount(() => {
    const previous = document.activeElement as HTMLElement | null
    dialog.showModal()
    if (navigation || dialog.querySelector('.dialog-heading')?.contains(document.activeElement)) {
      heading.focus({ preventScroll: true })
    }
    return () => {
      dialog.close()
      if (previous?.isConnected) previous.focus({ preventScroll: true })
    }
  })
</script>

<dialog
  class="player-dialog"
  class:preferences={!!navigation}
  class:roomy={!!navigation || !!back}
  class:sheet={!navigation && !back}
  bind:this={dialog}
  oncancel={(e) => {
    e.preventDefault()
    if (back) back()
    else close()
  }}
  onclick={(e) => {
    if (e.target === dialog) {
      const r = dialog.getBoundingClientRect()
      if (e.clientX < r.left || e.clientX > r.right || e.clientY < r.top || e.clientY > r.bottom)
        close()
    }
  }}
  aria-labelledby={headingId}
>
  <div class="dialog-heading">
    {#if back}
      <button class="back-to-settings" aria-label={p.t('backToSettings')} onclick={back}>
        <span><Icon name="right" size={14} /></span>{p.t('back')}
      </button>
    {:else}
      <span class="heading-icon"><Icon name={icon} size={19} /></span>
    {/if}
    <h2 id={headingId} bind:this={heading} tabindex="-1">{headingText}</h2>
    <span class="spacer"></span>
    <button class="icon" aria-label={p.t('close')} onclick={close}><Icon name="close" /></button>
  </div>
  {@render navigation?.()}
  <div class="dialog-body">
    {#if p.error}<p class="form-error" role="alert">
        {p.error}
        <button class="icon" aria-label={p.t('dismiss')} onclick={() => (p.error = '')}>
          <Icon name="close" size={14} />
        </button>
      </p>{/if}{@render children()}
  </div>
</dialog>

<style>
  :global(.app) .player-dialog {
    --glass-fill: var(--glass-sheet-fill);
  }
  /* The browser preview may composite a top-layer dialog without sampling the
     scene. Desktop playback already supplies the live blur inside the glass. */
  :global(.app:has(.player-dialog[open]) :is(.welcome, .welcome-tools)) { filter: blur(5px); }
  :global(.watching:where(:not(.desktop)):has(.player-dialog[open]) > :is(.inspector, .player-surface)) { filter: blur(3.5px); }
  @media (prefers-reduced-transparency: reduce), (forced-colors: active), (prefers-contrast: more) {
    :global(.app:has(.player-dialog[open]) :is(.welcome, .welcome-tools, .inspector, .player-surface)) { filter: none; }
  }
  @media (prefers-reduced-motion: no-preference) {
    :global(:is(.welcome, .welcome-tools, .watching:not(.desktop) > .player-surface)) { transition: filter 160ms ease-out; }
  }
  /* The native material remains visible around the sheet, including its first frame. */
  :global(.app) .player-dialog::backdrop { background: transparent; }
  :global(.watching) .player-dialog::backdrop { background: rgb(30 32 38 / 15%); }
  @media (prefers-reduced-motion: no-preference) {
    .player-dialog[open] { animation: reveal-dialog 220ms cubic-bezier(.24,.8,.32,1); }
    :global(.watching) .player-dialog[open]::backdrop { animation: present-scrim 220ms ease-out; }
  }
  /* Keep the glass and its text in their final positions as the sheet uncovers them. */
  @keyframes reveal-dialog {
    from { clip-path: inset(0 -64px 100% -64px); }
    to { clip-path: inset(0 -64px -64px -64px); }
  }
  @keyframes present-scrim { from { opacity: 0; } to { opacity: 1; } }
  :global(.app) .player-dialog.roomy {
    width: min(820px, calc(100vw - 24px));
    height: min(540px, calc(100dvh - 24px));
    max-height: calc(100dvh - 24px);
  }
  /* Leave the native traffic lights on their own strip in a windowed Mac player. */
  :global(.native-mac .app:not(.fullscreen)) .player-dialog {
    inset-block: 32px 6px;
    max-height: calc(100dvh - 44px);
  }
  :global(.app) .player-dialog.sheet[open] {
    inset-block: var(--stage-header) auto;
    margin-block: 0;
    max-height: calc(100dvh - var(--stage-header) - 8px);
    border-radius: 0 0 16px 16px;
  }
  :global(.watching:has(.player-dialog.sheet[open]) .window-chrome) { opacity: 1; }
  :global(.app) .player-dialog.preferences[open] {
    display: grid;
    grid-template-columns: minmax(128px, 21%) minmax(0, 1fr);
    grid-template-rows: auto minmax(0, 1fr);
  }
  .preferences .dialog-heading { grid-column: 1 / -1; }
  .dialog-heading h2:focus { outline: 0; }
  :global(.app) .preferences .dialog-body { padding: 16px 22px 20px; scrollbar-gutter: stable; }
  @media (max-width: 575px) {
    :global(.app) .player-dialog.preferences[open] {
      grid-template-columns: minmax(0, 1fr);
      grid-template-rows: auto auto minmax(0, 1fr);
    }
    :global(.app) .preferences .dialog-body { padding: 10px 14px 16px; }
  }
  .back-to-settings { gap: 3px; font-size: 12px; }
  .back-to-settings span { display: flex; transform: rotate(180deg); }
  :global(html[dir="rtl"]) .back-to-settings span { transform: none; }
</style>

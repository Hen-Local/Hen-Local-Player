<script lang="ts">
  import { onMount, type Snippet } from 'svelte'
  import { player as p } from './stores.svelte'
  import Icon from './Icon.svelte'
  let {
    title,
    icon,
    children,
    back,
    close = () => {
      p.dialog = null
    },
  }: { title: string; icon: string; children: Snippet; back?: () => void; close?: () => void } = $props()
  let dialog: HTMLDialogElement
  onMount(() => {
    const previous = document.activeElement as HTMLElement | null
    dialog.showModal()
    return () => {
      dialog.close()
      previous?.focus()
    }
  })
</script>

<dialog
  class="player-dialog"
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
  aria-label={title}
>
  <div class="dialog-heading">
    {#if back}
      <button class="back-to-settings" aria-label={p.t('backToSettings')} onclick={back}>
        <span><Icon name="right" size={14} /></span>{p.t('back')}
      </button>
    {:else}
      <span class="heading-icon"><Icon name={icon} size={19} /></span>
    {/if}
    <h2>{title}</h2>
    <span class="spacer"></span>
    <button class="icon" aria-label={p.t('close')} onclick={close}><Icon name="close" /></button>
  </div>
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
  .back-to-settings { gap: 3px; font-size: 12px; }
  .back-to-settings span { display: flex; transform: rotate(180deg); }
</style>

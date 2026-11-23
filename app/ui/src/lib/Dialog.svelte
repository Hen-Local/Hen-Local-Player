<script lang="ts">
  import { onMount, type Snippet } from 'svelte'
  import { player as p } from './stores.svelte'
  import Icon from './Icon.svelte'
  let {
    title,
    icon,
    children,
    close = () => {
      p.dialog = null
    },
  }: { title: string; icon: string; children: Snippet; close?: () => void } = $props()
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
    close()
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
    <span class="heading-icon"><Icon name={icon} size={19} /></span>
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

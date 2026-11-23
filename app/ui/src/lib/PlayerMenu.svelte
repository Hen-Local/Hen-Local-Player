<script lang="ts">
  import { onMount } from 'svelte'
  import { player as p } from './stores.svelte'
  import Icon from './Icon.svelte'

  let { hasMedia, canSample, onaction, ondismiss }: {
    hasMedia: boolean; canSample: boolean
    onaction: (id: string) => void; ondismiss: (restore: boolean) => void
  } = $props()
  let menu: HTMLDivElement
  type Entry = { id: string; label: Parameters<typeof p.t>[0]; icon: string }
  const groups = $derived.by((): Entry[][] => {
    const files: Entry[] = [{ id: 'folder', label: 'openFolder', icon: 'folder' }]
    if (hasMedia) {
      files.unshift({ id: 'open', label: 'open', icon: 'folder' })
      files.push({ id: 'url', label: 'openUrlAction', icon: 'link' })
    }
    files.push({ id: 'library', label: 'recent', icon: 'clock' })
    if (!hasMedia && canSample) files.push({ id: 'sample', label: 'sample', icon: 'play' })
    const other: Entry[] = [{ id: 'settings', label: 'settings', icon: 'settings' }]
    if (hasMedia) other.push({ id: 'close', label: 'closeMedia', icon: 'close' })
    return [files, other]
  })
  function key(e: KeyboardEvent) {
    const items = Array.from(menu.querySelectorAll('button'))
    const current = items.indexOf(document.activeElement as HTMLButtonElement)
    let next: number
    switch (e.key) {
      case 'ArrowDown': next = (current + 1) % items.length; break
      case 'ArrowUp': next = (current - 1 + items.length) % items.length; break
      case 'Home': next = 0; break
      case 'End': next = items.length - 1; break
      case 'Escape': e.preventDefault(); e.stopPropagation(); ondismiss(true); return
      default: return
    }
    e.preventDefault()
    e.stopPropagation()
    items[next].focus()
  }
  function outside(e: Event) {
    if (e.target instanceof Element && !menu.contains(e.target) && !e.target.closest('.more-trigger'))
      ondismiss(false)
  }
  onMount(() => menu.querySelector('button')?.focus({ preventScroll: true }))
</script>

<svelte:window onpointerdown={outside} onfocusin={outside}/>
<div class="player-menu" role="menu" aria-label={p.t('more')} bind:this={menu} onkeydown={key} tabindex="-1">
  {#each groups as group, i}
    {#if i}<div class="menu-divider" role="separator"></div>{/if}
    {#each group as item}
      <button role="menuitem" onclick={() => onaction(item.id)}>
        <Icon name={item.icon} size={15}/><span>{p.t(item.label)}</span>
      </button>
    {/each}
  {/each}
</div>

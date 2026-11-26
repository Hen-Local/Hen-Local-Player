<script lang="ts">
  import { onDestroy, tick } from 'svelte'
  import { player as p } from './stores.svelte'
  import Icon from './Icon.svelte'

  type AirPlayVideo = HTMLVideoElement & {
    webkitShowPlaybackTargetPicker?: () => void
    webkitCurrentPlaybackTargetIsWireless?: boolean
  }

  let { pipAvailable, pipActive, onpip, onopen }: {
    pipAvailable: boolean
    pipActive: boolean
    onpip: () => void
    onopen: (open: boolean) => void
  } = $props()
  let root = $state<HTMLDivElement>(), trigger = $state<HTMLButtonElement>(), menu = $state<HTMLDivElement>()
  let open = $state<'hover' | 'active' | null>(null)
  let receiverAvailable = $state(false), wireless = $state(false)
  let timer = 0
  const video = $derived<AirPlayVideo | null>(p.video)
  const supportsAirPlay = $derived(typeof video?.webkitShowPlaybackTargetPicker === 'function')
  const canRoute = $derived(receiverAvailable || wireless)
  const expanded = $derived(!!open)
  $effect(() => onopen(expanded))

  // WebKit starts route discovery when this event has a listener. Limit it
  // to the open output menu and keep an already-connected route selectable.
  $effect(() => {
    const element = video
    if (!expanded || !element || !supportsAirPlay) return
    wireless = !!element.webkitCurrentPlaybackTargetIsWireless
    receiverAvailable = false
    const availability = (event: Event) => {
      receiverAvailable = (event as Event & { availability: string }).availability === 'available'
    }
    const routeChanged = () => { wireless = !!element.webkitCurrentPlaybackTargetIsWireless }
    element.addEventListener('webkitplaybacktargetavailabilitychanged', availability)
    element.addEventListener('webkitcurrentplaybacktargetiswirelesschanged', routeChanged)
    return () => {
      element.removeEventListener('webkitplaybacktargetavailabilitychanged', availability)
      element.removeEventListener('webkitcurrentplaybacktargetiswirelesschanged', routeChanged)
    }
  })
  $effect(() => {
    if (!expanded) return
    const outside = (event: Event) => {
      if (event.target instanceof Node && !root?.contains(event.target)) open = null
    }
    document.addEventListener('pointerdown', outside, true)
    document.addEventListener('focusin', outside)
    return () => {
      document.removeEventListener('pointerdown', outside, true)
      document.removeEventListener('focusin', outside)
    }
  })

  function hover(event: PointerEvent) {
    clearTimeout(timer)
    if (event.pointerType === 'mouse' && !open) open = 'hover'
  }
  function leave() {
    clearTimeout(timer)
    if (open === 'hover') timer = window.setTimeout(() => {
      if (open === 'hover') open = null
    }, 180)
  }
  function close(restoreFocus = true) {
    clearTimeout(timer)
    open = null
    if (restoreFocus) trigger?.focus({ preventScroll: true })
  }
  function items() { return Array.from(menu?.querySelectorAll<HTMLButtonElement>('button') ?? []) }
  async function focusItem(last = false) {
    open = 'active'
    await tick()
    const choices = items()
    choices[last ? choices.length - 1 : 0]?.focus({ preventScroll: true })
  }
  function toggle(event: MouseEvent) {
    if (open === 'active') close()
    else if (event.detail === 0) void focusItem()
    else open = 'active'
  }
  function key(event: KeyboardEvent) {
    if (!root?.contains(event.target as Node) || event.altKey || event.metaKey || event.ctrlKey) return
    if (event.target === trigger && (event.key === 'ArrowDown' || event.key === 'ArrowUp')) {
      event.preventDefault()
      void focusItem(event.key === 'ArrowUp')
      return
    }
    if (!open || !menu?.contains(event.target as Node)) return
    const choices = items(), index = choices.indexOf(event.target as HTMLButtonElement)
    let next: number
    if (event.key === 'Home') next = 0
    else if (event.key === 'End') next = choices.length - 1
    else if (event.key === 'ArrowDown') next = (index + 1) % choices.length
    else if (event.key === 'ArrowUp') next = (index + choices.length - 1) % choices.length
    else return
    event.preventDefault()
    choices[next].focus({ preventScroll: true })
  }
  function route() {
    const element = video
    if (!element?.webkitShowPlaybackTargetPicker || !canRoute) return
    // Keep the native picker call in the activating click's user gesture.
    element.webkitShowPlaybackTargetPicker()
    close()
  }
  onDestroy(() => { clearTimeout(timer); onopen(false) })
</script>

<svelte:window onblur={() => close(false)} onkeydowncapture={(event) => {
  if (open && event.key === 'Escape') {
    event.preventDefault()
    close()
  } else key(event)
}} />
{#if pipAvailable || supportsAirPlay}
  <div bind:this={root} class="outputs" role="group" aria-label={p.t('more')}
    onpointerenter={hover} onpointerleave={leave}>
    <button bind:this={trigger} class="float-button output-trigger" title={p.t('more')} aria-label={p.t('more')}
      aria-haspopup="menu" aria-expanded={!!open} aria-controls={open ? 'playback-outputs' : undefined}
      onpointerenter={hover} onclick={toggle}><Icon name="more" size={19} /></button>
    {#if open}
      <div bind:this={menu} id="playback-outputs" role="menu" aria-label={p.t('more')} dir={p.locale === 'ar' || p.locale === 'fa' ? 'rtl' : 'ltr'}>
        {#if pipAvailable}
          <button role="menuitemcheckbox" aria-checked={pipActive} tabindex="-1"
            onclick={() => { onpip(); close() }}>
            <Icon name="pip" /><span>{p.t('pictureInPicture')}</span>
            {#if pipActive}<Icon name="check" size={13} />{/if}
          </button>
        {/if}
        {#if supportsAirPlay}
          <button role="menuitemcheckbox" aria-checked={wireless} aria-disabled={!canRoute}
            title={canRoute ? 'AirPlay' : p.t('noAirPlayDevices')} tabindex="-1" onclick={route}>
            <Icon name="airplay" /><span>AirPlay</span>
            {#if wireless}<Icon name="check" size={13} />{/if}
          </button>
        {/if}
      </div>
    {/if}
  </div>
{/if}

<style>
  .outputs { position: relative; display: flex; flex: none; }
  .output-trigger[aria-expanded='true']::before { content: ''; position: absolute; inset: -14px 0 100%; }
  [role='menu'] {
    position: absolute; right: 0; bottom: calc(100% + 10px); z-index: 1;
    width: max-content; max-width: min(260px, 85vw); padding: 5px; border-radius: 13px;
    background: #25272c;
    box-shadow: inset 0 1px 0 rgb(255 255 255 / 12%), 0 6px 20px rgb(0 0 0 / 28%);
    transform-origin: right bottom; animation: menu-in 130ms ease-out;
  }
  @supports ((backdrop-filter: blur(1px)) or (-webkit-backdrop-filter: blur(1px))) {
    [role='menu'] {
      background: var(--glass-sheet-fill);
      -webkit-backdrop-filter: var(--glass-backdrop); backdrop-filter: var(--glass-backdrop);
    }
  }
  [role='menu'] button {
    display: flex; width: 100%; align-items: center; gap: 8px; min-height: 33px;
    padding: 6px 9px; border: 0; border-radius: 8px; background: transparent;
    color: #f7f8fa; text-align: start; font: inherit; font-size: 13px; line-height: 1.2;
  }
  [role='menu'] button span { flex: 1; }
  [role='menu'] button:focus-visible { background: var(--surface-selection); }
  [role='menu'] button[aria-disabled='true'] { color: #bdc2ca; background: transparent; cursor: default; }
  @keyframes menu-in {
    from { opacity: 0; transform: translateY(3px) scale(.98); }
    to { opacity: 1; transform: none; }
  }
  @media (prefers-reduced-motion: reduce) { [role='menu'] { animation: none; } }
  @media (prefers-reduced-transparency: reduce), (prefers-contrast: more) {
    [role='menu'] { background: #25272c; -webkit-backdrop-filter: none; backdrop-filter: none; }
  }
</style>

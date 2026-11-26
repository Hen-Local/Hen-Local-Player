<script lang="ts">
  let { name, size = 18 }: { name: string; size?: number } = $props()
  const paths: Record<string, string> = {
    search: 'M16 10a6 6 0 1 1-12 0 6 6 0 0 1 12 0zm-2 5 6 6',
    folder: 'M3 7V5a1 1 0 0 1 1-1h5l2 3h9a1 1 0 0 1 1 1v2M3 8v11h16l3-10H6l-3 10',
    link: 'M10 13a4 4 0 0 0 6 0l4-4a4 4 0 0 0-6-6l-2 2M14 11a4 4 0 0 0-6 0l-4 4a4 4 0 0 0 6 6l2-2',
    gear: 'M10.25 4.05l-.5 1.9-1.5.9-1.9-.5-1.7 2.9 1.4 1.4v2.4l-1.4 1.5 1.7 2.9 1.9-.5 1.5.9.5 1.9h3.5l.5-1.9 1.5-.9 1.9.5 1.7-2.9-1.4-1.5v-2.4l1.4-1.4-1.7-2.9-1.9.5-1.5-.9-.5-1.9zM15 12a3 3 0 1 1-6 0 3 3 0 0 1 6 0z',
    settings: 'M4 6h7m4 0h5M4 12h2m4 0h10M4 18h10m4 0h2M11 3v6M6 9v6M14 15v6',
    export: 'M12 15V3m-4 4 4-4 4 4M4 13v7h16v-7',
    transcript: 'M3 4h18v16H3zM14 4v16m3-11h1m-1 4h1',
    more: 'M5 12h.1m6.9 0h.1m6.9 0h.1',
    info: 'M21 12a9 9 0 1 1-18 0 9 9 0 1 1 18 0ZM12 11v5m0-8v.1',
    play: 'm7 4 13 8-13 8z',
    pause: 'M7 5v14m9-14v14',
    volume: 'M3 9h4l5-4v14l-5-4H3zM16 8a6 6 0 0 1 0 8m3-11a10 10 0 0 1 0 14',
    mute: 'M3 9h4l5-4v14l-5-4H3zM16 9l5 6m0-6-5 6',
    full: 'M3 9V3h6m6 0h6v6m0 6v6h-6m-6 0H3v-6',
    collapse: 'M9 3v6H3m12-6v6h6M9 21v-6H3m12 6v-6h6',
    close: 'm6 6 12 12M6 18 18 6',
    down: 'm7 10 5 5 5-5',
    right: 'm9 6 6 6-6 6',
    film: 'M3 3h18v18H3zM7 3v18M17 3v18M3 8h4m-4 8h4M17 8h4m-4 8h4',
    clock: 'M12 3a9 9 0 1 0 .01 0M12 7v5l3 2',
    cc: 'M2 4h20v16H2zM10 9a3 3 0 1 0 0 6m9-6a3 3 0 1 0 0 6',
    audio: 'M3 10v4m4-8v12m5-15v18m5-15v12m4-8v4',
    check: 'm4 12 5 5L20 6',
    download: 'M12 3v12m-5-5 5 5 5-5M4 17v4h16v-4',
    rewind: 'm12 6-9 6 9 6zM21 6l-9 6 9 6z',
    'fast-forward': 'm3 6 9 6-9 6zM12 6l9 6-9 6z',
    'skip-back': 'M7 4L3 8l4 4M4 8h9a7 7 0 0 1 7 7',
    'skip-forward': 'm17 4 4 4-4 4m3-4h-9a7 7 0 0 0-7 7',
    repeat: 'M4 9a4 4 0 0 1 4-4h11l-3-3m3 3-3 3M20 15a4 4 0 0 1-4 4H5l3 3m-3-3 3-3',
    'repeat-one': 'M4 9a4 4 0 0 1 4-4h11l-3-3m3 3-3 3M20 15a4 4 0 0 1-4 4H5l3 3m-3-3 3-3m3-5 2-1v5',
    airplay: 'M7 16H3V4h18v12h-4m-5-3 6 7H6z',
    pip: 'M3 4h18v16H3zM11 11h8v6h-8z',
    previous: 'M5 5v14m13-14L7 12l11 7z',
    next: 'M19 5v14M6 5l11 7-11 7z',
    sidebar: 'M5 4h14a2 2 0 0 1 2 2v12a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V6a2 2 0 0 1 2-2zm4 0v16M5.5 8h1m-1 3h1',
    list: 'M4 6h16M4 12h13M4 18h9',
    alert: 'm12 3 10 17H2zM12 9v5m0 3v.1',
    monitor: 'M3 4h18v13H3zM8 21h8m-4-4v4',
    refresh: 'M3 10a9 9 0 1 1 1 8M3 4v6h6',
    trash: 'M3 6h18M9 6V3h6v3M5 6l1 15h12l1-15M10 10v7m4-7v7',
    keyboard: 'M2 5h20v14H2zM6 9h1m3 0h1m3 0h1m3 0h1M6 14h2m2 0h8',
  }
</script>

<svg
  width={size}
  height={size}
  viewBox="0 0 24 24"
  fill={['play', 'previous', 'next', 'rewind', 'fast-forward'].includes(name) ? 'currentColor' : 'none'}
  stroke="currentColor"
  stroke-width={name === 'more' ? 3.5 : name === 'pause' ? 3 : 1.55}
  stroke-linecap="round"
  stroke-linejoin="round"
  aria-hidden="true"
>
  <path d={paths[name]} />
  {#if name === 'skip-back' || name === 'skip-forward'}
    <text
      x="12"
      y="20"
      text-anchor="middle"
      font-size="10"
      font-weight="600"
      fill="currentColor"
      stroke="none">10</text
    >
  {/if}
</svg>

<script lang="ts">
  import { untrack } from 'svelte'
  import { player as p } from './stores.svelte'
  import type { EmbeddedTrack } from './backend'
  import { subtitleTracks as tracks } from './subtitle-tracks.svelte'

  const labels = {
    en: {
      track: 'Subtitle Track',
      generated: 'Generated Subtitles',
      embedded: 'Embedded',
      file: 'Subtitle Files',
      nearby: 'Next to This File',
      loading: 'Reading subtitles…',
    },
    zh: {
      track: '字幕轨道',
      generated: '自动生成的字幕',
      embedded: '内嵌字幕',
      file: '字幕文件',
      nearby: '同目录字幕',
      loading: '正在读取字幕…',
    },
  }
  const copy = $derived(labels[p.locale])
  const names = $derived(new Intl.DisplayNames([p.locale], { type: 'language' }))
  function trackLabel(track: EmbeddedTrack) {
    let language = track.language
    if (language) {
      try {
        language = names.of(language) || language
      } catch {
        /* Keep container metadata verbatim. */
      }
    }
    return [
      track.title || language || copy.track + ' ' + (track.order + 1),
      track.title && language,
    ]
      .filter(Boolean)
      .join(' · ')
  }
  $effect(() => {
    const id = p.media?.sessionId ?? ''
    untrack(() => {
      void tracks.discover(id)
    })
  })
</script>

<div class="subtitle-track-tools track-picker">
  <label class="track-field">
    <span>{copy.track}</span>
    <select
      aria-label={copy.track}
      disabled={!p.media}
      value={tracks.selected === 'generated' && p.settings.subtitleMode === 'off'
        ? 'off'
        : tracks.selected}
      onchange={(e) => {
        const key = e.currentTarget.value
        // Keep the actual track selected until an asynchronous read succeeds.
        e.currentTarget.value =
          tracks.selected === 'generated' && p.settings.subtitleMode === 'off'
            ? 'off'
            : tracks.selected
        void p.act(() => tracks.select(key))
      }}
    >
      <option value="off">{p.t('off')}</option>
      <option value="generated">{copy.generated}</option>
      {#if tracks.embedded.length}<optgroup label={copy.embedded}>
          {#each tracks.embedded as track (track.order)}
            <option value={'embedded:' + track.order}>{trackLabel(track)}</option>
          {/each}
        </optgroup>{/if}
      {#if tracks.sidecars.length}<optgroup label={copy.nearby}>
          {#each tracks.sidecars as name (name)}<option value={'sidecar:' + name}>{name}</option
            >{/each}
        </optgroup>{/if}
      {#if tracks.files.length}<optgroup label={copy.file}>
          {#each tracks.files as file (file.id)}
            <option value={file.id}>{file.name}</option>
          {/each}
        </optgroup>{/if}
    </select>
  </label>
</div>
{#if tracks.loading || tracks.discovering}<small class="read-status" role="status"
    >{copy.loading}</small
  >{/if}

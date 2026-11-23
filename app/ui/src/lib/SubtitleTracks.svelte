<script lang="ts">
  import { untrack, type Snippet } from 'svelte'
  import { player as p } from './stores.svelte'
  import type { EmbeddedTrack } from './backend'
  import { subtitleTracks as tracks } from './subtitle-tracks.svelte'

  let { children }: { children: Snippet } = $props()
  const labels = {
    en: {
      track: 'Subtitle track',
      generated: 'Generated subtitles',
      embedded: 'Embedded',
      file: 'Subtitle file',
      nearby: 'Beside this movie',
      load: 'Open subtitle file…',
      delay: 'Subtitle delay (s)',
      loading: 'Reading subtitles…',
      earlier: 'Show subtitles earlier',
      later: 'Show subtitles later',
      again: 'Retry embedded tracks',
    },
    zh: {
      track: '字幕轨道',
      generated: '自动生成的字幕',
      embedded: '内嵌字幕',
      file: '字幕文件',
      nearby: '同目录字幕',
      load: '打开字幕文件…',
      delay: '字幕延迟（秒）',
      loading: '正在读取字幕…',
      earlier: '提前显示字幕',
      later: '延后显示字幕',
      again: '重新读取内嵌字幕',
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
  function delay(seconds: number) {
    if (Number.isFinite(seconds) && Number.isSafeInteger(Math.round(seconds * 1000)))
      tracks.setDelay(Math.round(seconds * 1000))
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
<details class="track-settings">
  <summary>{p.t('subtitleControls')}</summary>
  <div class="subtitle-track-tools expanded-tools">
    <button
      class="import-subtitle"
      disabled={!p.media}
      onclick={() => window.dispatchEvent(new Event('hlp:subtitle-file'))}>{copy.load}</button
    >
    <div class="track-field">
      <label for="track-delay-seconds">{copy.delay}</label>
      <span class="delay-stepper">
        <button
          type="button"
          disabled={!p.media}
          title={copy.earlier}
          aria-label={copy.earlier}
          onclick={() => delay((tracks.delayMs - 100) / 1000)}>−</button
        >
        <input
          id="track-delay-seconds"
          type="number"
          step="0.1"
          disabled={!p.media}
          aria-label={copy.delay}
          value={tracks.delayMs / 1000}
          onchange={(e) => delay(e.currentTarget.valueAsNumber)}
        />
        <button
          type="button"
          disabled={!p.media}
          title={copy.later}
          aria-label={copy.later}
          onclick={() => delay((tracks.delayMs + 100) / 1000)}>+</button
        >
      </span>
    </div>
    {#if tracks.discoveryError}<button onclick={() => tracks.discover(p.media?.sessionId ?? '')}
        >{copy.again}</button
      >{/if}
    {@render children()}
  </div>
</details>
{#if tracks.loading || tracks.discovering}<small class="read-status" role="status"
    >{copy.loading}</small
  >{/if}

<style>
  .track-picker {
    flex: none;
    padding: 8px 12px 7px;
    width: 100%;
  }
  .track-settings {
    flex: none;
    min-height: 0;
    max-height: 47%;
    overflow: hidden auto;
    overscroll-behavior: none contain;
    border-bottom: 1px solid var(--hairline);
  }
  .track-settings summary {
    margin-inline: 4px;
    padding: 5px 8px 8px;
    border-radius: 4px;
    color: var(--sub);
    font-size: 12px;
    cursor: pointer;
  }
  .expanded-tools {
    padding: 6px 12px 12px;
  }
  .read-status {
    padding-inline: 12px;
  }
  .subtitle-track-tools {
    display: flex;
    flex-wrap: wrap;
    align-items: end;
    gap: 0.65rem;
    min-width: 0;
  }
  .track-field {
    display: grid;
    gap: 0.3rem;
    min-width: 0;
  }
  .track-field > span:first-child,
  .track-field > label {
    font-size: 0.75rem;
    color: var(--muted, #aaa);
  }
  .track-field select {
    max-width: 100%;
    min-width: 0;
  }
  .delay-stepper {
    display: inline-flex;
    align-items: center;
    gap: 2px;
  }
  .delay-stepper input {
    width: 4.3em;
    text-align: center;
    font-variant-numeric: tabular-nums;
    appearance: textfield;
  }
  .delay-stepper input::-webkit-inner-spin-button {
    appearance: none;
  }
  .delay-stepper button {
    min-width: 1.8rem;
    padding-inline: 0.3rem;
  }
</style>

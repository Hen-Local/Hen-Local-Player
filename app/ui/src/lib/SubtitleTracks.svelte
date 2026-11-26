<script lang="ts">
  import SelectField from './SelectField.svelte'
  import { untrack } from 'svelte'
  import { player as p } from './stores.svelte'
  import type { EmbeddedTrack } from './backend'
  import Icon from './Icon.svelte'
  import { subtitleTracks as tracks } from './subtitle-tracks.svelte'

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
      track.title || language || p.t('subtitleTrack') + ' ' + (track.order + 1),
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
    <span>{p.t('subtitleTrack')}</span>
    <SelectField>
      <select
        aria-label={p.t('subtitleTrack')}
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
        <option value="generated">{p.t('generatedSubtitles')}</option>
        {#if tracks.embedded.length}<optgroup label={p.t('embeddedTracks')}>
            {#each tracks.embedded as track (track.order)}
              <option value={'embedded:' + track.order}>{trackLabel(track)}</option>
            {/each}
          </optgroup>{/if}
        {#if tracks.sidecars.length}<optgroup label={p.t('nearbySubtitles')}>
            {#each tracks.sidecars as name (name)}<option value={'sidecar:' + name}>{name}</option
              >{/each}
          </optgroup>{/if}
        {#if tracks.files.length}<optgroup label={p.t('subtitleFiles')}>
            {#each tracks.files as file (file.id)}
              <option value={file.id}>{file.name}</option>
            {/each}
          </optgroup>{/if}
      </select>
    </SelectField>
  </label>
  <div class="track-sources">
    <button class="import-subtitle" aria-label={p.t('openSubtitleFile')} title={p.t('subtitleFileTypes')}
      onclick={() => window.dispatchEvent(new Event('hlp:subtitle-file'))}>
      <Icon name="folder" size={14} />{p.t('importSubtitleAction')}
    </button>
    <button class="online-subtitles" aria-label={p.t('findOnlineSubtitles')} aria-haspopup="dialog" aria-keyshortcuts="Meta+Shift+D Control+Shift+D"
      title={p.t('findOnlineSubtitles') + (/Mac/.test(navigator.platform) ? ' (⇧⌘D)' : ' (Ctrl+Shift+D)')}
      onclick={() => p.findOnlineSubtitles()}>
      <Icon name="search" size={14} />{p.t('searchSubtitleAction')}
    </button>
  </div>
  {#if tracks.discoveryError}
    <button onclick={() => tracks.discover(p.media?.sessionId ?? '')}>{p.t('retryTracks')}</button>
  {/if}
</div>
{#if tracks.loading || tracks.discovering}<small class="read-status" role="status"
    >{p.t('readingSubtitles')}</small
  >{/if}

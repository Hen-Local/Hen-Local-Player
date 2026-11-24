<script lang="ts">
  import { player as p, mockMode } from './stores.svelte'
  import Icon from './Icon.svelte'
  import SubtitleTracks from './SubtitleTracks.svelte'
  import SubtitleSettings from './SubtitleSettings.svelte'
  import SubtitleProgress from './SubtitleProgress.svelte'
  import TranscriptPanel from './TranscriptPanel.svelte'
  import { subtitleTracks as tracks } from './subtitle-tracks.svelte'
  import { componentNames } from './component-names'
  import { size } from './format'

  let off = $derived(
    tracks.selected === 'off' ||
      (tracks.selected === 'generated' && p.settings.subtitleMode === 'off'),
  )
  let generated = $derived(!off && tracks.selected === 'generated')
  let missing = $derived(p.missingSubtitleComponents)
  let translationOnly = $derived(
    missing.length > 0 &&
      missing.every((id) => ['mt-hy-mt2-1.8b', 'llama-runtime-b11429'].includes(id)),
  )
  let stage = $derived(p.status?.stage ?? 'idle')
  let browserFile = $derived(mockMode && p.currentPath.startsWith('browser:'))
  let unavailable = $derived(
    browserFile
      ? p.t('transcribeDesktop')
      : !p.media?.audioTracks.length
        ? p.t('noAudioTrack')
        : '',
  )
  let installing = $derived(
    !!p.installingSubtitleComponent ||
      p.components.some((c) => c.state === 'downloading' || c.state === 'verifying'),
  )
  let working = $derived(
    p.transcriptionRequested &&
      !missing.length &&
      !p.subtitleError &&
      !['complete', 'error', 'needs-setup', 'paused'].includes(stage),
  )
  let hasLines = $derived(!!(generated ? p.cues.length : tracks.authored?.cues.length))
  let empty = $derived(!hasLines && !tracks.loading && !tracks.discovering)
  let canCreate = $derived(
    empty &&
      !tracks.authored &&
      !p.transcriptionRequested &&
      !p.canRegenerate &&
      (!off ||
        (!p.cues.length &&
          !tracks.files.length &&
          !tracks.embedded.length &&
          !tracks.sidecars.length)) &&
      stage !== 'complete',
  )
  let cached = $derived(
    generated && hasLines && !p.transcriptionRequested && p.status?.stage === 'idle' &&
    (!p.status.vadComplete || p.status.uttDone + p.status.uttNoSpeech < p.status.uttTotal || p.status.trPendingSpans.length > 0),
  )
  let retry = $derived(
    generated &&
      (stage === 'error' || !!p.subtitleError || stage === 'needs-setup' || stage === 'paused') &&
      !missing.length &&
      !p.startingTranscription,
  )
  let showActions = $derived(off || canCreate || (generated && (missing.length > 0 || cached || retry)))
  let showProgress = $derived(generated && !missing.length &&
    (p.startingTranscription || (stage !== 'complete' && (working || cached || stage === 'error' || stage === 'paused'))))
</script>

<div class="subtitle-workspace" tabindex="-1">
  <div class="subtitle-options">
    <SubtitleTracks />
    <SubtitleSettings />
    {#if !off && (hasLines || (generated && working))}
      {#key (p.media?.sessionId ?? '') + '/' + tracks.selected}
        <TranscriptPanel />
      {/key}
    {/if}
    {#if generated && stage === 'complete' && !hasLines}
      <p class="empty-note" role="status">{p.t('noSpeech')}</p>
    {/if}
  </div>
  {#if showActions || showProgress}
    <div class="generation-footer">
      {#if showActions}
        <div class="generation-actions">
          {#if off}<p>{p.t('subtitlesOff')}</p>{/if}
          {#if canCreate}
            <button
              class="create-subtitles"
              disabled={!!unavailable || p.startingTranscription}
              onclick={() => p.startTranscription()}
            >
              <Icon name="cc" size={16} />{p.t('createSubtitles')}
            </button>
            {#if unavailable}<p>{unavailable}</p>{/if}
          {:else if generated}
            {#if missing.length}
              <strong>{p.t('neededComponents')}</strong>
              {#each missing as id}
                {@const component = p.components.find((c) => c.id === id)}
                {@const state = component?.state}
                <div class="missing-component">
                  <span
                    >{componentNames[id] || id}{#if component}<small>{size(component.bytesTotal)}</small
                      >{/if}</span
                  >
                  <button
                    disabled={installing}
                    aria-label={p.t('downloadOne') + ' · ' + (componentNames[id] || id)}
                    onclick={() => p.installSubtitleComponent(id)}
                    >{typeof state === 'object' ? p.t('retry') : p.t('download')}</button
                  >
                </div>
                {#if state === 'downloading' || state === 'verifying'}
                  <small role="status">{p.t(state)} · {size(component?.bytesDone ?? 0)}</small>
                {:else if typeof state === 'object'}<small class="form-error" role="alert"
                    >{state.failed}</small
                  >{/if}
              {/each}
              {#if translationOnly && p.settings.targetLanguage !== 'none'}
                <button
                  disabled={installing || p.startingTranscription}
                  onclick={() => p.startTranscription({ originalOnly: true })}>{p.t('originalOnly')}</button
                >
              {/if}
            {:else if retry || cached}
              <button class="create-subtitles" onclick={() => p.startTranscription()}
                >{p.t(
                  cached || stage === 'paused' ? 'continueTranscription' : 'retryTranscription',
                )}</button
              >
            {/if}
            {#if p.subtitleError}<small class="form-error" role="alert">{p.subtitleError}</small>{/if}
          {/if}
        </div>
      {/if}
      {#if showProgress}
        <SubtitleProgress {cached} />
      {/if}
    </div>
  {/if}
</div>

<style>
  .subtitle-workspace {
    display: flex;
    flex-direction: column;
    flex: 1;
    min-height: 0;
    overflow: hidden;
  }
  .subtitle-options {
    display: flex;
    flex: 1;
    flex-direction: column;
    min-height: 0;
    min-width: 0;
    overflow: hidden auto;
    overscroll-behavior: contain;
  }
  .generation-footer {
    flex: none;
    max-height: 49%;
    overflow: hidden auto;
    border-top: 1px solid var(--hairline);
    overscroll-behavior: contain;
  }
  .subtitle-options .empty-note {
    padding-inline: var(--inspector-inset);
  }
  .generation-actions {
    display: grid;
    align-content: start;
    gap: 9px;
    flex-shrink: 0;
    padding: 12px var(--inspector-inset);
    font-size: 12px;
  }
  .generation-actions p {
    margin: 0;
    line-height: 1.45;
    color: var(--muted);
  }
  .generation-actions .create-subtitles {
    width: 100%;
    min-height: 34px;
    border-color: var(--accent-text);
    color: var(--ink);
    background: var(--accent-soft);
  }
  .missing-component {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
  }
  .missing-component span {
    min-width: 0;
    overflow-wrap: anywhere;
  }
  .missing-component small {
    display: block;
    margin-top: 2px;
  }
</style>

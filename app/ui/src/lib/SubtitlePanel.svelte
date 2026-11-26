<script module lang="ts">
  const introducedFiles = new Set<string>()

  function introduce(node: HTMLElement, file: string) {
    if (!file || introducedFiles.has(file)) return
    introducedFiles.add(file)
    node.dataset.firstOffer = ''
  }
</script>

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

  let viewport = $state<HTMLDivElement>()
  let browsing = $state(false)

  function scrollByKey(event: KeyboardEvent) {
    if (!viewport || event.metaKey || event.ctrlKey || event.altKey ||
      (event.target instanceof Element && event.target.closest('input, select, textarea'))) return
    const page = viewport.clientHeight * .9
    const distance: Record<string, number> = {
      ArrowUp: -40, ArrowDown: 40, PageUp: -page, PageDown: page,
      Home: -viewport.scrollHeight, End: viewport.scrollHeight,
    }
    if (!(event.key in distance)) return
    event.preventDefault()
    browsing = true
    viewport.scrollTop += distance[event.key]
  }

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
  let fileKey = $derived(p.media?.sourcePath || p.currentPath || p.media?.sessionId || '')
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
    p.transcriptionRequested && !missing.length &&
      (stage === 'processing' || (stage === 'idle' && !p.subtitleError)),
  )
  let hasLines = $derived(!!(generated ? p.cues.length : tracks.authored?.cues.length))
  let canCreate = $derived(
    !p.cues.length && !tracks.loading && !tracks.discovering &&
      !p.transcriptionRequested && !p.startingTranscription && !p.canRegenerate &&
      !p.subtitleError && !missing.length && (stage === 'idle' || stage === 'needs-setup'),
  )
  let cached = $derived(
    generated && hasLines && !p.transcriptionRequested && p.status?.stage === 'idle' &&
    (!p.status.vadComplete || p.status.uttDone + p.status.uttNoSpeech < p.status.uttTotal || p.status.trPendingSpans.length > 0 || p.status.translationFailed > 0),
  )
  let retry = $derived(
    generated &&
      (['error', 'needs-setup', 'paused', 'partial'].includes(stage) || (stage !== 'processing' && !!p.subtitleError)) &&
      !missing.length &&
      !p.startingTranscription,
  )
  let showActions = $derived(off || canCreate || (generated && (missing.length > 0 || cached || retry)))
  let showProgress = $derived(generated && !missing.length &&
    (p.startingTranscription || (stage !== 'complete' && (working || cached || stage === 'error' || stage === 'paused'))))
</script>

<div class="subtitle-workspace" tabindex="-1">
  <!-- The one scrollable region is focusable so keyboard users can page through the inspector. -->
  <!-- svelte-ignore a11y_no_noninteractive_tabindex, a11y_no_noninteractive_element_interactions -->
  <div class="subtitle-options" role="region" aria-label={p.t('subtitles')} tabindex="0"
    bind:this={viewport} onkeydown={scrollByKey}
    onwheel={event => { if (event.deltaY) browsing = true }}
    ontouchmove={() => browsing = true}
    onpointerdown={() => browsing = true}
    onfocusin={event => { if (event.target !== viewport) browsing = true }}>
    <SubtitleTracks />
    <SubtitleSettings />
    {#if showActions || showProgress}
      <div class="generation-footer">
        {#if showActions}
          <div class="generation-actions">
            {#if off}<p>{p.t('subtitlesOff')}</p>{/if}
            {#if canCreate}
              <button
                class="create-subtitles generate-subtitles"
                disabled={!!unavailable || p.startingTranscription}
                onclick={() => p.startTranscription()}
              >
                {#if !unavailable}
                  {#key fileKey}
                    <span class="reflection" aria-hidden="true" use:introduce={fileKey}></span>
                  {/key}
                {/if}
                <Icon name="cc" size={16} /><span>{p.t('createSubtitles')}</span>
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
                {#if stage === 'partial'}
                  <p role="status">{p.t('untranslatedLines')} · {p.status?.translationFailed}</p>
                {/if}
                <button class="create-subtitles" onclick={() => p.startTranscription()}
                  >{p.t(
                    stage === 'partial' ? 'retryTranslation' :
                      cached || stage === 'paused' ? 'continueTranscription' : 'retryTranscription',
                  )}</button
                >
              {/if}
              {#if p.subtitleError && stage !== 'processing'}<small class="form-error" role="alert">{p.subtitleError}</small>{/if}
            {/if}
          </div>
        {/if}
        {#if showProgress}
          <SubtitleProgress {cached} />
        {/if}
      </div>
    {/if}
    {#if !off && (hasLines || (generated && working))}
      {#key (p.media?.sessionId ?? '') + '/' + tracks.selected}
        <TranscriptPanel {viewport} bind:browsing />
      {/key}
    {/if}
    {#if generated && stage === 'complete' && !hasLines}
      <p class="empty-note" role="status">{p.t('noSpeech')}</p>
    {/if}
  </div>
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
    margin-top: auto;
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
    border: 0;
    color: var(--ink);
    background: var(--surface-selection);
  }
  .generation-actions .generate-subtitles {
    position: relative;
    isolation: isolate;
    overflow: hidden;
    min-height: 37px;
  }
  .generation-actions .generate-subtitles:enabled {
    color: var(--accent-on-solid);
    background: linear-gradient(112deg, color-mix(in srgb, var(--accent) 90%, white), var(--accent));
    box-shadow: inset 0 1px rgb(255 255 255 / 28%);
  }
  .generation-actions .generate-subtitles:enabled:hover {
    color: var(--accent-on-solid);
    background: color-mix(in srgb, var(--accent) 92%, white);
  }
  @media (prefers-reduced-motion: no-preference) and (forced-colors: none) {
    .reflection:global([data-first-offer]) {
      position: absolute;
      z-index: -1;
      inset: 0;
      pointer-events: none;
      background: linear-gradient(108deg, transparent 32%, rgb(151 246 255 / 16%) 43%, rgb(255 255 255 / 22%) 49%, rgb(254 187 255 / 16%) 55%, transparent 67%);
      transform: translateX(-100%);
      animation: glint 1050ms 140ms ease-in-out;
    }
    @keyframes glint {
      to { transform: translateX(100%); }
    }
  }
  @media (forced-colors: active) {
    .generation-actions .generate-subtitles:enabled { border: 1px solid ButtonText; }
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

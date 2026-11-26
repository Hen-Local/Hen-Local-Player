<script lang="ts">
  import { onMount } from 'svelte'
  import Disclosure from './Disclosure.svelte'
  import SelectField from './SelectField.svelte'
  import { appUpdates as updates, player as p } from './stores.svelte'
  import { size } from './format'

  let release = $derived(updates.releases?.find(item => item.version === updates.selected))
  let showPrepared = $derived(updates.prepared && (!release || release.version === updates.prepared))
  let busy = $derived(updates.task !== 'idle')
  let progress = $derived(updates.progress)
  let current = $derived(updates.releases?.length && !updates.releases[0].newer)
  let checkLabel = $derived(p.t(updates.task === 'checking' ? 'checkingUpdates'
    : updates.error ? 'retry' : current ? 'noUpdates' : 'checkUpdates'))
  let notesOpen = $state(false)
  const errors = {
    UPDATE_FEED_UNAVAILABLE: 'updateFeedUnavailable', UPDATE_RATE_LIMITED: 'updateRateLimit',
  } as const
  let message = $derived.by(() => {
    if (updates.error) return p.t(errors[updates.error as keyof typeof errors] ?? 'updateFailed')
    if (updates.task === 'downloading') return p.t('downloading') + (progress ? ' · ' + size(progress.downloaded) : '')
    if (showPrepared) return p.t('updateReady').replace('{version}', updates.prepared ?? '')
    if (updates.releases?.length === 0) return p.t('noReleases')
    if (release?.newer && !release.installable) return p.t('noUpdatePackage')
    return ''
  })

  onMount(() => { void updates.loadLocal() })

  function openHistory(open: boolean) {
    if (open && !updates.releases && !updates.error) void updates.check()
  }
  function install() {
    p.remember(true)
    void updates.install()
  }
</script>

<section class="software-update" aria-labelledby="software-update-heading">
  <div class="version-row">
    <div class="version-summary">
      <strong id="software-update-heading">{p.t('softwareUpdate')}</strong>
      <p>{p.t('installedVersion')} <bdi dir="ltr">{updates.version || '…'}</bdi></p>
    </div>
    <button class="check-updates" disabled={busy} onclick={() => updates.check()}
      title={p.t('checkUpdates')} aria-live="polite">{checkLabel}</button>
  </div>

  <div class="history">
    <Disclosure label={p.t('releaseVersion')} ontoggle={openHistory}>
      <div class="history-content">
        <SelectField>
          <select aria-label={p.t('releaseVersion')} bind:value={updates.selected} disabled={busy || !updates.releases?.length}>
            {#each updates.releases ?? [] as item (item.version)}
              <option value={item.version}>{item.version}{item.version === updates.version ? ' · ' + p.t('installed') : ''}</option>
            {:else}
              <option value="">{updates.task === 'checking' ? p.t('checkingUpdates') : '—'}</option>
            {/each}
          </select>
        </SelectField>
        <Disclosure label={p.t('releaseNotes')} bind:expanded={notesOpen} disabled={!release?.notes && !notesOpen}>
          <div class="notes">{release?.notes || '—'}</div>
        </Disclosure>
      </div>
    </Disclosure>
    <div class="update-feedback" aria-live="polite">
      {#if showPrepared}
        <button disabled={busy} onclick={install} title={message}>
          {p.t(updates.task === 'installing' ? 'restarting' : 'restartInstall')}
        </button>
      {:else if updates.task === 'downloading'}
        <progress aria-label={message} title={message}
          value={progress?.total ? progress.downloaded : undefined} max={progress?.total || 1}></progress>
      {:else if release?.installable && !updates.error}
        <button disabled={busy} onclick={() => updates.download(updates.selected)}>{p.t('downloadUpdate')}</button>
      {:else if message}
        <span title={message} class:error={!!updates.error}>{message}</span>
      {/if}
    </div>
  </div>

  <label class="toggle update-preference" title={p.t('automaticUpdatesHelp')}>
    <input type="checkbox" checked={p.settings.autoDownloadUpdates} aria-describedby="auto-update-help"
      onchange={e => { const enabled = e.currentTarget.checked; void p.act(() => p.update({ autoDownloadUpdates: enabled })) }} />
    {p.t('automaticUpdates')}
  </label>
  <span id="auto-update-help" class="accessible-hint">{p.t('automaticUpdatesHelp')}</span>
</section>

<style>
  .software-update { padding-block: 0 16px; }
  .version-row { display: grid; grid-template-columns: minmax(0,1fr) minmax(0,44%); gap: 10px; align-items: center; }
  .version-summary strong { font-size: 15px; }
  .version-summary p { display: flex; flex-wrap: wrap; align-items: baseline; gap: 6px; color: var(--muted); margin: 4px 0 0; font-size: 12px; }
  .version-summary bdi { color: var(--ink); font-weight: 650; font-variant-numeric: tabular-nums; }
  .check-updates { width: min(100%, 11.5em); height: 34px; justify-self: end; }
  .check-updates, .update-feedback :is(button, span) { display: block; white-space: nowrap; text-overflow: ellipsis; overflow: hidden; }
  .history { display: grid; grid-template-columns: minmax(0, 1fr) minmax(0, 46%); gap: 0 8px; margin-block: 8px; }
  .history > :global(.disclosure) { display: contents; }
  .history > :global(.disclosure > .disclosure-trigger) { grid-column: 1; grid-row: 1; }
  .history > :global(.disclosure > .disclosure-content) { grid-column: 1 / -1; grid-row: 2; }
  .history-content { display: grid; gap: 4px; padding: 3px 4px 6px; }
  .notes { height: min(10em, 24dvh); overflow: auto; white-space: pre-wrap; overflow-wrap: anywhere; margin: 4px; line-height: 1.45; font-size: 12px; }
  .update-feedback { grid-row: 1; grid-column: 2; min-width: 0; display: flex; align-items: center; justify-content: flex-end; }
  .update-feedback :is(button, span, progress) { max-width: 100%; }
  .update-feedback span { color: var(--muted); font-size: 12px; line-height: 1.3; text-align: end; white-space: normal; display: -webkit-box; -webkit-box-orient: vertical; -webkit-line-clamp: 2; line-clamp: 2; }
  .update-feedback progress { width: 115px; height: 6px; }
  .history .error { color: #ffaeaa; }
  .update-preference { display: flex; font-size: 13px; font-weight: 450; line-height: 1.4; }
  .accessible-hint { position: absolute; clip-path: inset(50%); width: 1px; height: 1px; overflow: hidden; }
  @media (max-width: 575px) { .software-update { padding-bottom: 12px; } }
</style>

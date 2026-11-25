<script lang="ts">
  import { onMount } from 'svelte'
  import { appUpdates as updates, player as p } from './stores.svelte'
  import { size } from './format'

  let release = $derived(updates.releases?.find(item => item.version === updates.selected))
  let showPrepared = $derived(updates.prepared && (!release || release.version === updates.prepared))
  let busy = $derived(updates.task !== 'idle')
  let progress = $derived(updates.progress)
  const errors = {
    UPDATE_FEED_UNAVAILABLE: 'updateFeedUnavailable', UPDATE_RATE_LIMITED: 'updateRateLimit',
  } as const
  let errorText = $derived(p.t(errors[updates.error as keyof typeof errors] ?? 'updateFailed'))

  onMount(() => { void updates.loadLocal() })

  function install() {
    p.remember(true)
    void updates.install()
  }
</script>

<section class="software-update" aria-labelledby="software-update-heading">
  <div class="setting-block">
    <div>
      <strong id="software-update-heading">{p.t('softwareUpdate')}</strong>
      <small>{p.t('installedVersion')} {updates.version || '…'}</small>
    </div>
    <button disabled={busy} onclick={() => updates.check()}>
      {p.t(updates.task === 'checking' ? 'checkingUpdates' : 'checkUpdates')}
    </button>
  </div>
  <div class="update-options">
    <label class="toggle update-preference">
      <input type="checkbox" checked={p.settings.autoDownloadUpdates} aria-describedby="auto-update-help"
        onchange={e => { const enabled = e.currentTarget.checked; void p.act(() => p.update({ autoDownloadUpdates: enabled })) }} />
      {p.t('automaticUpdates')}
    </label>
  </div>
  <p id="auto-update-help" class="help">{p.t('automaticUpdatesHelp')}</p>

  {#if updates.releases?.length}
    <label class="version-picker">
      {p.t('releaseVersion')}
      <select bind:value={updates.selected} disabled={busy}>
        {#each updates.releases as item (item.version)}
          <option value={item.version}>{item.version}{item.version === updates.version ? ' · ' + p.t('installed') : ''}</option>
        {/each}
      </select>
    </label>
    {#if release?.notes}
      <details class="release-notes">
        <summary>{p.t('releaseNotes')}</summary>
        <div>{release.notes}</div>
      </details>
    {/if}
  {/if}

  {#if showPrepared}
    <div class="update-action">
      <small>{p.t('updateReady').replace('{version}', updates.prepared ?? '')}</small>
      <button disabled={busy} onclick={install}>
        {p.t(updates.task === 'installing' ? 'restarting' : 'restartInstall')}
      </button>
    </div>
  {:else if updates.task === 'downloading'}
    <div class="update-progress" role="status">
      <small>{p.t('downloading')}{progress ? ' · ' + size(progress.downloaded) : ''}</small>
      <progress aria-label={p.t('downloadUpdate')} value={progress?.total ? progress.downloaded : undefined}
        max={progress?.total || 1}></progress>
    </div>
  {:else if release?.installable}
    <button disabled={busy} onclick={() => updates.download(updates.selected)}>{p.t('downloadUpdate')}</button>
  {:else if updates.releases && !updates.error && updates.task !== 'checking'}
    {#if !updates.releases.length}
      <p class="help" role="status">{p.t('noReleases')}</p>
    {:else if release?.version !== updates.version}
      <p class="help" role="status">{p.t('noUpdatePackage')}</p>
    {:else if updates.releases[0].version === updates.version}
      <p class="help" role="status">{p.t('noUpdates')}</p>
    {/if}
  {/if}
  {#if updates.error}
    <p class="form-error" role="alert">{errorText}</p>
  {/if}
</section>

<style>
  .software-update { padding-bottom: 16px; margin-bottom: 16px; border-bottom: 1px solid var(--line); }
  .setting-block { padding: 0 0 12px; }
  .update-preference { display: flex; font-size: 12px; font-weight: 630; }
  .update-options { margin-top: 4px; }
  .help { margin: 5px 0 0; }
  .version-picker { display: grid; gap: 6px; margin-top: 14px; font-size: 12px; font-weight: 630; }
  .update-action { display: flex; flex-wrap: wrap; gap: 8px; align-items: center; justify-content: space-between; margin-top: 12px; }
  .update-action small, .update-progress small { color: var(--muted); font-size: 12px; }
  .update-progress { display: grid; gap: 4px; margin-top: 12px; }
  progress { width: 100%; height: 5px; }
  .release-notes { margin: 10px 0; font-size: 12px; }
  summary { cursor: pointer; }
  .release-notes div { max-height: 10em; overflow: auto; white-space: pre-wrap; overflow-wrap: anywhere; margin: 6px 0; line-height: 1.45; }
  .software-update > button { margin-top: 8px; }
  .form-error { margin: 10px 0 0; overflow-wrap: anywhere; }
</style>

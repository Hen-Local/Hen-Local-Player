<script lang="ts">
  import { onMount, tick } from 'svelte'
  import { appUpdates as updates, player as p } from './stores.svelte'
  import { size } from './format'

  let release = $derived(updates.releases?.find(item => item.version === updates.selected))
  let showPrepared = $derived(updates.prepared && (!release || release.version === updates.prepared))
  let busy = $derived(updates.task !== 'idle')
  let progress = $derived(updates.progress)
  let editingAccess = $state(false)
  let linkField = $state<HTMLInputElement>()
  let accessButton = $state<HTMLButtonElement>()
  const errors = {
    UPDATE_FEED_UNAVAILABLE: 'updateFeedUnavailable', UPDATE_RATE_LIMITED: 'updateRateLimit',
    UPDATE_ACCESS_REQUIRED: 'updateAccessHint', UPDATE_ACCESS_DENIED: 'updateAccessDenied',
    UPDATE_LINK_INVALID: 'updateLinkInvalid', UPDATE_LINK_SAVE_FAILED: 'updateLinkSaveFailed',
  } as const
  let errorText = $derived(p.t(errors[updates.error as keyof typeof errors] ?? 'updateFailed'))

  onMount(() => { void updates.loadLocal() })

  async function editAccess() {
    updates.error = ''
    editingAccess = true
    await tick()
    linkField?.focus()
  }
  async function closeEditor() {
    editingAccess = false
    await tick()
    accessButton?.focus()
  }
  async function saveAccess(remove = false) {
    if (!linkField) return
    if (!remove && !linkField.value.trim()) { linkField.focus(); return }
    const result = updates.setAccess(remove ? '' : linkField.value)
    linkField.value = ''
    if (await result) await closeEditor()
  }
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
    {#if updates.hasAccess}
      <button disabled={busy || editingAccess} onclick={() => updates.check()}>
        {p.t(updates.task === 'checking' ? 'checkingUpdates' : 'checkUpdates')}
      </button>
    {:else if !editingAccess}
      <button bind:this={accessButton} disabled={busy} onclick={editAccess}>{p.t('addUpdateLink')}</button>
    {/if}
  </div>

  {#if editingAccess}
    <form class="link-form" autocomplete="off" onsubmit={e => { e.preventDefault(); void saveAccess() }}>
      <label>
        {p.t('personalUpdateLink')}
        <input bind:this={linkField} type="password" inputmode="url" autocomplete="off" spellcheck="false" autocapitalize="none"
          placeholder="https://…" required disabled={updates.task === 'savingAccess'} />
      </label>
      <div class="link-actions">
        {#if updates.hasAccess}
          <button type="button" disabled={updates.task === 'savingAccess'} onclick={() => saveAccess(true)}>{p.t('removeUpdateLink')}</button>
        {/if}
        <button type="button" disabled={updates.task === 'savingAccess'}
          onclick={() => { updates.error = ''; void closeEditor() }}>{p.t('cancel')}</button>
        <button disabled={updates.task === 'savingAccess'}>{p.t('save')}</button>
      </div>
    </form>
  {:else if updates.hasAccess}
    <div class="access-controls">
      <label class="toggle update-preference">
        <input type="checkbox" checked={p.settings.autoDownloadUpdates} aria-describedby="auto-update-help"
          onchange={e => { const enabled = e.currentTarget.checked; void p.act(() => p.update({ autoDownloadUpdates: enabled })) }} />
        {p.t('automaticUpdates')}
      </label>
      <button bind:this={accessButton} disabled={busy && updates.task !== 'checking'} onclick={editAccess}>{p.t('manageUpdateLink')}</button>
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
  {:else}
    <p class="help">{p.t('updateAccessHint')}</p>
  {/if}
  {#if updates.error}
    <p class="form-error" role="alert">{errorText}</p>
  {/if}
</section>

<style>
  .software-update { padding-bottom: 16px; margin-bottom: 16px; border-bottom: 1px solid var(--line); }
  .setting-block { padding: 0 0 12px; }
  .update-preference { display: flex; font-size: 12px; font-weight: 630; }
  .access-controls, .link-actions { display: flex; align-items: center; flex-wrap: wrap; gap: 8px; }
  .access-controls { justify-content: space-between; }
  .access-controls button { font-size: 12px; }
  .link-actions { justify-content: flex-end; }
  .link-actions button:first-child:nth-last-child(3) { margin-right: auto; }
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

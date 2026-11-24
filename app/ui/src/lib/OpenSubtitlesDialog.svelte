<script module lang="ts">
  // Kept in the WebView's memory so repeated searches can reuse the key.
  let sessionKey = ''
</script>

<script lang="ts">
  import { onMount, tick, untrack } from 'svelte'
  import Dialog from './Dialog.svelte'
  import Icon from './Icon.svelte'
  import { backend, player as p } from './stores.svelte'
  import { subtitleTracks } from './subtitle-tracks.svelte'
  import type { OpenSubtitleResult } from './backend'

  const media = untrack(() => p.media)
  const languageCodes = ['en', 'zh-cn', 'zh-tw', 'zh-ca', 'ja', 'ko', 'fr', 'de', 'es', 'it', 'pt-pt', 'pt-br', 'ar', 'da', 'el', 'fa', 'fi', 'he', 'hi', 'hu', 'id', 'ms', 'nl', 'pl', 'ro', 'ru', 'sv', 'th', 'tr', 'uk', 'vi']
  const languageTags: Record<string, string> = { 'zh-cn': 'zh-Hans', 'zh-tw': 'zh-Hant', 'zh-ca': 'yue', 'pt-pt': 'pt-PT', 'pt-br': 'pt-BR' }
  const preferredCode: Record<string, string> = { zh: 'zh-cn', 'zh-Hant': 'zh-tw', yue: 'zh-ca', pt: 'pt-pt' }
  const preferred = untrack(() => preferredCode[p.settings.targetLanguage] ?? p.settings.targetLanguage)
  const names = $derived(new Intl.DisplayNames([p.locale], { type: 'language' }))
  const errorKeys = {
    desktop_only: 'onlineDesktopOnly', missing_key: 'onlineKeyMissing', invalid_key: 'onlineKeyInvalid',
    invalid_query: 'onlineQueryInvalid', invalid_language: 'onlineLanguageInvalid', quota: 'onlineQuota',
    rate_limited: 'onlineRateLimit', unavailable: 'onlineUnavailable', invalid_subtitle: 'onlineInvalidFile',
    network: 'onlineNetwork',
  } as const
  type ErrorCode = keyof typeof errorKeys

  let query = $state((media?.title ?? '').replace(/\.[a-z0-9]{1,5}$/i, ''))
  let language = $state(languageCodes.includes(preferred) ? preferred : '')
  let apiKey = $state(sessionKey)
  let hasConfiguredKey = $state(false)
  let editKey = $state(false)
  let ready = $state(false)
  let results = $state.raw<OpenSubtitleResult[]>([])
  let searched = $state(false)
  let working = $state<'search' | number | null>(null)
  let error = $state('')
  let matches = $state<HTMLUListElement>()
  let open = true
  let enabled = $derived(ready && !!query.trim() && (!!apiKey.trim() || hasConfiguredKey) && working === null)

  function languageLabel(code: string) {
    try { return names.of(languageTags[code] ?? code) || code }
    catch { return code }
  }
  function close() {
    open = false
    p.dialog = null
  }
  $effect(() => { if (p.media !== media) close() })

  function fail(e: unknown) {
    const code = String(e instanceof Error ? e.message : e)
    if (code === 'missing_key' || code === 'invalid_key') editKey = true
    error = code in errorKeys ? p.t(errorKeys[code as ErrorCode]) : p.t('onlineUnavailable')
  }
  onMount(() => {
    void backend.call('get_opensubtitles_config').then(config => {
      if (!open) return
      hasConfiguredKey = config.hasApiKey
      editKey = !hasConfiguredKey && !sessionKey
      ready = true
    }).catch(e => { if (open) fail(e) })
    return () => { open = false }
  })
  function resetResults() {
    results = []
    searched = false
    error = ''
  }
  async function search() {
    if (!enabled) return
    resetResults()
    working = 'search'
    const key = apiKey.trim()
    try {
      const rows = await backend.call('search_opensubtitles', { query: query.trim(), language, apiKey: key || undefined })
      if (open) {
        results = rows
        searched = true
        sessionKey = key
        editKey = false
      }
    } catch (e) {
      if (open) fail(e)
    } finally {
      if (open) working = null
    }
    await tick()
    if (open && searched) matches?.querySelector('button')?.focus()
  }
  async function load(row: OpenSubtitleResult) {
    if (working !== null) return
    working = row.fileId
    error = ''
    try {
      await subtitleTracks.importOnline(row.fileId, async () => {
        const file = await backend.call('download_opensubtitle', { fileId: row.fileId, apiKey: apiKey.trim() || undefined })
        return open && p.media === media ? file : null
      })
      if (open) close()
    } catch (e) {
      if (open) fail(e)
    } finally {
      if (open) working = null
    }
  }
</script>

<Dialog title="OpenSubtitles" icon="search" {close}>
  {#if ready}
    <form class="online-search" onsubmit={(e) => { e.preventDefault(); void search() }}>
      <label>{p.t('movieTitle')}
        <input type="search" maxlength="250" required bind:value={query} oninput={resetResults} disabled={working !== null} />
      </label>
      <label>{p.t('language')}
        <select bind:value={language} onchange={resetResults} disabled={working !== null}>
          <option value="">{p.t('allLanguages')}</option>
          {#each languageCodes as code}<option value={code}>{languageLabel(code)}</option>{/each}
        </select>
      </label>
      {#if editKey}
        <label>{p.t('apiKey')}
          <input type="password" autocomplete="off" spellcheck="false" bind:value={apiKey} disabled={working !== null}
            oninput={() => { error = ''; if (!apiKey) sessionKey = '' }} />
        </label>
        <div class="key-help">
          <button type="button" class="link-action" onclick={() => backend.call('open_opensubtitles_registration').catch(fail)}>
            {p.t('getApiKey')}<Icon name="link" size={13} />
          </button>
          <small>{p.t('keySession')}</small>
        </div>
      {/if}
      <div class="dialog-actions">
        {#if !editKey}<button type="button" class="edit-key link-action" disabled={working !== null}
          onclick={() => (editKey = true)}>{p.t('apiKey')}…</button>{/if}
        <button class="primary" type="submit" disabled={!enabled}>
          {working === 'search' ? p.t('subtitleSearching') : p.t('subtitleSearch')}
        </button>
      </div>
    </form>
  {/if}
  {#if error}<p class="form-error" role="alert">{error}</p>{/if}
  {#if searched}
    {#if results.length}
      <ul class="online-results" bind:this={matches}>
        {#each results as row (row.fileId)}
          <li>
            <div>
              <strong>{row.fileName || row.release}</strong>
              <small>{languageLabel(row.language)}{row.release && row.release !== row.fileName ? ' · ' + row.release : ''}</small>
            </div>
            <button disabled={working !== null} aria-label={p.t('useSubtitle') + ' · ' + (row.fileName || row.release)}
              onclick={() => load(row)}>
              <Icon name="download" size={14} />
              {working === row.fileId ? p.t('downloading') : p.t('useSubtitle')}
            </button>
          </li>
        {/each}
      </ul>
    {:else}<p class="empty-note" role="status">{p.t('noSubtitleMatches')}</p>{/if}
  {/if}
</Dialog>

<style>
  .online-search { display: grid; grid-template-columns: minmax(0, 3fr) minmax(0, 2fr); gap: 10px; }
  .online-search > label { min-width: 0; margin: 0; }
  .online-search > :nth-child(n+3) { grid-column: 1 / -1; }
  .online-search :is(select, input) { display: block; width: 100%; }
  .key-help { display: flex; flex-wrap: wrap; align-items: center; gap: 4px 10px; }
  .key-help .link-action { padding: 0; text-decoration: underline; font-size: 12px; }
  .online-search .dialog-actions { margin: 0; }
  .edit-key { margin-inline-end: auto; }
  .online-results { margin: 14px 0 0; padding: 0; list-style: none; }
  .online-results li { display: flex; align-items: center; gap: 10px; padding: 10px 0; border-top: 1px solid var(--hairline); }
  .online-results li > div { flex: 1; min-width: 0; }
  .online-results :is(strong,small) { display: block; overflow-wrap: anywhere; }
  .online-results strong { font-size: 13px; }
  .online-results small { margin-top: 3px; }
  .online-results button { flex: none; gap: 4px; padding: 4px 8px; }
</style>

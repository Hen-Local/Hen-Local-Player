<script lang="ts">
  import { onMount, tick, untrack } from 'svelte'
  import mediaExtensions from '../../../media-formats.json'
  import Dialog from './Dialog.svelte'
  import Icon from './Icon.svelte'
  import { backend, player as p } from './stores.svelte'
  import { subtitleTracks } from './subtitle-tracks.svelte'
  import type { OnlineSubtitleResult } from './backend'

  const media = untrack(() => p.media)
  const languageCodes = ['en', 'zh', 'yue', 'ja', 'ko', 'fr', 'de', 'es', 'it', 'pt', 'ar', 'da', 'el', 'fa', 'fi', 'he', 'hi', 'hu', 'id', 'ms', 'nl', 'pl', 'ro', 'ru', 'sv', 'th', 'tr', 'uk', 'vi']
  const languageTags: Record<string, string> = { 'zh-cn': 'zh-Hans', 'zh-tw': 'zh-Hant', 'zh-ca': 'yue', 'pt-pt': 'pt-PT', 'pt-br': 'pt-BR' }
  const preferred = untrack(() => p.settings.targetLanguage.toLowerCase().split('-')[0])
  const names = $derived(new Intl.DisplayNames([p.locale], { type: 'language' }))
  const errorKeys = {
    desktop_only: 'onlineDesktopOnly',
    invalid_query: 'onlineQueryInvalid', invalid_language: 'onlineLanguageInvalid', quota: 'onlineQuota',
    rate_limited: 'onlineRateLimit', unavailable: 'onlineUnavailable', invalid_subtitle: 'onlineInvalidFile',
    network: 'onlineNetwork',
  } as const
  type ErrorCode = keyof typeof errorKeys

  function suggestedTitle(name: string) {
    const stem = name.replace(/\.([a-z0-9]+)$/i, (suffix, ext: string) =>
      mediaExtensions.includes(ext.toLowerCase()) ? '' : suffix)
    const edition = /[\s._[(]+(?:\d{3,4}[pi]|blu[.-]?ray|web[.-]?(?:dl|rip)|b[dr]rip|dvd[.-]?rip|[xh][.-]?26[45]|hevc|av1)(?=$|[\s._)\]-]).*$/i
    let title = stem.replace(edition, '').trim()
    // A parenthesized release year is explicit. Drop a bare year only alongside
    // a release tag, retaining numeric titles and future years such as Blade Runner 2049.
    const bracketedYear = /[\s._]*[(\[](?:19|20)\d{2}[)\]]$/
    if (bracketedYear.test(title)) title = title.replace(bracketedYear, '') || title
    else if (title !== stem) {
      title = title.replace(/[\s._]+((?:19|20)\d{2})$/, (token, year: string) =>
        Number(year) <= new Date().getFullYear() + 1 ? '' : token)
    }
    if (!/\s/.test(title)) {
      // Scene filenames use dots as spaces. Keep decimals and dotted initials.
      title = title.replace(/\b(?:[A-Z]\.){2,}|\d+\.\d+|\./g, (token, at: number) => {
        if (token === '.') return ' '
        const next = title.slice(at + token.length)
        return token.endsWith('.') && /^[\p{L}\d]{2}/u.test(next) ? token + ' ' : token
      })
    }
    return title.replace(/_/g, ' ').trim() || stem
  }

  let query = $state(suggestedTitle(media?.title ?? ''))
  let language = $state(languageCodes.includes(preferred) ? preferred : '')
  let titleField: HTMLInputElement
  let results = $state.raw<OnlineSubtitleResult[]>([])
  let searched = $state(false)
  let working = $state<string | null>(null)
  let error = $state('')
  let matches = $state<HTMLUListElement>()
  let open = true
  let enabled = $derived(!!query.trim() && working === null)

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
    error = code in errorKeys ? p.t(errorKeys[code as ErrorCode]) : p.t('onlineUnavailable')
  }
  onMount(() => {
    void tick().then(() => { if (open) titleField.focus() })
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
    try {
      const rows = await backend.call('search_online_subtitles', { query: query.trim(), language })
      if (open) {
        results = rows
        searched = true
      }
    } catch (e) {
      if (open) fail(e)
    } finally {
      if (open) working = null
    }
    await tick()
    if (open && searched) matches?.querySelector('button')?.focus()
  }
  async function load(row: OnlineSubtitleResult) {
    if (working !== null) return
    working = row.fileId
    error = ''
    try {
      await subtitleTracks.importOnline(row.fileId, async () => {
        const file = await backend.call('download_online_subtitle', { fileId: row.fileId })
        if (!open || p.media !== media) return null
        return row.fileId.startsWith('guest:') && row.fileName ? { ...file, name: row.fileName } : file
      })
      if (open) close()
    } catch (e) {
      if (open) fail(e)
    } finally {
      if (open) working = null
    }
  }
</script>

<Dialog title={p.t('findOnlineSubtitles')} icon="search" {close}>
  <form class="online-search" onsubmit={(e) => { e.preventDefault(); void search() }}>
    <label>{p.t('movieTitle')}
      <input type="search" bind:this={titleField} maxlength="250" required bind:value={query} oninput={resetResults} disabled={working !== null} />
    </label>
    <label>{p.t('language')}
      <select bind:value={language} onchange={resetResults} disabled={working !== null}>
        <option value="">{p.t('allLanguages')}</option>
        {#each languageCodes as code}<option value={code}>{languageLabel(code)}</option>{/each}
      </select>
    </label>
    <div class="dialog-actions">
      <button class="primary" type="submit" disabled={!enabled}>
        {working === 'search' ? p.t('subtitleSearching') : p.t('subtitleSearch')}
      </button>
    </div>
  </form>
  {#if error}<p class="form-error" role="alert">{error}</p>{/if}
  {#if searched}
    {#if results.length}
      <ul class="online-results" bind:this={matches}>
        {#each results as row (row.fileId)}
          <li>
            <div>
              <strong>{row.fileName || row.release}</strong>
              <small>{languageLabel(row.language)}{row.release && row.release !== row.fileName ? ' · ' + row.release : ''} · {row.fileId.startsWith('opensubtitles:') ? 'OpenSubtitles' : 'Subtitle Search Engine'}</small>
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
  .online-search :is(select, input) { display: block; width: 100%; }
  .online-search .dialog-actions { grid-column: 1 / -1; margin: 0; }
  .online-results { margin: 14px 0 0; padding: 0; list-style: none; }
  .online-results li { display: flex; align-items: center; gap: 10px; padding: 10px 0; border-top: 1px solid var(--hairline); }
  .online-results li > div { flex: 1; min-width: 0; }
  .online-results :is(strong,small) { display: block; overflow-wrap: anywhere; }
  .online-results strong { font-size: 13px; }
  .online-results small { margin-top: 3px; }
  .online-results button { flex: none; gap: 4px; padding: 4px 8px; }
</style>

<script lang="ts">
  import Dialog from './Dialog.svelte'
  import { backend, player as p } from './stores.svelte'
  import { clock, size } from './format'
  import Icon from './Icon.svelte'
  import type { UrlInfo } from './types'
  let url = $state(''),
    info = $state<UrlInfo | null>(null),
    working = $state<'probe' | 'download' | null>(null),
    error = $state(''),
    showRights = $state(!localStorage.getItem('hlp.url.notice'))
  let active = true
  function close() {
    active = false
    if (working) void p.act(() => backend.call('cancel_download'))
    p.dialog = null
  }
  async function submit() {
    error = ''
    working = info ? 'download' : 'probe'
    p.download = null
    try {
      if (!info) {
        info = await backend.call('probe_url', { url: url.trim() })
      } else {
        if (showRights) {
          localStorage.setItem('hlp.url.notice', '1')
          showRights = false
        }
        await backend.call('download_url', { url: url.trim() })
        if (active) p.dialog = null
      }
    } catch (e) {
      if (active)
        error =
          String(e instanceof Error ? e.message : e)
            .trim()
            .split('\n')
            .pop() ?? ''
    } finally {
      working = null
    }
  }
</script>

<Dialog title={p.t('openUrl')} icon="link" {close}>
  <form
    onsubmit={(e) => {
      e.preventDefault()
      void submit()
    }}
  >
    <label>
      {p.t('url')}
      <input
        type="url"
        required
        placeholder={p.t('urlPlaceholder')}
        bind:value={url}
        oninput={() => {
          info = null
          error = ''
        }}
        disabled={!!working}
      />
    </label>
    {#if info}<div class="url-card">
        {#if info.thumbnail}<img src={info.thumbnail} alt="" />{:else}<Icon
            name="film"
            size={25}
          />{/if}
        <div>
          <strong>{info.title}</strong>
          <small>
            {info.extractor}{info.duration !== null ? ' · ' + clock(info.duration * 1000) : ''}
          </small>
        </div>
      </div>{/if}
    {#if showRights}<p class="help">{p.t('rights')}</p>{/if}
    {#if working === 'download'}<div class="download-progress">
        {#if p.download}<progress
            max={p.download.total || 1}
            value={p.download.total ? p.download.downloaded : undefined}
          ></progress>
          <small>
            {p.t('part')}
            {p.download.part}/{p.download.parts} · {p.download.total
              ? Math.round((100 * p.download.downloaded) / p.download.total) + '%'
              : size(p.download.downloaded)}{p.download.eta !== null
              ? ' · ' + clock(p.download.eta * 1000) + ' ' + p.t('eta')
              : ''}
          </small>{:else}<progress></progress>{/if}
      </div>{/if}
    {#if error}<p class="form-error" role="alert">
        {error}
        <br />
        {p.t('urlHint')}
      </p>{/if}
    <div class="dialog-actions">
      <button type="button" onclick={close}>{p.t('cancel')}</button>
      <button type="submit" class="primary" disabled={!!working || !url.trim()}>
        {working === 'probe'
          ? p.t('probing')
          : working
            ? p.t('downloading')
            : info
              ? p.t('download')
              : p.t('preview')}
      </button>
    </div>
  </form>
</Dialog>

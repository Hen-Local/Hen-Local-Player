import type { AppRelease, Backend } from './backend'

export const updateInterval = 24 * 60 * 60 * 1000

export class AppUpdates {
  version = $state('')
  hasAccess = $state<boolean | null>(null)
  releases = $state.raw<AppRelease[] | null>(null)
  selected = $state('')
  prepared = $state<string | null>(null)
  task = $state<'idle' | 'savingAccess' | 'checking' | 'downloading' | 'installing'>('idle')
  progress = $state<{ downloaded: number; total: number | null } | null>(null)
  error = $state('')
  private lastCheck = 0
  private accessRevision = 0
  private localRequest?: Promise<void>

  constructor(private backend: Backend) {}

  private clearReleases() {
    this.releases = null
    this.selected = ''
    this.prepared = null
    this.progress = null
  }

  private failed(cause: unknown) {
    const message = cause instanceof Error ? cause.message : String(cause)
    // Transport diagnostics may mention purchaser URLs. Only known codes reach the UI.
    this.error = ['UPDATE_ACCESS_REQUIRED', 'UPDATE_ACCESS_DENIED', 'UPDATE_RATE_LIMITED', 'UPDATE_FEED_UNAVAILABLE'].includes(message)
      ? message : 'UPDATE_FAILED'
    if (this.error === 'UPDATE_ACCESS_REQUIRED' || this.error === 'UPDATE_ACCESS_DENIED') {
      this.clearReleases()
      if (this.error === 'UPDATE_ACCESS_REQUIRED') this.hasAccess = false
    }
  }

  loadLocal() {
    const revision = this.accessRevision
    this.localRequest ??= Promise.all([
      this.backend.version().then(value => { this.version = value }),
      this.backend.call('app_update_access').then(async access => {
        if (revision !== this.accessRevision) return
        this.hasAccess = access
        if (!access) return
        const prepared = await this.backend.call('prepared_app_release')
        if (revision !== this.accessRevision) return
        this.prepared = prepared
        if (prepared && !this.selected) this.selected = prepared
      }),
    ]).then(() => undefined).catch(error => {
      if (revision !== this.accessRevision) return
      this.localRequest = undefined
      this.failed(error)
    })
    return this.localRequest
  }

  async setAccess(link: string) {
    if (this.task !== 'idle' && this.task !== 'checking') return false
    const value = link.trim()
    if (value) {
      try {
        const url = new URL(value)
        if (url.protocol !== 'https:' || !url.hostname || url.username || url.password || value.includes('#'))
          throw new Error()
      } catch {
        this.error = 'UPDATE_LINK_INVALID'
        return false
      }
    }
    // Invalidate an in-flight history lookup even if both accounts offer the same version.
    this.accessRevision++
    this.task = 'savingAccess'
    this.error = ''
    try {
      await this.backend.call('set_app_update_access', { url: value })
      this.hasAccess = Boolean(value)
      this.lastCheck = 0
      this.clearReleases()
      return true
    } catch {
      this.error = 'UPDATE_LINK_SAVE_FAILED'
      return false
    } finally {
      this.task = 'idle'
    }
  }

  async check() {
    await this.loadLocal()
    if (!this.hasAccess || this.task !== 'idle') return
    const revision = this.accessRevision
    this.task = 'checking'
    this.error = ''
    this.lastCheck = Date.now()
    try {
      const releases = await this.backend.call('list_app_releases')
      if (revision !== this.accessRevision) return
      this.releases = releases
      if (!releases.some(release => release.version === this.selected)) {
        this.selected = releases.find(release => release.installable)?.version ?? releases[0]?.version ?? ''
      }
    } catch (error) {
      if (revision === this.accessRevision) this.failed(error)
    } finally {
      if (revision === this.accessRevision) this.task = 'idle'
    }
  }

  async download(version: string) {
    if (!this.hasAccess || this.task !== 'idle' || !this.releases?.some(release => release.version === version && release.installable)) return
    this.task = 'downloading'
    this.progress = null
    this.selected = version
    this.error = ''
    let unlisten: (() => void) | undefined
    try {
      unlisten = await this.backend.listen('app-update://progress', event => {
        if (event.version === version) this.progress = { downloaded: event.downloaded, total: event.total }
      })
      await this.backend.call('download_app_release', { version })
      this.prepared = version
    } catch (error) {
      this.failed(error)
    } finally {
      unlisten?.()
      this.task = 'idle'
    }
  }

  async install() {
    const version = this.prepared
    if (!this.hasAccess || !version || this.task !== 'idle') return
    this.task = 'installing'
    this.error = ''
    try {
      await this.backend.call('install_app_release', { version })
    } catch (error) {
      this.failed(error)
      // Native verification removes an invalid staged copy so it can be downloaded again.
      if (this.prepared) this.prepared = await this.backend.call('prepared_app_release').catch(() => this.prepared)
    } finally {
      this.task = 'idle'
    }
  }

  async automatic(enabledAndIdle: () => boolean) {
    if (!enabledAndIdle()) return
    await this.loadLocal()
    if (!this.hasAccess || !enabledAndIdle() || this.task !== 'idle') return
    const revision = this.accessRevision
    if (Date.now() - this.lastCheck >= updateInterval) await this.check()
    const latest = this.releases?.find(release => release.installable)
    if (revision !== this.accessRevision || this.error || !enabledAndIdle() || !latest || this.prepared === latest.version) return
    await this.download(latest.version)
    return this.prepared === latest.version ? latest.version : undefined
  }
}

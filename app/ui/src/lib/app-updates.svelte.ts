import type { AppRelease, Backend } from './backend'

export const updateInterval = 24 * 60 * 60 * 1000

export class AppUpdates {
  version = $state('')
  releases = $state.raw<AppRelease[] | null>(null)
  selected = $state('')
  prepared = $state<string | null>(null)
  task = $state<'idle' | 'checking' | 'downloading' | 'installing'>('idle')
  progress = $state<{ downloaded: number; total: number | null } | null>(null)
  error = $state('')
  private lastCheck = 0
  private localRequest?: Promise<void>

  constructor(private backend: Backend) {}

  private failed(cause: unknown) {
    const message = cause instanceof Error ? cause.message : String(cause)
    this.error = ['UPDATE_RATE_LIMITED', 'UPDATE_FEED_UNAVAILABLE'].includes(message) ? message : 'UPDATE_FAILED'
  }

  loadLocal() {
    this.localRequest ??= Promise.all([
      this.backend.version().then(value => { this.version = value }),
      this.backend.call('prepared_app_release').then(value => {
        this.prepared = value
        if (value && !this.selected) this.selected = value
      }),
    ]).then(() => undefined).catch(error => {
      this.localRequest = undefined
      this.failed(error)
    })
    return this.localRequest
  }

  async check() {
    if (this.task !== 'idle') return
    this.task = 'checking'
    this.error = ''
    this.lastCheck = Date.now()
    try {
      const [releases] = await Promise.all([this.backend.call('list_app_releases'), this.loadLocal()])
      this.releases = releases
      this.selected = releases.find(release => release.installable)?.version ?? releases[0]?.version ?? ''
    } catch (error) {
      this.failed(error)
    } finally {
      this.task = 'idle'
    }
  }

  async download(version: string) {
    if (this.task !== 'idle' || !this.releases?.some(release => release.version === version && release.installable)) return
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
    if (!version || this.task !== 'idle') return
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
    if (!enabledAndIdle() || this.task !== 'idle') return
    if (Date.now() - this.lastCheck >= updateInterval) await this.check()
    const latest = this.releases?.find(release => release.installable)
    if (this.error || !enabledAndIdle() || !latest || this.prepared === latest.version) return
    await this.download(latest.version)
    return this.prepared === latest.version ? latest.version : undefined
  }
}

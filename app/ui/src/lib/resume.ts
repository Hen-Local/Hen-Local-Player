// Store only local viewing positions. The browser and Tauri keep distinct origins.
const key = 'hen-local-player.resume.v1'
type Position = { path: string; ms: number }

export class ResumeHistory {
  private entries = new Map<string, number>()

  constructor() {
    try {
      const saved: unknown = JSON.parse(localStorage.getItem(key) ?? '[]')
      if (Array.isArray(saved)) {
        for (const row of saved.slice(-200)) {
          if (row && typeof row.path === 'string' && Number.isFinite(row.ms) && row.ms >= 0)
            this.entries.set(row.path, row.ms)
        }
      }
    } catch {
      // Playback remains available when local storage is restricted.
    }
  }

  get(path: string) {
    return this.entries.get(path) ?? 0
  }

  save(path: string, ms: number, duration: number) {
    if (!path || !Number.isFinite(ms) || !Number.isFinite(duration) || duration <= 0) return
    // A finished movie starts from the beginning the next time it opens.
    const value = ms >= 1000 && ms < duration - Math.min(3000, duration * 0.05) ? Math.round(ms) : 0
    if (value === this.get(path)) return
    this.entries.delete(path)
    if (value) this.entries.set(path, value)
    if (this.entries.size > 200) this.entries.delete(this.entries.keys().next().value!)
    const records: Position[] = [...this.entries].map(([path, ms]) => ({ path, ms }))
    try {
      localStorage.setItem(key, JSON.stringify(records))
    } catch {
      // Retain positions in memory for the rest of this session.
    }
  }
}

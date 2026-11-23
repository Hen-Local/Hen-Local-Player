// Per-movie choices stay on this origin. Imported caption text lives in IndexedDB
// so a large file never blocks the playback thread on a localStorage write.
export type FileEntry = { id: string; name: string }
export type TrackPreference = {
  choice?: string
  last?: string
  delayMs?: number
  files?: FileEntry[]
}
const prefix = 'hen-player.subtitle-choice.v1.'
export function getPreference(path: string): TrackPreference {
  try {
    const saved = JSON.parse(localStorage.getItem(prefix + path) ?? '{}')
    return {
      choice: typeof saved?.choice === 'string' ? saved.choice : undefined,
      last: typeof saved?.last === 'string' ? saved.last : undefined,
      delayMs: Number.isSafeInteger(saved?.delayMs) ? saved.delayMs : 0,
      files: Array.isArray(saved?.files)
        ? saved.files.filter(
            (f: FileEntry) =>
              f &&
              typeof f.name === 'string' &&
              typeof f.id === 'string' &&
              f.id.startsWith('file:'),
          )
        : [],
    }
  } catch {
    return {}
  }
}

export function putPreference(path: string, pref: TrackPreference) {
  if (!path) return
  try {
    localStorage.setItem(prefix + path, JSON.stringify(pref))
  } catch {
    /* Private browsing can restrict storage. */
  }
}

async function importedFiles(): Promise<IDBDatabase> {
  return new Promise((resolve, reject) => {
    const open = indexedDB.open('hen-player.subtitle-files', 1)
    open.onupgradeneeded = () => open.result.createObjectStore('files')
    open.onsuccess = () => resolve(open.result)
    open.onerror = () => reject(open.error)
  })
}

export async function importedText(
  path: string,
  id: string,
  save?: string,
): Promise<string | undefined> {
  const db = await importedFiles()
  try {
    return await new Promise((resolve, reject) => {
      const tx = db.transaction('files', save === undefined ? 'readonly' : 'readwrite')
      const store = tx.objectStore('files')
      const key = [path, id]
      const request = save === undefined ? store.get(key) : store.put(save, key)
      tx.oncomplete = () => resolve(save ?? request.result)
      tx.onabort = () => reject(tx.error)
      tx.onerror = () => reject(tx.error)
    })
  } finally {
    db.close()
  }
}

import { defaultSubtitleLanguages } from './languages'
import type { Settings } from './types'

export const defaultSubtitleMode = 'translation'

export const subtitleAppearanceDefaults = {
  subtitleScale: 1,
  subtitleBottomOffset: 0.05,
  subtitleBackground: 0.7,
} as const

export function defaults(): Settings {
  return {
    autoDownloadUpdates: false,
    uiLanguage: 'system',
    accentTheme: /Mac/.test(navigator.platform) ? 'system' : 'neon-green',
    ...defaultSubtitleLanguages(navigator.languages.length ? navigator.languages : [navigator.language]),
    subtitleMode: defaultSubtitleMode,
    bilingualOrder: 'translation-first',
    ...subtitleAppearanceDefaults,
    showTranscriptPanel: false,
    lookaheadSeconds: 180,
    processWholeFile: true,
    dialogueChannelFocus: true,
    asrModel: 'qwen3-asr-1.7b',
    modelDownloadSource: 'auto',
    translationContext: true,
    downloadDir: '',
    cacheLimitGb: 30,
  }
}

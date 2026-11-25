import { defaultTarget } from './languages'
import type { Settings } from './types'

export const subtitleAppearanceDefaults = {
  subtitleScale: 1,
  subtitleBottomOffset: 0.05,
  subtitleBackground: 0.7,
} as const

export function defaults(): Settings {
  return {
    autoDownloadUpdates: false,
    uiLanguage: 'system',
    accentTheme: 'neon-blue',
    targetLanguage: defaultTarget(navigator.language),
    sourceLanguage: 'auto',
    subtitleMode: 'both',
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

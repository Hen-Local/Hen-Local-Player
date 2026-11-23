import { defaultTarget } from './languages'
import type { Settings } from './types'
export function defaults(): Settings {
  return {
    uiLanguage: 'system',
    accentTheme: 'neon-blue',
    targetLanguage: defaultTarget(navigator.language),
    sourceLanguage: 'auto',
    subtitleMode: 'both',
    bilingualOrder: 'translation-first',
    subtitleScale: 1,
    subtitleBackground: 0,
    subtitleBottomOffset: 0.06,
    showTranscriptPanel: false,
    lookaheadSeconds: 180,
    processWholeFile: true,
    dialogueChannelFocus: true,
    asrModel: 'qwen3-asr-1.7b',
    translationContext: false,
    downloadDir: '',
    cacheLimitGb: 30,
  }
}

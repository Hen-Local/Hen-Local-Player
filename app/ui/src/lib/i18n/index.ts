import en from './en'
import zh from './zh'
import { languages } from '../languages'

export type Messages = Record<keyof typeof en, string>
export type MessageKey = keyof Messages
export const interfaceLanguages = languages.filter(language => language.spoken)
const known = new Set(interfaceLanguages.map(language => language.code))
const catalogs = import.meta.glob<Messages>('./locales/*.json', { import: 'default' })

export function resolveUiLanguage(preference: string, systemLanguages: readonly string[]) {
  const candidates = preference === 'system' ? systemLanguages : [preference]
  for (const tag of candidates) {
    const normalized = tag.trim().toLowerCase().replaceAll('_', '-')
    const [base, extlang] = normalized.split('-')
    if (base === 'zh' && extlang === 'yue') return 'yue'
    if (base === 'fil') return 'tl'
    if (known.has(base)) return base
  }
  return 'en'
}

export async function loadMessages(locale: string): Promise<Messages> {
  if (locale === 'en') return en
  if (locale === 'zh') return zh
  return catalogs[`./locales/${locale}.json`]()
}

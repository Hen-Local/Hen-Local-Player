// Mirrors the canonical language table in crates/hlp-core/src/lang.rs.
export const languages = [
  { code: 'zh', en: 'Chinese', zh: '中文', spoken: true, target: true },
  { code: 'en', en: 'English', zh: '英语', spoken: true, target: true },
  { code: 'fr', en: 'French', zh: '法语', spoken: true, target: true },
  { code: 'pt', en: 'Portuguese', zh: '葡萄牙语', spoken: true, target: true },
  { code: 'es', en: 'Spanish', zh: '西班牙语', spoken: true, target: true },
  { code: 'ja', en: 'Japanese', zh: '日语', spoken: true, target: true },
  { code: 'tr', en: 'Turkish', zh: '土耳其语', spoken: true, target: true },
  { code: 'ru', en: 'Russian', zh: '俄语', spoken: true, target: true },
  { code: 'ar', en: 'Arabic', zh: '阿拉伯语', spoken: true, target: true },
  { code: 'ko', en: 'Korean', zh: '韩语', spoken: true, target: true },
  { code: 'th', en: 'Thai', zh: '泰语', spoken: true, target: true },
  { code: 'it', en: 'Italian', zh: '意大利语', spoken: true, target: true },
  { code: 'de', en: 'German', zh: '德语', spoken: true, target: true },
  { code: 'vi', en: 'Vietnamese', zh: '越南语', spoken: true, target: true },
  { code: 'ms', en: 'Malay', zh: '马来语', spoken: true, target: true },
  { code: 'id', en: 'Indonesian', zh: '印尼语', spoken: true, target: true },
  { code: 'tl', en: 'Filipino', zh: '菲律宾语', spoken: true, target: true },
  { code: 'hi', en: 'Hindi', zh: '印地语', spoken: true, target: true },
  { code: 'zh-Hant', en: 'Traditional Chinese', zh: '繁体中文', spoken: false, target: true },
  { code: 'pl', en: 'Polish', zh: '波兰语', spoken: true, target: true },
  { code: 'cs', en: 'Czech', zh: '捷克语', spoken: true, target: true },
  { code: 'nl', en: 'Dutch', zh: '荷兰语', spoken: true, target: true },
  { code: 'km', en: 'Khmer', zh: '高棉语', spoken: false, target: true },
  { code: 'my', en: 'Burmese', zh: '缅甸语', spoken: false, target: true },
  { code: 'fa', en: 'Persian', zh: '波斯语', spoken: true, target: true },
  { code: 'gu', en: 'Gujarati', zh: '古吉拉特语', spoken: false, target: true },
  { code: 'ur', en: 'Urdu', zh: '乌尔都语', spoken: false, target: true },
  { code: 'te', en: 'Telugu', zh: '泰卢固语', spoken: false, target: true },
  { code: 'mr', en: 'Marathi', zh: '马拉地语', spoken: false, target: true },
  { code: 'he', en: 'Hebrew', zh: '希伯来语', spoken: false, target: true },
  { code: 'bn', en: 'Bengali', zh: '孟加拉语', spoken: false, target: true },
  { code: 'ta', en: 'Tamil', zh: '泰米尔语', spoken: false, target: true },
  { code: 'uk', en: 'Ukrainian', zh: '乌克兰语', spoken: false, target: true },
  { code: 'bo', en: 'Tibetan', zh: '藏语', spoken: false, target: true },
  { code: 'kk', en: 'Kazakh', zh: '哈萨克语', spoken: false, target: true },
  { code: 'mn', en: 'Mongolian', zh: '蒙古语', spoken: false, target: true },
  { code: 'ug', en: 'Uyghur', zh: '维吾尔语', spoken: false, target: true },
  { code: 'yue', en: 'Cantonese', zh: '粤语', spoken: true, target: true },
  { code: 'sv', en: 'Swedish', zh: '瑞典语', spoken: true, target: false },
  { code: 'da', en: 'Danish', zh: '丹麦语', spoken: true, target: false },
  { code: 'fi', en: 'Finnish', zh: '芬兰语', spoken: true, target: false },
  { code: 'el', en: 'Greek', zh: '希腊语', spoken: true, target: false },
  { code: 'ro', en: 'Romanian', zh: '罗马尼亚语', spoken: true, target: false },
  { code: 'hu', en: 'Hungarian', zh: '匈牙利语', spoken: true, target: false },
  { code: 'mk', en: 'Macedonian', zh: '马其顿语', spoken: true, target: false },
]

export function languageName(code: string | null, locale: 'en' | 'zh', native = false) {
  if (!code) return ''
  const label = languages.find((l) => l.code === code)?.[locale] ?? code
  if (!native) return label
  try {
    const endonym = new Intl.DisplayNames([code], { type: 'language' }).of(code)
    return endonym && endonym.toLocaleLowerCase() !== label.toLocaleLowerCase()
      ? label + ' · ' + endonym
      : label
  } catch {
    return label
  }
}
export function defaultTarget(locale: string) {
  const value = locale.toLowerCase()
  if (/^zh-(tw|hk|hant)/.test(value)) return 'zh-Hant'
  const base = value.split('-')[0]
  return languages.some((l) => l.target && l.code === base) ? base : 'en'
}

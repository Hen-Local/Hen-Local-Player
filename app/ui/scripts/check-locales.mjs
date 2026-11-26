import assert from 'node:assert/strict'
import { fileURLToPath } from 'node:url'
import { createServer } from 'vite'

const server = await createServer({
  root: fileURLToPath(new URL('../', import.meta.url)),
  configFile: false,
  appType: 'custom',
  server: { middlewareMode: true, hmr: false, watch: null },
  optimizeDeps: { noDiscovery: true },
})

try {
  const { interfaceLanguages, resolveUiLanguage, loadMessages } =
    await server.ssrLoadModule('/src/lib/i18n/index.ts')
  const english = await loadMessages('en')
  const keys = Object.keys(english).sort()
  const tokens = value => (value.match(/\{\w+\}/g) ?? []).sort()
  for (const { code } of interfaceLanguages) {
    const messages = await loadMessages(code)
    assert.deepEqual(Object.keys(messages).sort(), keys, code + ' has missing or unexpected strings')
    for (const key of keys) {
      assert.equal(typeof messages[key], 'string', code + '.' + key)
      assert.ok(messages[key].trim(), code + '.' + key + ' is empty')
      assert.deepEqual(tokens(messages[key]), tokens(english[key]), code + '.' + key)
    }
    assert.equal(messages.app, 'Hen Local Player', code + '.app')
    assert.equal(resolveUiLanguage(code, ['en']), code)
  }
  assert.equal(resolveUiLanguage('system', ['uk-UA', 'FIL_ph']), 'tl')
  assert.equal(resolveUiLanguage('system', ['zh-yue-HK']), 'yue')
  assert.equal(resolveUiLanguage('system', ['pt-BR']), 'pt')
  assert.equal(resolveUiLanguage('system', ['ar-SA']), 'ar')
  assert.equal(resolveUiLanguage('system', ['zz']), 'en')
  console.log(`Checked ${keys.length} messages in each of ${interfaceLanguages.length} ASR interface languages.`)
} finally {
  await server.close()
}

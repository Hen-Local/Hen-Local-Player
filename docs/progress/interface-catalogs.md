# Interface translation catalogs

The player now ships interface text for the 30 spoken languages in its Qwen3-ASR inventory. All 28 additional catalogs live in `app/ui/src/lib/i18n/locales`. Their strings are included in the application. Selecting a language works offline without downloading models.

The new catalogs include player controls, subtitle settings, transcription and translation status, setup and update messages, menu commands, and file-picker labels. Drafts produced with the local Hy-MT2 model were reviewed and edited for playback terminology. Several catalogs, including Macedonian and Romanian, were written in full. Transcription, subtitle translation and playback have distinct labels. The product name remains `Hen Local Player` in every language.

- `node app/ui/scripts/check-locales.mjs` verified all 320 message keys and their placeholders in all 30 interface languages.
- An additional local comparison verified every new catalog preserved the product and component names, UTF encodings, model identifiers and URL placeholder wherever they occur in the English source.

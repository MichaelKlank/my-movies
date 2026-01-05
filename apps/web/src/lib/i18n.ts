import i18n from 'i18next'
import { initReactI18next } from 'react-i18next'
import de from '../locales/de.json'
import en from '../locales/en.json'
import fr from '../locales/fr.json'
import es from '../locales/es.json'
import it from '../locales/it.json'
import pt from '../locales/pt.json'
import ru from '../locales/ru.json'
import ja from '../locales/ja.json'
import ko from '../locales/ko.json'
import zh from '../locales/zh.json'

// Map TMDB language codes to i18next locales
export function tmdbToI18nLocale(tmdbLang: string | null | undefined): string {
  if (!tmdbLang) {
    // Use browser language as fallback
    const browserLang = navigator.language.split('-')[0]
    return mapBrowserLangToLocale(browserLang)
  }
  
  const langCode = tmdbLang.split('-')[0].toLowerCase()
  return mapBrowserLangToLocale(langCode) || 'en'
}

function mapBrowserLangToLocale(langCode: string): string {
  switch (langCode.toLowerCase()) {
    case 'de': return 'de'
    case 'en': return 'en'
    case 'fr': return 'fr'
    case 'es': return 'es'
    case 'it': return 'it'
    case 'pt': return 'pt'
    case 'ru': return 'ru'
    case 'ja': return 'ja'
    case 'ko': return 'ko'
    case 'zh': return 'zh'
    default: return 'en'
  }
}

i18n
  .use(initReactI18next)
  .init({
    resources: {
      de: { translation: de },
      en: { translation: en },
      fr: { translation: fr },
      es: { translation: es },
      it: { translation: it },
      pt: { translation: pt },
      ru: { translation: ru },
      ja: { translation: ja },
      ko: { translation: ko },
      zh: { translation: zh },
    },
    lng: 'en', // default language (English is the base)
    fallbackLng: 'en',
    interpolation: {
      escapeValue: false, // React already escapes values
    },
  })

export default i18n

import { useEffect } from 'react'
import { useTranslation } from 'react-i18next'
import { useQuery } from '@tanstack/react-query'
import { api } from '@/lib/api'
import { tmdbToI18nLocale } from '@/lib/i18n'

export function I18nProvider({ children }: { children: React.ReactNode }) {
  const { i18n } = useTranslation()
  
  // Get user's language preference
  const { data: user } = useQuery({
    queryKey: ['me'],
    queryFn: () => api.me(),
    retry: false,
    staleTime: Infinity, // Cache invalidation happens via WebSocket events
  })

  // Update locale when user language changes
  useEffect(() => {
    if (user?.language) {
      const newLocale = tmdbToI18nLocale(user.language)
      if (i18n.language !== newLocale) {
        i18n.changeLanguage(newLocale)
      }
    } else {
      // Use browser language as fallback
      const browserLang = navigator.language.split('-')[0]
      const defaultLocale = tmdbToI18nLocale(browserLang)
      if (i18n.language !== defaultLocale) {
        i18n.changeLanguage(defaultLocale)
      }
    }
  }, [user?.language, i18n])

  return <>{children}</>
}

export function useI18n() {
  const { t } = useTranslation()
  return { t }
}

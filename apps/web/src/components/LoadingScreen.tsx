import { useI18n } from '@/hooks/useI18n'

export function LoadingScreen() {
  const { t } = useI18n()
  
  return (
    <div className="flex min-h-screen items-center justify-center">
      <div className="text-muted-foreground">{t('common.loading')}</div>
    </div>
  )
}


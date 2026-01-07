import { createFileRoute, redirect } from '@tanstack/react-router'
import { useState } from 'react'
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query'
import { Settings, Key, CheckCircle, AlertCircle, Loader2, Eye, EyeOff } from 'lucide-react'
import { api, SettingStatus } from '@/lib/api'
import { useAuth } from '@/hooks/useAuth'
import { useI18n } from '@/hooks/useI18n'

export const Route = createFileRoute('/settings')({
  beforeLoad: ({ context }) => {
    if (!context.auth.isAuthenticated) {
      throw redirect({ to: '/login' })
    }
  },
  component: SettingsPage,
})

function SettingsPage() {
  const { user } = useAuth()
  const { t } = useI18n()

  // Only admins can access settings
  if (user?.role !== 'admin') {
    return (
      <div className="container mx-auto px-4 py-8">
        <div className="rounded-lg border border-destructive/50 bg-destructive/10 p-6 text-center">
          <AlertCircle className="mx-auto h-12 w-12 text-destructive" />
          <h2 className="mt-4 text-xl font-semibold">{t('settings.accessDenied')}</h2>
          <p className="mt-2 text-muted-foreground">
            {t('settings.onlyAdmins')}
          </p>
        </div>
      </div>
    )
  }

  const { data: settings, isLoading } = useQuery({
    queryKey: ['settings'],
    queryFn: () => api.getSettings(),
  })

  if (isLoading) {
    return (
      <div className="container mx-auto px-4 py-8">
        <div className="flex items-center justify-center">
          <Loader2 className="h-8 w-8 animate-spin" />
        </div>
      </div>
    )
  }

  return (
    <div className="container mx-auto px-4 py-8">
      <div className="mb-8">
        <h1 className="flex items-center gap-3 text-3xl font-bold">
          <Settings className="h-8 w-8" />
          {t('settings.title')}
        </h1>
        <p className="mt-2 text-muted-foreground">
          {t('settings.subtitle')}
        </p>
      </div>

      <div className="space-y-6">
        {settings?.map((setting) => (
          <SettingCard key={setting.key} setting={setting} />
        ))}
      </div>

      <div className="mt-8 rounded-lg border bg-muted/50 p-4">
        <h3 className="font-medium">{t('settings.note')}</h3>
        <p className="mt-1 text-sm text-muted-foreground">
          {t('settings.noteText')}
        </p>
      </div>
    </div>
  )
}

function SettingCard({ setting }: { setting: SettingStatus }) {
  const { t } = useI18n()
  const [isEditing, setIsEditing] = useState(false)
  const [value, setValue] = useState('')
  const [showValue, setShowValue] = useState(false)
  const queryClient = useQueryClient()

  const updateMutation = useMutation({
    mutationFn: (newValue: string) => api.updateSetting(setting.key, newValue),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['settings'] })
      setIsEditing(false)
      setValue('')
    },
  })

  const testMutation = useMutation({
    mutationFn: () => api.testTmdb(),
  })

  const handleSave = () => {
    if (value.trim()) {
      updateMutation.mutate(value.trim())
    }
  }

  return (
    <div className="rounded-lg border bg-card p-4 md:p-6">
      <div className="flex items-start justify-between">
        <div className="flex items-start gap-3 md:gap-4 flex-1 min-w-0">
          <div className="rounded-full bg-primary/10 p-2 shrink-0">
            <Key className="h-4 w-4 md:h-5 md:w-5 text-primary" />
          </div>
          <div className="flex-1 min-w-0">
            <h3 className="font-semibold text-sm md:text-base">{getSettingTitle(setting.key)}</h3>
            <p className="mt-1 text-xs md:text-sm text-muted-foreground break-words">
              {setting.key === 'tmdb_api_key' ? t('settings.tmdbApiKeyDesc') : setting.description}
            </p>
            <div className="mt-2 flex flex-wrap items-center gap-2">
              <code className="rounded bg-muted px-2 py-0.5 text-xs break-all">{setting.env_var}</code>
              <StatusBadge setting={setting} />
            </div>
          </div>
        </div>
      </div>

      {isEditing ? (
        <div className="mt-4 space-y-3">
          <div className="relative">
            <input
              type={showValue ? 'text' : 'password'}
              value={value}
              onChange={(e) => setValue(e.target.value)}
              placeholder={`${setting.env_var} ${t('settings.enterValue')}`}
              className="w-full rounded-md border bg-background px-4 py-3 pr-10 text-base md:text-sm focus:border-primary focus:outline-none focus:ring-1 focus:ring-primary min-h-touch"
              autoFocus
            />
            <button
              type="button"
              onClick={() => setShowValue(!showValue)}
              className="absolute right-2 top-1/2 -translate-y-1/2 text-muted-foreground hover:text-foreground active:text-foreground min-h-touch min-w-touch flex items-center justify-center"
            >
              {showValue ? <EyeOff className="h-4 w-4" /> : <Eye className="h-4 w-4" />}
            </button>
          </div>
          <div className="flex flex-col sm:flex-row gap-2">
            <button
              onClick={handleSave}
              disabled={!value.trim() || updateMutation.isPending}
              className="flex items-center justify-center rounded-md bg-primary px-4 py-3 text-sm font-medium text-primary-foreground hover:bg-primary/90 active:bg-primary/80 disabled:opacity-50 min-h-touch w-full sm:w-auto"
            >
              {updateMutation.isPending ? t('settings.saving') : t('common.save')}
            </button>
            <button
              onClick={() => {
                setIsEditing(false)
                setValue('')
              }}
              className="flex items-center justify-center rounded-md border px-4 py-3 text-sm font-medium hover:bg-muted active:bg-muted/80 min-h-touch w-full sm:w-auto"
            >
              {t('common.cancel')}
            </button>
          </div>
          {updateMutation.isError && (
            <p className="text-sm text-destructive">
              {t('settings.error')}: {updateMutation.error instanceof Error ? updateMutation.error.message : t('settings.unknownError')}
            </p>
          )}
        </div>
      ) : (
        <div className="mt-4 flex flex-col sm:flex-row items-stretch sm:items-center gap-2">
          <button
            onClick={() => setIsEditing(true)}
            className="flex items-center justify-center rounded-md border px-4 py-3 text-sm font-medium hover:bg-muted active:bg-muted/80 min-h-touch w-full sm:w-auto"
          >
            {setting.is_configured ? t('settings.change') : t('settings.configure')}
          </button>
          
          {setting.key === 'tmdb_api_key' && setting.is_configured && (
            <button
              onClick={() => testMutation.mutate()}
              disabled={testMutation.isPending}
              className="flex items-center justify-center gap-2 rounded-md border px-4 py-3 text-sm font-medium hover:bg-muted active:bg-muted/80 disabled:opacity-50 min-h-touch w-full sm:w-auto"
            >
              {testMutation.isPending ? (
                <>
                  <Loader2 className="h-4 w-4 animate-spin" />
                  <span className="hidden sm:inline">{t('settings.testApi')}</span>
                </>
              ) : (
                t('settings.testApi')
              )}
            </button>
          )}
        </div>
      )}

      {testMutation.data && (
        <div
          className={`mt-4 rounded-md p-3 text-sm ${
            testMutation.data.success
              ? 'bg-green-500/10 text-green-700 dark:text-green-400'
              : 'bg-destructive/10 text-destructive'
          }`}
        >
          {testMutation.data.message}
        </div>
      )}
    </div>
  )
}

function StatusBadge({ setting }: { setting: SettingStatus }) {
  const { t } = useI18n()
  
  if (!setting.is_configured) {
    return (
      <span className="inline-flex items-center gap-1 rounded-full bg-yellow-500/10 px-2 py-0.5 text-xs font-medium text-yellow-700 dark:text-yellow-400">
        <AlertCircle className="h-3 w-3" />
        {t('users.notConfigured')}
      </span>
    )
  }

  if (setting.source === 'environment') {
    return (
      <span className="inline-flex items-center gap-1 rounded-full bg-blue-500/10 px-2 py-0.5 text-xs font-medium text-blue-700 dark:text-blue-400">
        <CheckCircle className="h-3 w-3" />
        {t('settings.viaEnvironment')}
      </span>
    )
  }

  return (
    <span className="inline-flex items-center gap-1 rounded-full bg-green-500/10 px-2 py-0.5 text-xs font-medium text-green-700 dark:text-green-400">
      <CheckCircle className="h-3 w-3" />
      {t('users.configured')}
    </span>
  )
}

function getSettingTitle(key: string): string {
  const { t } = useI18n()
  switch (key) {
    case 'tmdb_api_key':
      return t('settings.tmdbApiKey')
    default:
      return key
  }
}


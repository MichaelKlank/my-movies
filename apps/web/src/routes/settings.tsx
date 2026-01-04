import { createFileRoute, redirect } from '@tanstack/react-router'
import { useState } from 'react'
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query'
import { Settings, Key, CheckCircle, AlertCircle, Loader2, Eye, EyeOff } from 'lucide-react'
import { api, SettingStatus } from '@/lib/api'
import { useAuth } from '@/hooks/useAuth'

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

  // Only admins can access settings
  if (user?.role !== 'admin') {
    return (
      <div className="container mx-auto px-4 py-8">
        <div className="rounded-lg border border-destructive/50 bg-destructive/10 p-6 text-center">
          <AlertCircle className="mx-auto h-12 w-12 text-destructive" />
          <h2 className="mt-4 text-xl font-semibold">Zugriff verweigert</h2>
          <p className="mt-2 text-muted-foreground">
            Nur Administratoren können die Einstellungen ändern.
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
          Einstellungen
        </h1>
        <p className="mt-2 text-muted-foreground">
          Konfiguriere API-Keys und andere Einstellungen für My Movies.
        </p>
      </div>

      <div className="space-y-6">
        {settings?.map((setting) => (
          <SettingCard key={setting.key} setting={setting} />
        ))}
      </div>

      <div className="mt-8 rounded-lg border bg-muted/50 p-4">
        <h3 className="font-medium">Hinweis</h3>
        <p className="mt-1 text-sm text-muted-foreground">
          Einstellungen, die über Umgebungsvariablen gesetzt sind, haben Vorrang vor den hier
          gespeicherten Werten. In Docker oder Server-Deployments solltest du die .env Datei
          verwenden.
        </p>
      </div>
    </div>
  )
}

function SettingCard({ setting }: { setting: SettingStatus }) {
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
    <div className="rounded-lg border bg-card p-6">
      <div className="flex items-start justify-between">
        <div className="flex items-start gap-4">
          <div className="rounded-full bg-primary/10 p-2">
            <Key className="h-5 w-5 text-primary" />
          </div>
          <div>
            <h3 className="font-semibold">{getSettingTitle(setting.key)}</h3>
            <p className="mt-1 text-sm text-muted-foreground">{setting.description}</p>
            <div className="mt-2 flex items-center gap-2">
              <code className="rounded bg-muted px-2 py-0.5 text-xs">{setting.env_var}</code>
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
              placeholder={`${setting.env_var} eingeben...`}
              className="w-full rounded-md border bg-background px-3 py-2 pr-10 text-sm focus:border-primary focus:outline-none focus:ring-1 focus:ring-primary"
              autoFocus
            />
            <button
              type="button"
              onClick={() => setShowValue(!showValue)}
              className="absolute right-2 top-1/2 -translate-y-1/2 text-muted-foreground hover:text-foreground"
            >
              {showValue ? <EyeOff className="h-4 w-4" /> : <Eye className="h-4 w-4" />}
            </button>
          </div>
          <div className="flex gap-2">
            <button
              onClick={handleSave}
              disabled={!value.trim() || updateMutation.isPending}
              className="rounded-md bg-primary px-4 py-2 text-sm font-medium text-primary-foreground hover:bg-primary/90 disabled:opacity-50"
            >
              {updateMutation.isPending ? 'Speichern...' : 'Speichern'}
            </button>
            <button
              onClick={() => {
                setIsEditing(false)
                setValue('')
              }}
              className="rounded-md border px-4 py-2 text-sm font-medium hover:bg-muted"
            >
              Abbrechen
            </button>
          </div>
          {updateMutation.isError && (
            <p className="text-sm text-destructive">
              Fehler: {updateMutation.error instanceof Error ? updateMutation.error.message : 'Unbekannter Fehler'}
            </p>
          )}
        </div>
      ) : (
        <div className="mt-4 flex items-center gap-2">
          <button
            onClick={() => setIsEditing(true)}
            className="rounded-md border px-4 py-2 text-sm font-medium hover:bg-muted"
          >
            {setting.is_configured ? 'Ändern' : 'Konfigurieren'}
          </button>
          
          {setting.key === 'tmdb_api_key' && setting.is_configured && (
            <button
              onClick={() => testMutation.mutate()}
              disabled={testMutation.isPending}
              className="rounded-md border px-4 py-2 text-sm font-medium hover:bg-muted disabled:opacity-50"
            >
              {testMutation.isPending ? (
                <Loader2 className="h-4 w-4 animate-spin" />
              ) : (
                'API testen'
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
  if (!setting.is_configured) {
    return (
      <span className="inline-flex items-center gap-1 rounded-full bg-yellow-500/10 px-2 py-0.5 text-xs font-medium text-yellow-700 dark:text-yellow-400">
        <AlertCircle className="h-3 w-3" />
        Nicht konfiguriert
      </span>
    )
  }

  if (setting.source === 'environment') {
    return (
      <span className="inline-flex items-center gap-1 rounded-full bg-blue-500/10 px-2 py-0.5 text-xs font-medium text-blue-700 dark:text-blue-400">
        <CheckCircle className="h-3 w-3" />
        Via Umgebungsvariable
      </span>
    )
  }

  return (
    <span className="inline-flex items-center gap-1 rounded-full bg-green-500/10 px-2 py-0.5 text-xs font-medium text-green-700 dark:text-green-400">
      <CheckCircle className="h-3 w-3" />
      Konfiguriert
    </span>
  )
}

function getSettingTitle(key: string): string {
  switch (key) {
    case 'tmdb_api_key':
      return 'TMDB API Key'
    default:
      return key
  }
}


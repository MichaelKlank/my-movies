import { createFileRoute, redirect } from '@tanstack/react-router'
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query'
import { useState, useEffect, useRef } from 'react'
import { User, Settings, Globe, Shield, Save, Loader2, Upload, X, ImagePlus, ChevronDown } from 'lucide-react'
import { api } from '@/lib/api'
import { useI18n } from '@/hooks/useI18n'
import { useAuth } from '@/hooks/useAuth'
import { Avatar } from '@/components/Avatar'

export const Route = createFileRoute('/me')({
  beforeLoad: ({ context }) => {
    if (!context.auth.isAuthenticated) {
      throw redirect({ to: '/login' })
    }
  },
  component: ProfilePage,
})

// Common TMDB language codes
const LANGUAGES = [
  { code: null, labelKey: 'profile.systemLanguage', flag: '🌐' },
  { code: 'de-DE', label: 'Deutsch', flag: '🇩🇪' },
  { code: 'en-US', label: 'English (US)', flag: '🇺🇸' },
  { code: 'en-GB', label: 'English (UK)', flag: '🇬🇧' },
  { code: 'fr-FR', label: 'Français', flag: '🇫🇷' },
  { code: 'es-ES', label: 'Español', flag: '🇪🇸' },
  { code: 'it-IT', label: 'Italiano', flag: '🇮🇹' },
  { code: 'pt-BR', label: 'Português (BR)', flag: '🇧🇷' },
  { code: 'ru-RU', label: 'Русский', flag: '🇷🇺' },
  { code: 'ja-JP', label: '日本語', flag: '🇯🇵' },
  { code: 'ko-KR', label: '한국어', flag: '🇰🇷' },
  { code: 'zh-CN', label: '中文 (简体)', flag: '🇨🇳' },
] as const

function ProfilePage() {
  const { t } = useI18n()
  const { updateUser } = useAuth()
  const queryClient = useQueryClient()
  const [selectedLanguage, setSelectedLanguage] = useState<string | null>(null)
  const [includeAdult, setIncludeAdult] = useState(false)
  const [avatarPreview, setAvatarPreview] = useState<string | null>(null)
  const [showLanguageDropdown, setShowLanguageDropdown] = useState(false)
  const fileInputRef = useRef<HTMLInputElement>(null)
  const languageDropdownRef = useRef<HTMLDivElement>(null)

  const { data: user, isLoading } = useQuery({
    queryKey: ['me'],
    queryFn: () => api.me(),
  })

  useEffect(() => {
    if (user) {
      setSelectedLanguage(user.language || null)
      setIncludeAdult(user.include_adult)
    }
  }, [user])

  // Detect system language on mount
  useEffect(() => {
    if (!user?.language && navigator.language) {
      // Map browser language to TMDB format
      const browserLang = navigator.language
      const systemLang = LANGUAGES.find(
        (lang) => lang.code && browserLang.startsWith(lang.code.split('-')[0])
      )?.code || null
      
      if (systemLang && selectedLanguage === null) {
        setSelectedLanguage(systemLang)
      }
    }
  }, [user, selectedLanguage])

  // Close language dropdown when clicking outside
  useEffect(() => {
    const handleClickOutside = (event: MouseEvent) => {
      if (languageDropdownRef.current && !languageDropdownRef.current.contains(event.target as Node)) {
        setShowLanguageDropdown(false)
      }
    }
    if (showLanguageDropdown) {
      document.addEventListener('mousedown', handleClickOutside)
      return () => document.removeEventListener('mousedown', handleClickOutside)
    }
  }, [showLanguageDropdown])

  const updateLanguageMutation = useMutation({
    mutationFn: (language: string | null) => api.updateLanguage(language),
    onSuccess: (updatedUser) => {
      // Optimistic update - WebSocket event will handle invalidation
      queryClient.setQueryData(['me'], updatedUser)
      queryClient.setQueryData(['user'], updatedUser)
      updateUser(updatedUser)
    },
  })

  const updateIncludeAdultMutation = useMutation({
    mutationFn: (includeAdult: boolean) => api.updateIncludeAdult(includeAdult),
    onSuccess: (updatedUser) => {
      // Optimistic update - WebSocket event will handle invalidation
      queryClient.setQueryData(['me'], updatedUser)
      queryClient.setQueryData(['user'], updatedUser)
      updateUser(updatedUser)
    },
  })

  const handleLanguageChange = (language: string | null) => {
    setSelectedLanguage(language)
    updateLanguageMutation.mutate(language)
  }

  const handleIncludeAdultChange = (checked: boolean) => {
    setIncludeAdult(checked)
    updateIncludeAdultMutation.mutate(checked)
  }

  const uploadAvatarMutation = useMutation({
    mutationFn: (file: File) => api.uploadAvatar(file),
    onSuccess: (data) => {
      // Optimistic update - WebSocket event will handle invalidation
      if (data.user) {
        queryClient.setQueryData(['me'], data.user)
        queryClient.setQueryData(['user'], data.user)
        updateUser(data.user)
      }
      setAvatarPreview(null)
    },
  })

  const deleteAvatarMutation = useMutation({
    mutationFn: () => api.deleteAvatar(),
    onSuccess: (data) => {
      // Optimistic update - WebSocket event will handle invalidation
      if (data.user) {
        queryClient.setQueryData(['me'], data.user)
        queryClient.setQueryData(['user'], data.user)
        updateUser(data.user)
      }
      setAvatarPreview(null)
    },
  })

  const handleFileSelect = (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0]
    if (!file) return

    if (!file.type.startsWith('image/')) {
      alert(t('poster.invalidFileType'))
      return
    }

    // Create preview
    const reader = new FileReader()
    reader.onloadend = () => {
      setAvatarPreview(reader.result as string)
    }
    reader.readAsDataURL(file)
  }

  const handleUpload = () => {
    const file = fileInputRef.current?.files?.[0]
    if (!file) return
    uploadAvatarMutation.mutate(file)
  }

  const handleDeleteAvatar = () => {
    if (confirm(t('common.confirm') + ': ' + t('profile.deleteAvatar') + '?')) {
      deleteAvatarMutation.mutate()
    }
  }

  if (isLoading) {
    return (
      <div className="container mx-auto px-4 py-8">
        <div className="flex items-center justify-center gap-2">
          <Loader2 className="h-8 w-8 animate-spin" />
          <span className="text-muted-foreground">{t('common.loading')}</span>
        </div>
      </div>
    )
  }

  if (!user) {
    return (
      <div className="container mx-auto px-4 py-8">
        <div className="text-center text-muted-foreground">{t('common.loading')}</div>
      </div>
    )
  }

  return (
    <div className="container mx-auto px-4 py-4 md:py-8 max-w-2xl">
      <div className="mb-6 md:mb-8">
        <h1 className="flex items-center gap-3 text-2xl md:text-3xl font-bold">
          <User className="h-6 w-6 md:h-8 md:w-8" />
          {t('profile.title')}
        </h1>
        <p className="mt-2 text-sm md:text-base text-muted-foreground">
          {t('profile.subtitle')}
        </p>
      </div>

      <div className="space-y-4 md:space-y-6">
        {/* Avatar Section */}
        <div className="rounded-lg border bg-card p-4 md:p-6">
          <h2 className="text-lg md:text-xl font-semibold mb-4">{t('profile.avatar')}</h2>
          <div className="flex flex-col md:flex-row items-center md:items-start gap-4 md:gap-6">
            <div className="relative group">
              <Avatar user={user} size="xl" />
              <div className="absolute inset-0 bg-black/60 opacity-0 group-hover:opacity-100 transition-opacity rounded-full flex items-center justify-center">
                <ImagePlus className="h-6 w-6 text-white" />
              </div>
            </div>
            <div className="flex-1 w-full md:w-auto space-y-3">
              <div>
                <input
                  ref={fileInputRef}
                  type="file"
                  accept="image/*"
                  onChange={handleFileSelect}
                  className="hidden"
                />
                {avatarPreview ? (
                  <div className="space-y-2">
                    <img
                      src={avatarPreview}
                      alt="Preview"
                      className="h-20 w-20 rounded-full object-cover"
                    />
                    <div className="flex flex-col sm:flex-row gap-2">
                      <button
                        onClick={handleUpload}
                        disabled={uploadAvatarMutation.isPending}
                        className="flex items-center justify-center gap-2 rounded-md bg-primary px-4 py-3 text-sm text-primary-foreground hover:bg-primary/90 active:bg-primary/80 disabled:opacity-50 min-h-touch w-full sm:w-auto"
                      >
                        {uploadAvatarMutation.isPending ? (
                          <>
                            <Loader2 className="h-4 w-4 animate-spin" />
                            {t('common.loading')}
                          </>
                        ) : (
                          <>
                            <Save className="h-4 w-4" />
                            {t('common.save')}
                          </>
                        )}
                      </button>
                      <button
                        onClick={() => {
                          setAvatarPreview(null)
                          if (fileInputRef.current) fileInputRef.current.value = ''
                        }}
                        className="flex items-center justify-center gap-2 rounded-md bg-secondary px-4 py-3 text-sm hover:bg-secondary/80 active:bg-secondary/60 min-h-touch w-full sm:w-auto"
                      >
                        <X className="h-4 w-4" />
                        {t('common.cancel')}
                      </button>
                    </div>
                  </div>
                ) : (
                  <div className="space-y-2">
                    <button
                      onClick={() => fileInputRef.current?.click()}
                      className="flex items-center justify-center gap-2 rounded-md bg-secondary px-4 py-3 text-sm hover:bg-secondary/80 active:bg-secondary/60 min-h-touch w-full sm:w-auto"
                    >
                      <Upload className="h-4 w-4" />
                      {t('profile.uploadAvatar')}
                    </button>
                    {user.avatar_path && (
                      <button
                        onClick={handleDeleteAvatar}
                        disabled={deleteAvatarMutation.isPending}
                        className="flex items-center justify-center gap-2 rounded-md bg-destructive/10 text-destructive px-4 py-3 text-sm hover:bg-destructive/20 active:bg-destructive/30 disabled:opacity-50 min-h-touch w-full sm:w-auto"
                      >
                        {deleteAvatarMutation.isPending ? (
                          <>
                            <Loader2 className="h-4 w-4 animate-spin" />
                            {t('common.loading')}
                          </>
                        ) : (
                          <>
                            <X className="h-4 w-4" />
                            {t('profile.deleteAvatar')}
                          </>
                        )}
                      </button>
                    )}
                  </div>
                )}
              </div>
              <p className="text-xs text-muted-foreground">
                {t('profile.supportedFormats')}
              </p>
            </div>
          </div>
        </div>

        {/* User Info Card */}
        <div className="rounded-lg border bg-card p-4 md:p-6">
          <h2 className="text-lg md:text-xl font-semibold mb-4">{t('profile.userInfo')}</h2>
          <div className="space-y-3">
            <div>
              <label className="text-xs md:text-sm font-medium text-muted-foreground">{t('profile.username')}</label>
              <p className="text-base md:text-lg font-medium break-words">{user.username}</p>
            </div>
            <div>
              <label className="text-xs md:text-sm font-medium text-muted-foreground">{t('profile.email')}</label>
              <p className="text-base md:text-lg break-words">{user.email}</p>
            </div>
            <div>
              <label className="text-xs md:text-sm font-medium text-muted-foreground">{t('profile.role')}</label>
              <div className="flex items-center gap-2 mt-1">
                {user.role === 'admin' ? (
                  <>
                    <Shield className="h-4 w-4 text-primary" />
                    <span className="text-base md:text-lg font-medium">{t('profile.roleAdmin')}</span>
                  </>
                ) : (
                  <span className="text-base md:text-lg">{t('profile.roleUser')}</span>
                )}
              </div>
            </div>
            <div>
              <label className="text-xs md:text-sm font-medium text-muted-foreground">{t('profile.registeredAt')}</label>
              <p className="text-base md:text-lg">
                {new Date(user.created_at).toLocaleDateString('de-DE', {
                  year: 'numeric',
                  month: 'long',
                  day: 'numeric',
                })}
              </p>
            </div>
          </div>
        </div>

        {/* Language Settings */}
        <div className="rounded-lg border bg-card p-4 md:p-6">
          <div className="flex items-center gap-2 mb-4">
            <Globe className="h-5 w-5" />
            <h2 className="text-lg md:text-xl font-semibold">{t('profile.language')}</h2>
          </div>
          <p className="text-xs md:text-sm text-muted-foreground mb-4">
            {t('profile.languageDesc')}
          </p>
          <div className="relative" ref={languageDropdownRef}>
            <button
              type="button"
              onClick={() => setShowLanguageDropdown(!showLanguageDropdown)}
              disabled={updateLanguageMutation.isPending}
              className="w-full flex items-center justify-between rounded-md border bg-background px-4 py-3 text-sm focus:outline-none focus:ring-2 focus:ring-primary disabled:opacity-50 min-h-touch"
            >
              <span className="flex items-center gap-2">
                {(() => {
                  const currentLang = LANGUAGES.find(l => l.code === selectedLanguage) || LANGUAGES[0]
                  return (
                    <>
                      <span className="text-lg">{currentLang.flag}</span>
                      <span>{'labelKey' in currentLang ? t(currentLang.labelKey) : currentLang.label}</span>
                    </>
                  )
                })()}
              </span>
              <ChevronDown className={`h-4 w-4 transition-transform ${showLanguageDropdown ? 'rotate-180' : ''}`} />
            </button>
            {showLanguageDropdown && (
              <div className="absolute z-50 w-full mt-1 bg-card border rounded-md shadow-lg max-h-60 overflow-auto">
                {LANGUAGES.map((lang) => (
                  <button
                    key={lang.code || 'system'}
                    type="button"
                    onClick={() => {
                      handleLanguageChange(lang.code)
                      setShowLanguageDropdown(false)
                    }}
                    className={`w-full flex items-center gap-2 px-4 py-3 text-sm text-left hover:bg-muted active:bg-muted/80 transition-colors min-h-touch ${
                      selectedLanguage === lang.code ? 'bg-muted' : ''
                    }`}
                  >
                    <span className="text-lg">{lang.flag}</span>
                    <span>{'labelKey' in lang ? t(lang.labelKey) : lang.label}</span>
                  </button>
                ))}
              </div>
            )}
          </div>
          {updateLanguageMutation.isPending && (
            <div className="flex items-center gap-2 mt-2 text-sm text-muted-foreground">
              <Loader2 className="h-4 w-4 animate-spin" />
              {t('common.loading')}
            </div>
          )}
        </div>

        {/* Content Settings */}
        <div className="rounded-lg border bg-card p-4 md:p-6">
          <div className="flex items-center gap-2 mb-4">
            <Settings className="h-5 w-5" />
            <h2 className="text-lg md:text-xl font-semibold">{t('profile.contentSettings')}</h2>
          </div>
          <div className="flex items-center justify-between gap-4">
            <div className="flex-1">
              <label className="text-xs md:text-sm font-medium">{t('profile.includeAdult')}</label>
              <p className="text-xs md:text-sm text-muted-foreground mt-1">
                {t('profile.includeAdultDesc')}
              </p>
            </div>
            <label className="relative inline-flex items-center cursor-pointer shrink-0">
              <input
                type="checkbox"
                checked={includeAdult}
                onChange={(e) => handleIncludeAdultChange(e.target.checked)}
                disabled={updateIncludeAdultMutation.isPending}
                className="sr-only peer"
              />
              <div className="w-11 h-6 bg-gray-200 peer-focus:outline-none peer-focus:ring-4 peer-focus:ring-primary/20 rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-gray-300 after:border after:rounded-full after:h-5 after:w-5 after:transition-all peer-checked:bg-primary"></div>
            </label>
          </div>
          {updateIncludeAdultMutation.isPending && (
            <div className="flex items-center gap-2 mt-2 text-sm text-muted-foreground">
              <Loader2 className="h-4 w-4 animate-spin" />
              {t('common.loading')}
            </div>
          )}
        </div>
      </div>
    </div>
  )
}


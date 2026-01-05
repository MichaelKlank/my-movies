import { createFileRoute, Link, useNavigate } from '@tanstack/react-router'
import { useState } from 'react'
import { Film, ArrowLeft, Check } from 'lucide-react'
import { api } from '@/lib/api'
import { useI18n } from '@/hooks/useI18n'

export const Route = createFileRoute('/reset-password')({
  component: ResetPasswordPage,
})

function ResetPasswordPage() {
  const { t } = useI18n()
  const navigate = useNavigate()
  const searchParams = new URLSearchParams(window.location.search)
  const token = searchParams.get('token') || ''

  const [password, setPassword] = useState('')
  const [confirmPassword, setConfirmPassword] = useState('')
  const [isLoading, setIsLoading] = useState(false)
  const [success, setSuccess] = useState(false)
  const [error, setError] = useState('')

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault()
    setError('')

    if (password !== confirmPassword) {
      setError(t('resetPassword.passwordsDontMatch'))
      return
    }

    if (password.length < 6) {
      setError(t('resetPassword.passwordTooShort'))
      return
    }

    setIsLoading(true)

    try {
      await api.resetPassword(token, password)
      setSuccess(true)
      // Redirect to login after 3 seconds
      setTimeout(() => {
        navigate({ to: '/login' })
      }, 3000)
    } catch (err) {
      setError(err instanceof Error ? err.message : t('settings.unknownError'))
    } finally {
      setIsLoading(false)
    }
  }

  if (!token) {
    return (
      <div className="flex min-h-screen items-center justify-center px-4">
        <div className="w-full max-w-sm space-y-6 text-center">
          <div className="mx-auto flex h-12 w-12 items-center justify-center rounded-full bg-destructive/10">
            <Film className="h-6 w-6 text-destructive" />
          </div>
          <h1 className="text-2xl font-bold">{t('resetPassword.invalidLink')}</h1>
          <p className="text-muted-foreground">
            {t('resetPassword.invalidLinkDesc')}
          </p>
          <Link
            to="/forgot-password"
            className="inline-flex items-center gap-2 text-sm text-primary hover:underline"
          >
            {t('resetPassword.requestNewLink')}
          </Link>
        </div>
      </div>
    )
  }

  return (
    <div className="flex min-h-screen items-center justify-center px-4">
      <div className="w-full max-w-sm space-y-6">
        <div className="text-center">
          <div className="mx-auto flex h-12 w-12 items-center justify-center rounded-full bg-primary">
            <Film className="h-6 w-6 text-primary-foreground" />
          </div>
          <h1 className="mt-4 text-2xl font-bold">{t('resetPassword.title')}</h1>
          <p className="text-muted-foreground">
            {t('resetPassword.subtitle')}
          </p>
        </div>

        {success ? (
          <div className="space-y-4">
            <div className="rounded-md bg-green-500/10 p-4 text-sm text-green-600">
              <div className="flex items-center gap-2">
                <Check className="h-5 w-5" />
                <p className="font-medium">{t('resetPassword.passwordChanged')}</p>
              </div>
              <p className="mt-1">
                {t('resetPassword.redirecting')}
              </p>
            </div>
          </div>
        ) : (
          <form onSubmit={handleSubmit} className="space-y-4">
            {error && (
              <div className="rounded-md bg-destructive/10 p-3 text-sm text-destructive">
                {error}
              </div>
            )}

            <div className="space-y-2">
              <label htmlFor="password" className="text-sm font-medium">
                {t('resetPassword.newPassword')}
              </label>
              <input
                id="password"
                type="password"
                value={password}
                onChange={e => setPassword(e.target.value)}
                className="w-full rounded-md border bg-background px-3 py-2 text-sm focus:border-primary focus:outline-none focus:ring-1 focus:ring-primary"
                placeholder="••••••••"
                required
                minLength={6}
              />
            </div>

            <div className="space-y-2">
              <label htmlFor="confirmPassword" className="text-sm font-medium">
                {t('resetPassword.confirmPassword')}
              </label>
              <input
                id="confirmPassword"
                type="password"
                value={confirmPassword}
                onChange={e => setConfirmPassword(e.target.value)}
                className="w-full rounded-md border bg-background px-3 py-2 text-sm focus:border-primary focus:outline-none focus:ring-1 focus:ring-primary"
                placeholder="••••••••"
                required
                minLength={6}
              />
            </div>

            <button
              type="submit"
              disabled={isLoading}
              className="w-full rounded-md bg-primary px-4 py-2 text-sm font-medium text-primary-foreground hover:bg-primary/90 disabled:opacity-50"
            >
              {isLoading ? t('settings.saving') : t('auth.changePassword')}
            </button>

            <Link
              to="/login"
              className="flex items-center justify-center gap-2 text-sm text-muted-foreground hover:text-foreground"
            >
              <ArrowLeft className="h-4 w-4" />
              {t('resetPassword.backToLogin')}
            </Link>
          </form>
        )}
      </div>
    </div>
  )
}


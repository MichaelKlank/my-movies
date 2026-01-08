import { createFileRoute, useNavigate, useRouter, redirect, Link } from '@tanstack/react-router'
import { useState, useEffect } from 'react'
import { Film } from 'lucide-react'
import { useAuth } from '@/hooks/useAuth'
import { useI18n } from '@/hooks/useI18n'

export const Route = createFileRoute('/login')({
  beforeLoad: ({ context }) => {
    if (context.auth.isAuthenticated) {
      throw redirect({ to: '/' })
    }
  },
  component: LoginPage,
})

function LoginPage() {
  const { t } = useI18n()
  const [isRegister, setIsRegister] = useState(false)
  const [username, setUsername] = useState('')
  const [email, setEmail] = useState('')
  const [password, setPassword] = useState('')
  const [error, setError] = useState('')
  const [isLoading, setIsLoading] = useState(false)

  const { login, register, isAuthenticated } = useAuth()
  const navigate = useNavigate()
  const router = useRouter()

  // Redirect if already authenticated (fallback for beforeLoad)
  useEffect(() => {
    if (isAuthenticated) {
      navigate({ to: '/' })
    }
  }, [isAuthenticated, navigate])

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault()
    setError('')
    setIsLoading(true)

    try {
      if (isRegister) {
        await register(username, email, password)
      } else {
        await login(username, password)
      }
      await router.invalidate()
      await navigate({ to: '/' })
    } catch (err) {
      setError(err instanceof Error ? err.message : t('settings.unknownError'))
    } finally {
      setIsLoading(false)
    }
  }

  return (
    <div className="flex min-h-screen items-center justify-center px-4 py-safe-top pb-safe-bottom">
      <div className="w-full max-w-sm space-y-6">
        <div className="text-center">
          <div className="mx-auto flex h-12 w-12 items-center justify-center rounded-full bg-primary">
            <Film className="h-6 w-6 text-primary-foreground" />
          </div>
          <h1 className="mt-4 text-xl md:text-2xl font-bold">My Movies</h1>
          <p className="text-sm md:text-base text-muted-foreground">
            {isRegister ? t('auth.register') : t('auth.login')}
          </p>
        </div>

        <form onSubmit={handleSubmit} className="space-y-4">
          {error && (
            <div className="rounded-md bg-destructive/10 p-3 text-sm text-destructive">
              {error}
            </div>
          )}

          <div className="space-y-2">
            <label htmlFor="username" className="text-sm font-medium">
              {t('auth.username')}
            </label>
            <input
              id="username"
              type="text"
              value={username}
              onChange={e => setUsername(e.target.value)}
              className="w-full rounded-md border bg-background px-4 py-3 text-base md:text-sm focus:border-primary focus:outline-none focus:ring-1 focus:ring-primary min-h-touch"
              required
              autoComplete="username"
            />
          </div>

          {isRegister && (
            <div className="space-y-2">
              <label htmlFor="email" className="text-sm font-medium">
                {t('auth.email')}
              </label>
              <input
                id="email"
                type="email"
                value={email}
                onChange={e => setEmail(e.target.value)}
                className="w-full rounded-md border bg-background px-4 py-3 text-base md:text-sm focus:border-primary focus:outline-none focus:ring-1 focus:ring-primary min-h-touch"
                required
                autoComplete="email"
              />
            </div>
          )}

          <div className="space-y-2">
            <label htmlFor="password" className="text-sm font-medium">
              {t('auth.password')}
            </label>
            <input
              id="password"
              type="password"
              value={password}
              onChange={e => setPassword(e.target.value)}
              className="w-full rounded-md border bg-background px-4 py-3 text-base md:text-sm focus:border-primary focus:outline-none focus:ring-1 focus:ring-primary min-h-touch"
              required
              autoComplete={isRegister ? 'new-password' : 'current-password'}
            />
          </div>

          <button
            type="submit"
            disabled={isLoading}
            className="w-full rounded-md bg-primary px-4 py-3 text-base md:text-sm font-medium text-primary-foreground hover:bg-primary/90 active:bg-primary/80 disabled:opacity-50 min-h-touch"
          >
            {isLoading ? t('common.loading') : isRegister ? t('auth.register') : t('auth.login')}
          </button>
        </form>

        <p className="text-center text-xs md:text-sm text-muted-foreground">
          {isRegister ? t('auth.alreadyHaveAccount') : t('auth.noAccountYet')}{' '}
          <button
            type="button"
            onClick={() => setIsRegister(!isRegister)}
            className="font-medium underline hover:text-foreground active:text-foreground min-h-touch min-w-touch"
          >
            {isRegister ? t('auth.login') : t('auth.register')}
          </button>
        </p>

        {!isRegister && (
          <p className="text-center text-xs md:text-sm">
            <Link
              to="/forgot-password"
              className="text-muted-foreground hover:text-foreground active:text-foreground underline min-h-touch min-w-touch inline-block"
            >
              {t('auth.forgotPassword')}
            </Link>
          </p>
        )}
      </div>
    </div>
  )
}

import { createFileRoute, useNavigate, useRouter, redirect, Link } from '@tanstack/react-router'
import { useState, useEffect } from 'react'
import { Film } from 'lucide-react'
import { useAuth } from '@/hooks/useAuth'
import { useI18n } from '@/hooks/useI18n'
import { Button } from '@/components/ui/Button'
import { Input } from '@/components/ui/Input'

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
  const [confirmPassword, setConfirmPassword] = useState('')
  const [error, setError] = useState('')
  const [isLoading, setIsLoading] = useState(false)

  const { login, register, isAuthenticated } = useAuth()
  const navigate = useNavigate()
  const router = useRouter()

  useEffect(() => {
    if (isAuthenticated) {
      navigate({ to: '/' })
    }
  }, [isAuthenticated, navigate])

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault()
    setError('')

    if (isRegister) {
      if (password.length < 6) {
        setError(t('resetPassword.passwordTooShort'))
        return
      }
      if (password !== confirmPassword) {
        setError(t('resetPassword.passwordsDontMatch'))
        return
      }
    }

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
            <Input
              id="username"
              type="text"
              value={username}
              onChange={e => setUsername(e.target.value)}
              required
              autoComplete="username"
            />
          </div>

          {isRegister && (
            <div className="space-y-2">
              <label htmlFor="email" className="text-sm font-medium">
                {t('auth.email')}
              </label>
              <Input
                id="email"
                type="email"
                value={email}
                onChange={e => setEmail(e.target.value)}
                required
                autoComplete="email"
              />
            </div>
          )}

          <div className="space-y-2">
            <label htmlFor="password" className="text-sm font-medium">
              {t('auth.password')}
            </label>
            <Input
              id="password"
              type="password"
              value={password}
              onChange={e => setPassword(e.target.value)}
              required
              autoComplete={isRegister ? 'new-password' : 'current-password'}
            />
          </div>

          {isRegister && (
            <div className="space-y-2">
              <label htmlFor="confirmPassword" className="text-sm font-medium">
                {t('resetPassword.confirmPassword')}
              </label>
              <Input
                id="confirmPassword"
                type="password"
                value={confirmPassword}
                onChange={e => setConfirmPassword(e.target.value)}
                required
                autoComplete="new-password"
              />
            </div>
          )}

          <Button type="submit" isLoading={isLoading} className="w-full">
            {isRegister ? t('auth.register') : t('auth.login')}
          </Button>
        </form>

        <p className="text-center text-xs md:text-sm text-muted-foreground">
          {isRegister ? t('auth.alreadyHaveAccount') : t('auth.noAccountYet')}{' '}
          <button
            type="button"
            onClick={() => {
              setIsRegister(!isRegister)
              setConfirmPassword('')
              setError('')
            }}
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

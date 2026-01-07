import { createRootRouteWithContext, Outlet, Link, useNavigate } from '@tanstack/react-router'
import { TanStackRouterDevtools } from '@tanstack/react-router-devtools'
import { Film, Tv, ScanLine, Upload, LogOut, User, Users, Settings } from 'lucide-react'
import { useAuth } from '@/hooks/useAuth'
import { useI18n } from '@/hooks/useI18n'
import { useWebSocketSync } from '@/hooks/useWebSocket'
import { Avatar } from '@/components/Avatar'

interface RouterContext {
  auth: ReturnType<typeof useAuth>
}

export const Route = createRootRouteWithContext<RouterContext>()({
  component: RootLayout,
})

function RootLayout() {
  const { isAuthenticated, user, logout } = useAuth()
  const { t } = useI18n()
  const navigate = useNavigate()

  // Set up WebSocket sync for real-time updates
  useWebSocketSync()

  const handleLogout = () => {
    logout()
    navigate({ to: '/login' })
  }

  if (!isAuthenticated) {
    return (
      <div className="min-h-screen bg-background">
        <Outlet />
      </div>
    )
  }

  return (
    <div className="min-h-screen bg-background flex flex-col">
      {/* Header - Hidden on mobile, shown on desktop */}
      <header className="hidden md:block border-b bg-card pt-safe-top">
        <div className="container flex h-14 items-center justify-between px-4">
          <Link to="/" className="flex items-center gap-2 font-semibold">
            <Film className="h-5 w-5" />
            <span>My Movies</span>
          </Link>

          <nav className="flex items-center gap-1">
            <Link
              to="/movies"
              className="flex items-center gap-2 rounded-md px-3 py-2 text-sm hover:bg-accent [&.active]:bg-accent min-h-touch min-w-touch"
            >
              <Film className="h-4 w-4" />
              <span>{t('nav.movies')}</span>
            </Link>
            <Link
              to="/series"
              className="flex items-center gap-2 rounded-md px-3 py-2 text-sm hover:bg-accent [&.active]:bg-accent min-h-touch min-w-touch"
            >
              <Tv className="h-4 w-4" />
              <span>{t('nav.series')}</span>
            </Link>
            <Link
              to="/scan"
              className="flex items-center gap-2 rounded-md px-3 py-2 text-sm hover:bg-accent [&.active]:bg-accent min-h-touch min-w-touch"
            >
              <ScanLine className="h-4 w-4" />
              <span>{t('nav.scan')}</span>
            </Link>
            <Link
              to="/import"
              className="flex items-center gap-2 rounded-md px-3 py-2 text-sm hover:bg-accent [&.active]:bg-accent min-h-touch min-w-touch"
            >
              <Upload className="h-4 w-4" />
              <span>{t('nav.import')}</span>
            </Link>
          </nav>

          <div className="flex items-center gap-1">
            <Link
              to="/me"
              className="flex items-center justify-center h-10 w-10 rounded-md hover:bg-accent [&.active]:bg-accent min-h-touch min-w-touch"
              title={t('nav.profile')}
            >
              {user ? (
                <Avatar user={user} size="sm" />
              ) : (
                <User className="h-5 w-5" />
              )}
            </Link>
            {user?.role === 'admin' && (
              <>
                <Link
                  to="/users"
                  className="flex items-center justify-center h-10 w-10 rounded-md hover:bg-accent [&.active]:bg-accent min-h-touch min-w-touch"
                  title={t('nav.users')}
                >
                  <Users className="h-5 w-5" />
                </Link>
                <Link
                  to="/settings"
                  className="flex items-center justify-center h-10 w-10 rounded-md hover:bg-accent [&.active]:bg-accent min-h-touch min-w-touch"
                  title={t('nav.settings')}
                >
                  <Settings className="h-5 w-5" />
                </Link>
              </>
            )}
            <button
              onClick={handleLogout}
              className="flex items-center justify-center h-10 w-10 rounded-md hover:bg-accent min-h-touch min-w-touch"
              title={t('nav.logout')}
            >
              <LogOut className="h-5 w-5" />
            </button>
          </div>
        </div>
      </header>

      {/* Mobile Header - Simple title bar */}
      <header className="md:hidden border-b bg-card pt-safe-top">
        <div className="flex h-14 items-center justify-between px-4">
          <Link to="/" className="flex items-center gap-2 font-semibold">
            <Film className="h-5 w-5" />
            <span className="text-base">My Movies</span>
          </Link>
          <Link
            to="/me"
            className="flex items-center justify-center h-10 w-10 rounded-md hover:bg-accent [&.active]:bg-accent min-h-touch min-w-touch"
            title={t('nav.profile')}
          >
            {user ? (
              <Avatar user={user} size="sm" />
            ) : (
              <User className="h-5 w-5" />
            )}
          </Link>
        </div>
      </header>

      {/* Main content */}
      <main className="flex-1 container px-4 py-4 md:py-6 pb-20 md:pb-6 overflow-y-auto">
        <Outlet />
      </main>

      {/* Bottom Navigation Bar - Mobile only */}
      <nav className="md:hidden fixed bottom-0 left-0 right-0 border-t bg-card pb-safe-bottom z-50">
        <div className="flex items-center justify-around h-16">
          <Link
            to="/"
            className="flex flex-col items-center justify-center gap-1 flex-1 h-full min-h-touch [&.active]:text-primary [&.active]:bg-accent/50"
            title={t('nav.dashboard')}
          >
            <Film className="h-5 w-5" />
            <span className="text-xs">{t('nav.dashboard')}</span>
          </Link>
          <Link
            to="/movies"
            className="flex flex-col items-center justify-center gap-1 flex-1 h-full min-h-touch [&.active]:text-primary [&.active]:bg-accent/50"
            title={t('nav.movies')}
          >
            <Film className="h-5 w-5" />
            <span className="text-xs">{t('nav.movies')}</span>
          </Link>
          <Link
            to="/scan"
            className="flex flex-col items-center justify-center gap-1 flex-1 h-full min-h-touch [&.active]:text-primary [&.active]:bg-accent/50"
            title={t('nav.scan')}
          >
            <ScanLine className="h-5 w-5" />
            <span className="text-xs">{t('nav.scan')}</span>
          </Link>
          <Link
            to="/series"
            className="flex flex-col items-center justify-center gap-1 flex-1 h-full min-h-touch [&.active]:text-primary [&.active]:bg-accent/50"
            title={t('nav.series')}
          >
            <Tv className="h-5 w-5" />
            <span className="text-xs">{t('nav.series')}</span>
          </Link>
          <button
            onClick={() => navigate({ to: '/me' })}
            className="flex flex-col items-center justify-center gap-1 flex-1 h-full min-h-touch"
            title={t('nav.profile')}
          >
            {user ? (
              <Avatar user={user} size="sm" />
            ) : (
              <User className="h-5 w-5" />
            )}
            <span className="text-xs">{t('nav.profile')}</span>
          </button>
        </div>
      </nav>

      {/* Dev tools (only in development) */}
      {import.meta.env.DEV && <TanStackRouterDevtools />}
    </div>
  )
}

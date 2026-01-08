import { createRootRouteWithContext, Outlet, Link, useNavigate, useLocation } from '@tanstack/react-router'
import { TanStackRouterDevtools } from '@tanstack/react-router-devtools'
import { Film, LogOut, User, Users, Settings, Search, Home } from 'lucide-react'
import { useAuth } from '@/hooks/useAuth'
import { useI18n } from '@/hooks/useI18n'
import { useWebSocketSync } from '@/hooks/useWebSocket'
import { Avatar } from '@/components/Avatar'
import { createContext, useContext, useState } from 'react'

// Context for search toolbar visibility (used by movies page)
interface SearchToolbarContextType {
  showToolbar: boolean
  setShowToolbar: (show: boolean) => void
  hasActiveFilter: boolean
  setHasActiveFilter: (active: boolean) => void
}

const SearchToolbarContext = createContext<SearchToolbarContextType>({
  showToolbar: false,
  setShowToolbar: () => {},
  hasActiveFilter: false,
  setHasActiveFilter: () => {},
})

export const useSearchToolbar = () => useContext(SearchToolbarContext)

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
  const location = useLocation()
  
  // Search toolbar state (shared with movies page)
  const [showToolbar, setShowToolbar] = useState(false)
  const [hasActiveFilter, setHasActiveFilter] = useState(false)

  // Set up WebSocket sync for real-time updates
  useWebSocketSync()

  const handleLogout = () => {
    logout()
    navigate({ to: '/login' })
  }

  // Check if we're on the movies page
  const isMoviesPage = location.pathname === '/movies' || location.pathname === '/movies/'

  if (!isAuthenticated) {
    return (
      <div className="min-h-screen bg-background">
        <Outlet />
      </div>
    )
  }

  return (
    <SearchToolbarContext.Provider value={{ showToolbar, setShowToolbar, hasActiveFilter, setHasActiveFilter }}>
    <div className="h-screen bg-background flex flex-col overflow-hidden relative">
      {/* Header - Hidden on mobile, shown on desktop */}
      <header className="hidden md:block border-b bg-card shrink-0 relative z-50" style={{ paddingTop: 'env(safe-area-inset-top, 0px)' }}>
        <div className="container flex h-14 items-center justify-between px-4">
          <Link to="/movies" className="flex items-center gap-2 font-semibold">
            <Film className="h-5 w-5" />
            <span>My Movies</span>
          </Link>

          <div className="flex items-center gap-1">
            {/* Search button for movies page */}
            {isMoviesPage && (
              <button
                onClick={() => setShowToolbar(!showToolbar)}
                className={`relative flex items-center justify-center h-10 w-10 rounded-md hover:bg-accent ${showToolbar ? 'bg-accent' : ''}`}
                title={t('movies.search')}
              >
                <Search className={`h-5 w-5 ${(showToolbar || hasActiveFilter) ? 'text-primary' : ''}`} />
                {hasActiveFilter && !showToolbar && (
                  <span className="absolute top-1 right-1 h-2 w-2 rounded-full bg-primary" />
                )}
              </button>
            )}
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

      {/* Mobile Header - Fixed at top */}
      <header className="md:hidden fixed top-0 left-0 right-0 border-b bg-card z-50 shrink-0 shadow-sm" style={{ 
        paddingTop: 'env(safe-area-inset-top, 0px)',
        backgroundColor: 'hsl(var(--card))' // Ensure solid background
      }}>
        <div className="flex h-14 items-center justify-between px-4">
          <div className="w-10" /> {/* Spacer for balance */}
          <Link to="/movies" className="flex items-center gap-2 font-semibold">
            <Film className="h-5 w-5" />
            <span className="text-base">My Movies</span>
          </Link>
          {/* Search icon - only on movies page */}
          {isMoviesPage ? (
            <button
              onClick={() => setShowToolbar(!showToolbar)}
              className="relative flex items-center justify-center h-10 w-10 rounded-md hover:bg-accent active:bg-accent/80"
            >
              <Search className={`h-5 w-5 ${(showToolbar || hasActiveFilter) ? 'text-primary' : ''}`} />
              {hasActiveFilter && !showToolbar && (
                <span className="absolute top-1 right-1 h-2 w-2 rounded-full bg-primary" />
              )}
            </button>
          ) : (
            <div className="w-10" /> // Spacer when not on movies page
          )}
        </div>
      </header>

      {/* Main content - Scrollable area */}
      <main 
        className="flex-1 container px-4 overflow-y-auto min-h-0 relative z-0 pt-safe-top pb-safe-bottom"
        style={{ 
          paddingTop: 'calc(4.5rem + env(safe-area-inset-top, 0px))', // 72px (h-14 + extra) + safe area for mobile fixed header
          paddingBottom: 'calc(5rem + env(safe-area-inset-bottom, 0px))', // 80px (h-16 + extra) + safe area for bottom navigation
          WebkitOverflowScrolling: 'touch' // Smooth scrolling on iOS
        }}
      >
        <Outlet />
      </main>

      {/* Bottom Navigation Bar - Mobile only */}
      <nav className="md:hidden fixed bottom-0 left-0 right-0 border-t bg-card z-50 shadow-lg pb-safe-bottom" style={{ 
        backgroundColor: 'hsl(var(--card))' // Ensure solid background
      }}>
        <div className="flex items-center justify-around h-16">
          <Link
            to="/movies"
            className="flex flex-col items-center justify-center gap-1 flex-1 h-full min-h-touch [&.active]:text-primary [&.active]:bg-accent/50"
            title={t('nav.home')}
          >
            <Home className="h-5 w-5" />
            <span className="text-xs">{t('nav.home')}</span>
          </Link>
          <Link
            to="/me"
            className="flex flex-col items-center justify-center gap-1 flex-1 h-full min-h-touch [&.active]:text-primary [&.active]:bg-accent/50"
            title={t('nav.profile')}
          >
            {user ? (
              <Avatar user={user} size="sm" />
            ) : (
              <User className="h-5 w-5" />
            )}
            <span className="text-xs">{t('nav.profile')}</span>
          </Link>
          {user?.role === 'admin' && (
            <>
              <Link
                to="/users"
                className="flex flex-col items-center justify-center gap-1 flex-1 h-full min-h-touch [&.active]:text-primary [&.active]:bg-accent/50"
                title={t('nav.users')}
              >
                <Users className="h-5 w-5" />
                <span className="text-xs">{t('nav.users')}</span>
              </Link>
              <Link
                to="/settings"
                className="flex flex-col items-center justify-center gap-1 flex-1 h-full min-h-touch [&.active]:text-primary [&.active]:bg-accent/50"
                title={t('nav.settings')}
              >
                <Settings className="h-5 w-5" />
                <span className="text-xs">{t('nav.settings')}</span>
              </Link>
            </>
          )}
          <button
            onClick={handleLogout}
            className="flex flex-col items-center justify-center gap-1 flex-1 h-full min-h-touch text-destructive hover:bg-destructive/10 active:bg-destructive/20"
            title={t('nav.logout')}
          >
            <LogOut className="h-5 w-5" />
            <span className="text-xs">{t('nav.logout')}</span>
          </button>
        </div>
      </nav>

      {/* Dev tools (only in development) */}
      {import.meta.env.DEV && <TanStackRouterDevtools />}
    </div>
    </SearchToolbarContext.Provider>
  )
}

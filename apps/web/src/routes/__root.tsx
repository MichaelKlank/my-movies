import { createRootRouteWithContext, Outlet, Link, useNavigate, useLocation } from '@tanstack/react-router'
import { TanStackRouterDevtools } from '@tanstack/react-router-devtools'
import { Film, LogOut, User, Users, Settings, Search, Home, HardDriveDownload } from 'lucide-react'
import { useAuth } from '@/hooks/useAuth'
import { useI18n } from '@/hooks/useI18n'
import { useWebSocketSync } from '@/hooks/useWebSocket'
import { Avatar } from '@/components/Avatar'
import { createContext, useContext, useState, useRef, useEffect } from 'react'

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

// Avatar Dropdown Menu Component
function AvatarMenu() {
  const { user, logout } = useAuth()
  const { t } = useI18n()
  const navigate = useNavigate()
  const [isOpen, setIsOpen] = useState(false)
  const menuRef = useRef<HTMLDivElement>(null)

  // Close menu when clicking outside
  useEffect(() => {
    const handleClickOutside = (e: MouseEvent) => {
      if (menuRef.current && !menuRef.current.contains(e.target as Node)) {
        setIsOpen(false)
      }
    }

    if (isOpen) {
      document.addEventListener('mousedown', handleClickOutside)
    }
    return () => document.removeEventListener('mousedown', handleClickOutside)
  }, [isOpen])

  const handleLogout = () => {
    setIsOpen(false)
    logout()
    navigate({ to: '/login' })
  }

  const handleNavigate = (to: string) => {
    setIsOpen(false)
    navigate({ to })
  }

  if (!user) return null

  return (
    <div ref={menuRef} className="relative">
      <button
        onClick={() => setIsOpen(!isOpen)}
        className="flex items-center justify-center h-10 w-10 rounded-full hover:bg-accent transition-colors"
      >
        <Avatar user={user} size="md" />
      </button>

      {isOpen && (
        <div className="absolute right-0 top-full mt-2 w-56 rounded-lg border bg-card shadow-lg py-1 z-50 animate-in fade-in slide-in-from-top-2 duration-150">
          {/* User info header */}
          <div className="px-4 py-3 border-b">
            <p className="font-medium truncate">{user.username}</p>
            <p className="text-sm text-muted-foreground truncate">{user.email}</p>
          </div>

          {/* Menu items */}
          <div className="py-1">
            <button
              onClick={() => handleNavigate('/me')}
              className="flex items-center gap-3 w-full px-4 py-2 text-sm hover:bg-accent transition-colors text-left"
            >
              <User className="h-4 w-4" />
              {t('nav.profile')}
            </button>

            {user.role === 'admin' && (
              <>
                <button
                  onClick={() => handleNavigate('/users')}
                  className="flex items-center gap-3 w-full px-4 py-2 text-sm hover:bg-accent transition-colors text-left"
                >
                  <Users className="h-4 w-4" />
                  {t('nav.users')}
                </button>
                <button
                  onClick={() => handleNavigate('/settings')}
                  className="flex items-center gap-3 w-full px-4 py-2 text-sm hover:bg-accent transition-colors text-left"
                >
                  <Settings className="h-4 w-4" />
                  {t('nav.settings')}
                </button>
              </>
            )}
          </div>

          {/* Logout */}
          <div className="border-t py-1">
            <button
              onClick={handleLogout}
              className="flex items-center gap-3 w-full px-4 py-2 text-sm text-destructive hover:bg-destructive/10 transition-colors text-left"
            >
              <LogOut className="h-4 w-4" />
              {t('nav.logout')}
            </button>
          </div>
        </div>
      )}
    </div>
  )
}

function RootLayout() {
  const { isAuthenticated } = useAuth()
  const { t } = useI18n()
  const location = useLocation()
  
  // Search toolbar state (shared with movies page)
  const [showToolbar, setShowToolbar] = useState(false)
  const [hasActiveFilter, setHasActiveFilter] = useState(false)

  // Set up WebSocket sync for real-time updates
  useWebSocketSync()

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
            {/* Home button */}
            <Link
              to="/movies"
              className="flex items-center justify-center h-10 w-10 rounded-md hover:bg-accent [&.active]:text-primary min-h-touch min-w-touch"
              title={t('nav.home')}
            >
              <Home className="h-5 w-5" />
            </Link>
            {/* Search button for movies page */}
            {isMoviesPage && (
              <button
                onClick={() => setShowToolbar(!showToolbar)}
                className="relative flex items-center justify-center h-10 w-10 rounded-md hover:bg-accent"
                title={t('movies.search')}
              >
                <Search className={`h-5 w-5 ${(showToolbar || hasActiveFilter) ? 'text-primary' : ''}`} />
                {hasActiveFilter && !showToolbar && (
                  <span className="absolute top-1 right-1 h-2 w-2 rounded-full bg-primary" />
                )}
              </button>
            )}
            <Link
              to="/backup"
              className="flex items-center justify-center h-10 w-10 rounded-md hover:bg-accent [&.active]:text-primary min-h-touch min-w-touch"
              title={t('nav.backup')}
            >
              <HardDriveDownload className="h-5 w-5" />
            </Link>
            {/* Avatar dropdown menu */}
            <AvatarMenu />
          </div>
        </div>
      </header>

      {/* Mobile Header - Fixed at top */}
      <header className="md:hidden fixed top-0 left-0 right-0 border-b bg-card z-50 shrink-0 shadow-sm" style={{ 
        paddingTop: 'env(safe-area-inset-top, 0px)',
        backgroundColor: 'hsl(var(--card))' // Ensure solid background
      }}>
        <div className="flex h-14 items-center justify-between px-4">
          {/* Search icon - only on movies page, otherwise spacer */}
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
            <div className="w-10" />
          )}
          <Link to="/movies" className="flex items-center gap-2 font-semibold">
            <Film className="h-5 w-5" />
            <span className="text-base">My Movies</span>
          </Link>
          {/* Avatar dropdown menu - always on right */}
          <AvatarMenu />
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

      {/* Bottom Navigation Bar - Mobile only (simplified - profile/settings/logout in avatar menu) */}
      <nav className="md:hidden fixed bottom-0 left-0 right-0 border-t bg-card z-50 shadow-lg pb-safe-bottom" style={{ 
        backgroundColor: 'hsl(var(--card))' // Ensure solid background
      }}>
        <div className="flex items-center justify-around h-16">
          <Link
            to="/movies"
            className="flex flex-col items-center justify-center gap-1 flex-1 h-full min-h-touch [&.active]:text-primary"
            title={t('nav.home')}
          >
            <Home className="h-5 w-5" />
            <span className="text-xs">{t('nav.home')}</span>
          </Link>
          <Link
            to="/backup"
            className="flex flex-col items-center justify-center gap-1 flex-1 h-full min-h-touch [&.active]:text-primary"
            title={t('nav.backup')}
          >
            <HardDriveDownload className="h-5 w-5" />
            <span className="text-xs">{t('nav.backup')}</span>
          </Link>
        </div>
      </nav>

      {/* Dev tools (only in development) */}
      {import.meta.env.DEV && <TanStackRouterDevtools />}
    </div>
    </SearchToolbarContext.Provider>
  )
}

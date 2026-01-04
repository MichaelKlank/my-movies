import { createRootRouteWithContext, Outlet, Link, useNavigate } from '@tanstack/react-router'
import { TanStackRouterDevtools } from '@tanstack/react-router-devtools'
import { Film, Tv, ScanLine, Upload, LogOut, User, Settings } from 'lucide-react'
import { useAuth } from '@/hooks/useAuth'
import { useWebSocketSync } from '@/hooks/useWebSocket'

interface RouterContext {
  auth: ReturnType<typeof useAuth>
}

export const Route = createRootRouteWithContext<RouterContext>()({
  component: RootLayout,
})

function RootLayout() {
  const { isAuthenticated, user, logout } = useAuth()
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
    <div className="min-h-screen bg-background">
      {/* Header */}
      <header className="border-b bg-card">
        <div className="container flex h-14 items-center justify-between px-4">
          <Link to="/" className="flex items-center gap-2 font-semibold">
            <Film className="h-5 w-5" />
            <span>My Movies</span>
          </Link>

          <nav className="flex items-center gap-1">
            <Link
              to="/movies"
              className="flex items-center gap-2 rounded-md px-3 py-2 text-sm hover:bg-accent [&.active]:bg-accent"
            >
              <Film className="h-4 w-4" />
              <span className="hidden sm:inline">Filme</span>
            </Link>
            <Link
              to="/series"
              className="flex items-center gap-2 rounded-md px-3 py-2 text-sm hover:bg-accent [&.active]:bg-accent"
            >
              <Tv className="h-4 w-4" />
              <span className="hidden sm:inline">Serien</span>
            </Link>
            <Link
              to="/scan"
              className="flex items-center gap-2 rounded-md px-3 py-2 text-sm hover:bg-accent [&.active]:bg-accent"
            >
              <ScanLine className="h-4 w-4" />
              <span className="hidden sm:inline">Scannen</span>
            </Link>
            <Link
              to="/import"
              className="flex items-center gap-2 rounded-md px-3 py-2 text-sm hover:bg-accent [&.active]:bg-accent"
            >
              <Upload className="h-4 w-4" />
              <span className="hidden sm:inline">Import</span>
            </Link>
          </nav>

          <div className="flex items-center gap-2">
            <span className="flex items-center gap-1 text-sm text-muted-foreground">
              <User className="h-4 w-4" />
              <span className="hidden sm:inline">{user?.username}</span>
            </span>
            {user?.role === 'admin' && (
              <Link
                to="/settings"
                className="rounded-md p-2 hover:bg-accent [&.active]:bg-accent"
                title="Einstellungen"
              >
                <Settings className="h-4 w-4" />
              </Link>
            )}
            <button
              onClick={handleLogout}
              className="rounded-md p-2 hover:bg-accent"
              title="Abmelden"
            >
              <LogOut className="h-4 w-4" />
            </button>
          </div>
        </div>
      </header>

      {/* Main content */}
      <main className="container px-4 py-6">
        <Outlet />
      </main>

      {/* Dev tools (only in development) */}
      {import.meta.env.DEV && <TanStackRouterDevtools />}
    </div>
  )
}

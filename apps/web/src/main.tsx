import React from 'react'
import ReactDOM from 'react-dom/client'
import { RouterProvider, createRouter } from '@tanstack/react-router'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'

import { routeTree } from './routeTree.gen'
import { AuthProvider, useAuth } from './hooks/useAuth'
import './styles/globals.css'

// Create router instance
const router = createRouter({
  routeTree,
  context: {
    auth: undefined!,
  },
})

// Register router for type safety
declare module '@tanstack/react-router' {
  interface Register {
    router: typeof router
  }
}

// Create query client
// staleTime: Infinity - Data is never considered stale automatically
// Cache invalidation happens via WebSocket events
// When invalidated, queries become stale and refetch on next observation
const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      staleTime: Infinity,
      gcTime: 1000 * 60 * 60, // Keep unused cache for 1 hour
      retry: 1,
      refetchOnWindowFocus: false,
      refetchOnReconnect: false,
      // refetchOnMount: true (default) - refetches if stale (after invalidation)
    },
  },
})

function InnerApp() {
  const auth = useAuth()
  
  // Wait for auth to finish loading before rendering router
  if (auth.isLoading) {
    return (
      <div className="flex min-h-screen items-center justify-center">
        <div className="text-muted-foreground">Laden...</div>
      </div>
    )
  }
  
  return <RouterProvider router={router} context={{ auth }} />
}

function App() {
  return (
    <QueryClientProvider client={queryClient}>
      <AuthProvider>
        <InnerApp />
      </AuthProvider>
    </QueryClientProvider>
  )
}

ReactDOM.createRoot(document.getElementById('root')!).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
)

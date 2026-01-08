import { createFileRoute, redirect } from '@tanstack/react-router'

export const Route = createFileRoute('/')({
  beforeLoad: ({ context }) => {
    if (!context.auth.isAuthenticated) {
      throw redirect({ to: '/login' })
    }
    // Redirect to movies page - no separate dashboard needed
    throw redirect({ to: '/movies' })
  },
  component: () => null, // Never rendered due to redirect
})

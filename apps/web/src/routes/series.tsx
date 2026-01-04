import { createFileRoute, redirect, Link } from '@tanstack/react-router'
import { useQuery } from '@tanstack/react-query'
import { Tv, Plus } from 'lucide-react'
import { api } from '@/lib/api'

export const Route = createFileRoute('/series')({
  beforeLoad: ({ context }) => {
    if (!context.auth.isAuthenticated) {
      throw redirect({ to: '/login' })
    }
  },
  component: SeriesPage,
})

function SeriesPage() {
  const { data: series = [], isLoading } = useQuery({
    queryKey: ['series'],
    queryFn: () => api.getSeries(),
  })

  return (
    <div className="space-y-6">
      <div className="flex flex-col gap-4 sm:flex-row sm:items-center sm:justify-between">
        <h1 className="text-2xl font-bold">Serien</h1>
        <Link
          to="/scan"
          className="flex items-center justify-center gap-2 rounded-md bg-primary px-4 py-2 text-sm text-primary-foreground hover:bg-primary/90"
        >
          <Plus className="h-4 w-4" />
          Serie hinzufügen
        </Link>
      </div>

      {isLoading ? (
        <div className="text-center py-12 text-muted-foreground">Laden...</div>
      ) : series.length === 0 ? (
        <div className="text-center py-12">
          <Tv className="mx-auto h-12 w-12 text-muted-foreground" />
          <p className="mt-4 text-muted-foreground">Keine Serien gefunden</p>
        </div>
      ) : (
        <div className="grid gap-4 grid-cols-2 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5">
          {series.map(s => (
            <div
              key={s.id}
              className="rounded-lg border bg-card overflow-hidden"
            >
              <div className="aspect-[2/3] bg-muted flex items-center justify-center">
                <Tv className="h-8 w-8 text-muted-foreground" />
              </div>
              <div className="p-3">
                <h3 className="font-medium text-sm truncate">{s.title}</h3>
                {s.network && (
                  <p className="text-xs text-muted-foreground">{s.network}</p>
                )}
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  )
}

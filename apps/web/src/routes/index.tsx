import { createFileRoute, redirect, Link } from '@tanstack/react-router'
import { useQuery } from '@tanstack/react-query'
import { Film, Tv, Plus } from 'lucide-react'
import { api } from '@/lib/api'

export const Route = createFileRoute('/')({
  beforeLoad: ({ context }) => {
    if (!context.auth.isAuthenticated) {
      throw redirect({ to: '/login' })
    }
  },
  component: Dashboard,
})

function Dashboard() {
  const { data: moviesData } = useQuery({
    queryKey: ['movies', 'recent'],
    queryFn: () => api.getMovies({ limit: '5', sort_by: 'created_at', sort_order: 'desc' }),
  })

  const { data: series = [] } = useQuery({
    queryKey: ['series'],
    queryFn: () => api.getSeries({ limit: '5' }),
  })

  const movies = moviesData?.items ?? []
  const totalMovies = moviesData?.total ?? 0

  return (
    <div className="space-y-8">
      <div className="flex items-center justify-between">
        <h1 className="text-2xl font-bold">Dashboard</h1>
        <Link
          to="/scan"
          className="flex items-center gap-2 rounded-md bg-primary px-4 py-2 text-sm text-primary-foreground hover:bg-primary/90"
        >
          <Plus className="h-4 w-4" />
          Hinzufügen
        </Link>
      </div>

      {/* Stats */}
      <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-4">
        <div className="rounded-lg border bg-card p-4">
          <div className="flex items-center gap-2 text-muted-foreground">
            <Film className="h-4 w-4" />
            <span className="text-sm">Filme</span>
          </div>
          <p className="mt-2 text-2xl font-bold">{totalMovies}</p>
        </div>
        <div className="rounded-lg border bg-card p-4">
          <div className="flex items-center gap-2 text-muted-foreground">
            <Tv className="h-4 w-4" />
            <span className="text-sm">Serien</span>
          </div>
          <p className="mt-2 text-2xl font-bold">{series.length}</p>
        </div>
      </div>

      {/* Recently added movies */}
      <section>
        <div className="mb-4 flex items-center justify-between">
          <h2 className="text-lg font-semibold">Zuletzt hinzugefügt</h2>
          <Link to="/movies" className="text-sm text-muted-foreground hover:underline">
            Alle anzeigen
          </Link>
        </div>
        <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-5">
          {movies.map(movie => (
            <Link
              key={movie.id}
              to="/movies/$movieId"
              params={{ movieId: movie.id }}
              className="group rounded-lg border bg-card overflow-hidden hover:border-primary"
            >
              <div className="aspect-[2/3] bg-muted flex items-center justify-center overflow-hidden">
                {movie.poster_path ? (
                  <img
                    src={`https://image.tmdb.org/t/p/w342${movie.poster_path}`}
                    alt={movie.title}
                    className="w-full h-full object-cover"
                  />
                ) : (
                  <Film className="h-8 w-8 text-muted-foreground" />
                )}
              </div>
              <div className="p-3">
                <h3 className="font-medium truncate group-hover:text-primary">{movie.title}</h3>
                {movie.production_year && (
                  <p className="text-sm text-muted-foreground">{movie.production_year}</p>
                )}
              </div>
            </Link>
          ))}
          {movies.length === 0 && (
            <p className="text-muted-foreground col-span-full text-center py-8">
              Noch keine Filme hinzugefügt.{' '}
              <Link to="/scan" className="underline hover:text-foreground">
                Jetzt scannen
              </Link>
            </p>
          )}
        </div>
      </section>
    </div>
  )
}

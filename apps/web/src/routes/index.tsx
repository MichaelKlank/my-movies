import { createFileRoute, redirect, Link } from '@tanstack/react-router'
import { useQuery } from '@tanstack/react-query'
import { Film, Tv, Plus } from 'lucide-react'
import { api } from '@/lib/api'
import { useI18n } from '@/hooks/useI18n'
import { PosterImage } from '@/components/PosterImage'

export const Route = createFileRoute('/')({
  beforeLoad: ({ context }) => {
    if (!context.auth.isAuthenticated) {
      throw redirect({ to: '/login' })
    }
  },
  component: Dashboard,
})

function Dashboard() {
  const { t } = useI18n()
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
    <div className="space-y-6 md:space-y-8">
      <div className="flex items-center justify-between">
        <h1 className="text-xl md:text-2xl font-bold">{t('dashboard.title')}</h1>
        <Link
          to="/scan"
          className="flex items-center gap-2 rounded-md bg-primary px-4 py-3 text-sm text-primary-foreground hover:bg-primary/90 active:bg-primary/80 min-h-touch"
        >
          <Plus className="h-4 w-4" />
          <span className="hidden sm:inline">{t('common.add')}</span>
        </Link>
      </div>

      {/* Stats */}
      <div className="grid gap-3 md:gap-4 grid-cols-2 lg:grid-cols-4">
        <div className="rounded-lg border bg-card p-4">
          <div className="flex items-center gap-2 text-muted-foreground">
            <Film className="h-4 w-4" />
            <span className="text-xs md:text-sm">{t('nav.movies')}</span>
          </div>
          <p className="mt-2 text-xl md:text-2xl font-bold">{totalMovies}</p>
        </div>
        <div className="rounded-lg border bg-card p-4">
          <div className="flex items-center gap-2 text-muted-foreground">
            <Tv className="h-4 w-4" />
            <span className="text-xs md:text-sm">{t('nav.series')}</span>
          </div>
          <p className="mt-2 text-xl md:text-2xl font-bold">{series.length}</p>
        </div>
      </div>

      {/* Recently added movies */}
      <section>
        <div className="mb-4 flex items-center justify-between">
          <h2 className="text-base md:text-lg font-semibold">{t('common.recentlyAdded')}</h2>
          <Link to="/movies" className="text-xs md:text-sm text-muted-foreground hover:underline active:text-foreground min-h-touch min-w-touch flex items-center">
            {t('common.showAll')}
          </Link>
        </div>
        <div className="grid gap-3 md:gap-4 grid-cols-2 sm:grid-cols-3 lg:grid-cols-5">
          {movies.map(movie => (
            <Link
              key={movie.id}
              to="/movies/$movieId"
              params={{ movieId: movie.id }}
              className="group rounded-lg border bg-card overflow-hidden hover:border-primary active:border-primary transition-colors"
            >
              <div className="aspect-[2/3] bg-muted flex items-center justify-center overflow-hidden">
                <PosterImage
                  posterPath={null}
                  movieId={movie.id}
                  size="w342"
                  alt={movie.title}
                  className="w-full h-full object-cover"
                  updatedAt={movie.updated_at}
                />
              </div>
              <div className="p-2 md:p-3">
                <h3 className="font-medium truncate text-sm md:text-base group-active:text-primary">{movie.title}</h3>
                {movie.production_year && (
                  <p className="text-xs md:text-sm text-muted-foreground">{movie.production_year}</p>
                )}
              </div>
            </Link>
          ))}
          {movies.length === 0 && (
            <p className="text-muted-foreground col-span-full text-center py-8 text-sm md:text-base">
              {t('common.noMoviesYet')}{' '}
              <Link to="/scan" className="underline hover:text-foreground active:text-primary">
                {t('common.scanNow')}
              </Link>
            </p>
          )}
        </div>
      </section>
    </div>
  )
}

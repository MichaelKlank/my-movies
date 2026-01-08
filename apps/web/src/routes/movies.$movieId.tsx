import { PosterImage } from '@/components/PosterImage'
import { useI18n } from '@/hooks/useI18n'
import { api } from '@/lib/api'
import { useMutation, useQuery } from '@tanstack/react-query'
import { createFileRoute, redirect, useNavigate } from '@tanstack/react-router'
import { ArrowLeft, Calendar, Check, ChevronDown, Clock, Disc, MapPin, RefreshCw, Star, Trash2 } from 'lucide-react'
import { useState } from 'react'

export const Route = createFileRoute('/movies/$movieId')({
  beforeLoad: ({ context }) => {
    if (!context.auth.isAuthenticated) {
      throw redirect({ to: '/login' })
    }
  },
  component: MovieDetailPage,
})

function MovieDetailPage() {
  const { movieId } = Route.useParams()
  const navigate = useNavigate()
  const [showRefreshMenu, setShowRefreshMenu] = useState(false)

  const { data: movie, isLoading } = useQuery({
    queryKey: ['movie', movieId],
    queryFn: () => api.getMovie(movieId),
  })

  const toggleWatchedMutation = useMutation({
    mutationFn: () => api.updateMovie(movieId, { watched: !movie?.watched }),
    // WebSocket event will handle cache invalidation
  })

  const deleteMutation = useMutation({
    mutationFn: () => api.deleteMovie(movieId),
    onSuccess: () => {
      // WebSocket event will handle cache invalidation
      navigate({ to: '/movies' })
    },
  })

  const refreshTmdbMutation = useMutation({
    mutationFn: (force: boolean) => api.refreshMovieTmdb(movieId, force),
    // WebSocket event will handle cache invalidation
  })

  const { t } = useI18n()

  if (isLoading) {
    return <div className="text-center py-12 text-muted-foreground">{t('common.loading')}</div>
  }

  if (!movie) {
    return <div className="text-center py-12 text-muted-foreground">{t('movies.notFound')}</div>
  }

  const discTypeLabel = (type?: string) => {
    switch (type?.toLowerCase()) {
      case 'bluray': return 'Blu-ray'
      case 'uhdbluray': return '4K UHD Blu-ray'
      case 'dvd': return 'DVD'
      case 'hddvd': return 'HD DVD'
      default: return type || ''
    }
  }

  return (
    <div className="space-y-4 md:space-y-6">
      <button
        onClick={() => navigate({ to: '/movies' })}
        className="flex items-center gap-2 text-sm text-muted-foreground hover:text-foreground active:text-foreground min-h-touch"
      >
        <ArrowLeft className="h-4 w-4" />
        <span className="hidden sm:inline">{t('common.back')} {t('nav.movies')}</span>
        <span className="sm:hidden">{t('common.back')}</span>
      </button>

      <div className="flex flex-col lg:grid lg:grid-cols-[350px_1fr] gap-6 md:gap-8">
        {/* Poster */}
        <div className="space-y-4">
          <div className="aspect-[2/3] max-w-xs mx-auto lg:max-w-none rounded-lg bg-muted flex items-center justify-center overflow-hidden shadow-lg">
            <PosterImage
              posterPath={null}
              movieId={movie.id}
              size="w500"
              alt={movie.title}
              className="w-full h-full object-cover"
              updatedAt={movie.updated_at}
            />
          </div>

          {/* Quick Info Cards */}
          <div className="grid grid-cols-2 gap-2">
            {movie.production_year && (
              <div className="rounded-lg bg-card border p-3 text-center">
                <Calendar className="h-4 w-4 mx-auto mb-1 text-muted-foreground" />
                <p className="text-sm font-medium">{movie.production_year}</p>
                <p className="text-xs text-muted-foreground">{t('movies.year')}</p>
              </div>
            )}
            {movie.running_time && (
              <div className="rounded-lg bg-card border p-3 text-center">
                <Clock className="h-4 w-4 mx-auto mb-1 text-muted-foreground" />
                <p className="text-sm font-medium">{movie.running_time} {t('movies.minutes')}</p>
                <p className="text-xs text-muted-foreground">{t('movies.runningTime')}</p>
              </div>
            )}
            {movie.personal_rating && (
              <div className="rounded-lg bg-card border p-3 text-center">
                <Star className="h-4 w-4 mx-auto mb-1 text-yellow-500" />
                <p className="text-sm font-medium">{movie.personal_rating}/10</p>
                <p className="text-xs text-muted-foreground">{t('movies.rating')}</p>
              </div>
            )}
            {movie.disc_type && (
              <div className="rounded-lg bg-card border p-3 text-center">
                <Disc className="h-4 w-4 mx-auto mb-1 text-muted-foreground" />
                <p className="text-sm font-medium">{discTypeLabel(movie.disc_type)}</p>
                <p className="text-xs text-muted-foreground">{t('movies.format')}</p>
              </div>
            )}
          </div>
        </div>

        {/* Details */}
        <div className="space-y-4 md:space-y-6">
          {/* Title Section */}
          <div>
            <div className="flex flex-col sm:flex-row sm:items-start sm:justify-between gap-3 sm:gap-4">
              <div className="flex-1 min-w-0">
                <h1 className="text-2xl md:text-3xl font-bold break-words">{movie.title}</h1>
                {movie.original_title && movie.original_title !== movie.title && (
                  <p className="text-base md:text-lg text-muted-foreground mt-1 break-words">{movie.original_title}</p>
                )}
              </div>
              {movie.watched && (
                <div className="flex items-center gap-1 rounded-full bg-green-500 px-3 py-2 text-sm text-white shrink-0">
                  <Check className="h-4 w-4" />
                  {t('movies.watched')}
                </div>
              )}
            </div>

            {movie.tagline && (
              <p className="mt-3 text-base md:text-lg italic text-muted-foreground break-words">„{movie.tagline}"</p>
            )}
          </div>

          {/* Genres */}
          {movie.genres && (
            <div className="flex flex-wrap gap-2">
              {movie.genres.split(',').map(genre => (
                <span
                  key={genre}
                  className="rounded-full bg-primary/10 text-primary px-3 py-1 text-sm"
                >
                  {genre.trim()}
                </span>
              ))}
            </div>
          )}

          {/* Description */}
          {movie.description && (
            <div className="space-y-2">
              <h2 className="text-base md:text-lg font-semibold">{t('movies.plot')}</h2>
              <p className="text-sm md:text-base text-muted-foreground leading-relaxed break-words">{movie.description}</p>
            </div>
          )}

          {/* Cast & Crew */}
          <div className="grid gap-4 md:gap-6 sm:grid-cols-2">
            {movie.director && (
              <div>
                <h2 className="text-xs md:text-sm font-semibold text-muted-foreground uppercase tracking-wide mb-2">{t('movies.director')}</h2>
                <p className="text-sm md:text-base break-words">{movie.director}</p>
              </div>
            )}

            {movie.actors && (
              <div className="sm:col-span-2">
                <h2 className="text-xs md:text-sm font-semibold text-muted-foreground uppercase tracking-wide mb-2">{t('movies.cast')}</h2>
                <p className="text-sm md:text-base leading-relaxed break-words">{movie.actors}</p>
              </div>
            )}
          </div>

          {/* Additional Info */}
          <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-3 pt-4 border-t">
            {movie.barcode && (
              <div>
                <h3 className="text-xs font-semibold text-muted-foreground uppercase">{t('movies.barcode')}</h3>
                <p className="text-sm font-mono break-all">{movie.barcode}</p>
              </div>
            )}
            {movie.imdb_id && (
              <div>
                <h3 className="text-xs font-semibold text-muted-foreground uppercase">IMDb</h3>
                <a
                  href={`https://www.imdb.com/title/${movie.imdb_id}`}
                  target="_blank"
                  rel="noopener noreferrer"
                  className="text-sm text-primary hover:underline active:text-primary/80 break-all"
                >
                  {movie.imdb_id}
                </a>
              </div>
            )}
            {movie.tmdb_id && (
              <div>
                <h3 className="text-xs font-semibold text-muted-foreground uppercase">TMDB</h3>
                <a
                  href={`https://www.themoviedb.org/movie/${movie.tmdb_id}`}
                  target="_blank"
                  rel="noopener noreferrer"
                  className="text-sm text-primary hover:underline active:text-primary/80 break-all"
                >
                  {movie.tmdb_id}
                </a>
              </div>
            )}
            {movie.location && (
              <div>
                <h3 className="text-xs font-semibold text-muted-foreground uppercase">{t('movies.location')}</h3>
                <p className="text-sm flex items-center gap-1 break-words">
                  <MapPin className="h-3 w-3 shrink-0" />
                  {movie.location}
                </p>
              </div>
            )}
            {movie.edition && (
              <div>
                <h3 className="text-xs font-semibold text-muted-foreground uppercase">{t('movies.edition')}</h3>
                <p className="text-sm break-words">{movie.edition}</p>
              </div>
            )}
          </div>

          {/* Notes */}
          {movie.notes && (
            <div className="rounded-lg bg-muted/50 p-4">
              <h2 className="text-sm font-semibold mb-2">{t('movies.notes')}</h2>
              <p className="text-sm text-muted-foreground break-words whitespace-pre-wrap">{movie.notes}</p>
            </div>
          )}

          {/* Actions */}
          <div className="flex flex-col sm:flex-row flex-wrap gap-3 pt-4 border-t">
            <button
              onClick={() => toggleWatchedMutation.mutate()}
              disabled={toggleWatchedMutation.isPending}
              className={`flex items-center justify-center gap-2 rounded-md px-4 py-3 text-sm font-medium transition-colors min-h-touch w-full sm:w-auto ${
                movie.watched
                  ? 'bg-green-500 text-white hover:bg-green-600 active:bg-green-700'
                  : 'bg-secondary hover:bg-secondary/80 active:bg-secondary/60'
              }`}
            >
              <Check className="h-4 w-4" />
              {movie.watched ? t('movies.watched') : t('movies.markAsWatched')}
            </button>
            
            <div className="relative w-full sm:w-auto">
              <button
                onClick={() => setShowRefreshMenu(!showRefreshMenu)}
                disabled={refreshTmdbMutation.isPending}
                className="flex items-center justify-center gap-2 rounded-md bg-secondary px-4 py-3 text-sm font-medium hover:bg-secondary/80 active:bg-secondary/60 min-h-touch w-full sm:w-auto"
              >
                <RefreshCw className={`h-4 w-4 ${refreshTmdbMutation.isPending ? 'animate-spin' : ''}`} />
                {t('movies.refreshTmdb')}
                <ChevronDown className={`h-4 w-4 transition-transform ${showRefreshMenu ? 'rotate-180' : ''}`} />
              </button>
              
              {/* Desktop dropdown */}
              {showRefreshMenu && (
                <div className="hidden sm:block absolute top-full left-0 mt-1 bg-card border rounded-md shadow-lg z-10 min-w-[200px]">
                  <button
                    onClick={() => {
                      refreshTmdbMutation.mutate(false)
                      setShowRefreshMenu(false)
                    }}
                    disabled={refreshTmdbMutation.isPending}
                    className="w-full text-left px-4 py-3 text-sm hover:bg-muted active:bg-muted/80 transition-colors min-h-touch"
                  >
                    {t('movies.refreshTmdbMissing')}
                  </button>
                  <button
                    onClick={() => {
                      refreshTmdbMutation.mutate(true)
                      setShowRefreshMenu(false)
                    }}
                    disabled={refreshTmdbMutation.isPending}
                    className="w-full text-left px-4 py-3 text-sm hover:bg-muted active:bg-muted/80 transition-colors border-t min-h-touch"
                  >
                    {t('movies.refreshTmdbAll')}
                  </button>
                </div>
              )}
            </div>

            {/* Mobile dialog for TMDB refresh */}
            {showRefreshMenu && (
              <div 
                className="sm:hidden fixed inset-0 bg-black/50 z-50"
                onClick={() => setShowRefreshMenu(false)}
              >
                <div 
                  className="fixed left-4 right-4 top-1/2 -translate-y-1/2 bg-card rounded-xl shadow-2xl overflow-hidden"
                  onClick={(e) => e.stopPropagation()}
                >
                  <div className="px-4 py-3 border-b bg-muted/50">
                    <h3 className="text-sm font-semibold text-center">
                      {t('movies.refreshTmdb')}
                    </h3>
                  </div>
                  <button
                    onClick={() => {
                      refreshTmdbMutation.mutate(false)
                      setShowRefreshMenu(false)
                    }}
                    disabled={refreshTmdbMutation.isPending}
                    className="w-full text-left px-4 py-4 text-sm hover:bg-muted active:bg-muted/80 transition-colors min-h-touch"
                  >
                    {t('movies.refreshTmdbMissing')}
                  </button>
                  <button
                    onClick={() => {
                      refreshTmdbMutation.mutate(true)
                      setShowRefreshMenu(false)
                    }}
                    disabled={refreshTmdbMutation.isPending}
                    className="w-full text-left px-4 py-4 text-sm hover:bg-muted active:bg-muted/80 transition-colors border-t min-h-touch"
                  >
                    {t('movies.refreshTmdbAll')}
                  </button>
                  <button
                    onClick={() => setShowRefreshMenu(false)}
                    className="w-full text-center px-4 py-4 text-sm font-medium text-destructive hover:bg-muted active:bg-muted/80 transition-colors border-t min-h-touch"
                  >
                    {t('common.cancel')}
                  </button>
                </div>
              </div>
            )}

            <button
              onClick={() => {
                if (confirm(t('movies.deleteConfirm'))) {
                  deleteMutation.mutate()
                }
              }}
              disabled={deleteMutation.isPending}
              className="flex items-center justify-center gap-2 rounded-md bg-destructive px-4 py-3 text-sm font-medium text-destructive-foreground hover:bg-destructive/90 active:bg-destructive/80 min-h-touch w-full sm:w-auto sm:ml-auto"
            >
              <Trash2 className="h-4 w-4" />
              {t('movies.delete')}
            </button>
          </div>
        </div>
      </div>
    </div>
  )
}

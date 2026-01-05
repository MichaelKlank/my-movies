import { createFileRoute, redirect, useNavigate } from '@tanstack/react-router'
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query'
import { Film, ArrowLeft, Check, Trash2, RefreshCw, Star, Clock, Calendar, MapPin, Disc } from 'lucide-react'
import { api } from '@/lib/api'
import { useI18n } from '@/hooks/useI18n'

// Helper to get poster URL - supports TMDB paths, full URLs, and local uploads
function getPosterUrl(posterPath: string | undefined | null, size: 'w92' | 'w342' | 'w500' = 'w342'): string | null {
  if (!posterPath) return null
  // If it starts with http, it's a full URL
  if (posterPath.startsWith('http')) return posterPath
  // If it starts with /uploads, it's a local file
  if (posterPath.startsWith('/uploads')) return posterPath
  // Otherwise it's a TMDB path
  return `https://image.tmdb.org/t/p/${size}${posterPath}`
}

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
  const queryClient = useQueryClient()

  const { data: movie, isLoading } = useQuery({
    queryKey: ['movie', movieId],
    queryFn: () => api.getMovie(movieId),
  })

  const toggleWatchedMutation = useMutation({
    mutationFn: () => api.updateMovie(movieId, { watched: !movie?.watched }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['movie', movieId] })
      queryClient.invalidateQueries({ queryKey: ['movies'] })
    },
  })

  const deleteMutation = useMutation({
    mutationFn: () => api.deleteMovie(movieId),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['movies'] })
      navigate({ to: '/movies' })
    },
  })

  const refreshTmdbMutation = useMutation({
    mutationFn: () => api.refreshMovieTmdb(movieId),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['movie', movieId] })
      queryClient.invalidateQueries({ queryKey: ['movies'] })
    },
  })

  const { t } = useI18n()

  if (isLoading) {
    return <div className="text-center py-12 text-muted-foreground">{t('common.loading')}</div>
  }

  if (!movie) {
    return <div className="text-center py-12 text-muted-foreground">{t('movies.notFound')}</div>
  }

  const discTypeLabel = (type?: string) => {
    switch (type) {
      case 'BluRay': return 'Blu-ray'
      case 'UhdBluRay': return '4K UHD Blu-ray'
      case 'Dvd': return 'DVD'
      default: return type
    }
  }

  return (
    <div className="space-y-6">
      <button
        onClick={() => navigate({ to: '/movies' })}
        className="flex items-center gap-2 text-sm text-muted-foreground hover:text-foreground"
      >
        <ArrowLeft className="h-4 w-4" />
        Zurück zur Übersicht
      </button>

      <div className="grid gap-8 lg:grid-cols-[350px_1fr]">
        {/* Poster */}
        <div className="space-y-4">
          <div className="aspect-[2/3] rounded-lg bg-muted flex items-center justify-center overflow-hidden shadow-lg">
            {getPosterUrl(movie.poster_path, 'w500') ? (
              <img
                src={getPosterUrl(movie.poster_path, 'w500')!}
                alt={movie.title}
                className="w-full h-full object-cover"
              />
            ) : (
              <Film className="h-20 w-20 text-muted-foreground" />
            )}
          </div>

          {/* Quick Info Cards */}
          <div className="grid grid-cols-2 gap-2">
            {movie.production_year && (
              <div className="rounded-lg bg-card border p-3 text-center">
                <Calendar className="h-4 w-4 mx-auto mb-1 text-muted-foreground" />
                <p className="text-sm font-medium">{movie.production_year}</p>
                <p className="text-xs text-muted-foreground">Jahr</p>
              </div>
            )}
            {movie.running_time && (
              <div className="rounded-lg bg-card border p-3 text-center">
                <Clock className="h-4 w-4 mx-auto mb-1 text-muted-foreground" />
                <p className="text-sm font-medium">{movie.running_time} min</p>
                <p className="text-xs text-muted-foreground">Laufzeit</p>
              </div>
            )}
            {movie.personal_rating && (
              <div className="rounded-lg bg-card border p-3 text-center">
                <Star className="h-4 w-4 mx-auto mb-1 text-yellow-500" />
                <p className="text-sm font-medium">{movie.personal_rating}/10</p>
                <p className="text-xs text-muted-foreground">Bewertung</p>
              </div>
            )}
            {movie.disc_type && (
              <div className="rounded-lg bg-card border p-3 text-center">
                <Disc className="h-4 w-4 mx-auto mb-1 text-muted-foreground" />
                <p className="text-sm font-medium">{discTypeLabel(movie.disc_type)}</p>
                <p className="text-xs text-muted-foreground">Format</p>
              </div>
            )}
          </div>
        </div>

        {/* Details */}
        <div className="space-y-6">
          {/* Title Section */}
          <div>
            <div className="flex items-start justify-between gap-4">
              <div>
                <h1 className="text-3xl font-bold">{movie.title}</h1>
                {movie.original_title && movie.original_title !== movie.title && (
                  <p className="text-lg text-muted-foreground mt-1">{movie.original_title}</p>
                )}
              </div>
              {movie.watched && (
                <div className="flex items-center gap-1 rounded-full bg-green-500 px-3 py-1 text-sm text-white">
                  <Check className="h-4 w-4" />
                  Gesehen
                </div>
              )}
            </div>

            {movie.tagline && (
              <p className="mt-3 text-lg italic text-muted-foreground">„{movie.tagline}"</p>
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
              <h2 className="text-lg font-semibold">Handlung</h2>
              <p className="text-muted-foreground leading-relaxed">{movie.description}</p>
            </div>
          )}

          {/* Cast & Crew */}
          <div className="grid gap-6 sm:grid-cols-2">
            {movie.director && (
              <div>
                <h2 className="text-sm font-semibold text-muted-foreground uppercase tracking-wide mb-2">Regie</h2>
                <p>{movie.director}</p>
              </div>
            )}

            {movie.actors && (
              <div className="sm:col-span-2">
                <h2 className="text-sm font-semibold text-muted-foreground uppercase tracking-wide mb-2">Darsteller</h2>
                <p className="leading-relaxed">{movie.actors}</p>
              </div>
            )}
          </div>

          {/* Additional Info */}
          <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-3 pt-4 border-t">
            {movie.barcode && (
              <div>
                <h3 className="text-xs font-semibold text-muted-foreground uppercase">Barcode</h3>
                <p className="text-sm font-mono">{movie.barcode}</p>
              </div>
            )}
            {movie.imdb_id && (
              <div>
                <h3 className="text-xs font-semibold text-muted-foreground uppercase">IMDb</h3>
                <a
                  href={`https://www.imdb.com/title/${movie.imdb_id}`}
                  target="_blank"
                  rel="noopener noreferrer"
                  className="text-sm text-primary hover:underline"
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
                  className="text-sm text-primary hover:underline"
                >
                  {movie.tmdb_id}
                </a>
              </div>
            )}
            {movie.location && (
              <div>
                <h3 className="text-xs font-semibold text-muted-foreground uppercase">Standort</h3>
                <p className="text-sm flex items-center gap-1">
                  <MapPin className="h-3 w-3" />
                  {movie.location}
                </p>
              </div>
            )}
            {movie.edition && (
              <div>
                <h3 className="text-xs font-semibold text-muted-foreground uppercase">Edition</h3>
                <p className="text-sm">{movie.edition}</p>
              </div>
            )}
          </div>

          {/* Notes */}
          {movie.notes && (
            <div className="rounded-lg bg-muted/50 p-4">
              <h2 className="text-sm font-semibold mb-2">Notizen</h2>
              <p className="text-sm text-muted-foreground">{movie.notes}</p>
            </div>
          )}

          {/* Actions */}
          <div className="flex flex-wrap gap-3 pt-4 border-t">
            <button
              onClick={() => toggleWatchedMutation.mutate()}
              disabled={toggleWatchedMutation.isPending}
              className={`flex items-center gap-2 rounded-md px-4 py-2 text-sm font-medium transition-colors ${
                movie.watched
                  ? 'bg-green-500 text-white hover:bg-green-600'
                  : 'bg-secondary hover:bg-secondary/80'
              }`}
            >
              <Check className="h-4 w-4" />
              {movie.watched ? 'Gesehen' : 'Als gesehen markieren'}
            </button>
            
            <button
              onClick={() => refreshTmdbMutation.mutate()}
              disabled={refreshTmdbMutation.isPending}
              className="flex items-center gap-2 rounded-md bg-secondary px-4 py-2 text-sm font-medium hover:bg-secondary/80"
            >
              <RefreshCw className={`h-4 w-4 ${refreshTmdbMutation.isPending ? 'animate-spin' : ''}`} />
              TMDB aktualisieren
            </button>

            <button
              onClick={() => {
                if (confirm('Film wirklich löschen?')) {
                  deleteMutation.mutate()
                }
              }}
              disabled={deleteMutation.isPending}
              className="flex items-center gap-2 rounded-md bg-destructive px-4 py-2 text-sm font-medium text-destructive-foreground hover:bg-destructive/90 ml-auto"
            >
              <Trash2 className="h-4 w-4" />
              Löschen
            </button>
          </div>
        </div>
      </div>
    </div>
  )
}

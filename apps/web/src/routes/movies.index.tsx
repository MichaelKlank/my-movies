import { createFileRoute, redirect, Link } from '@tanstack/react-router'
import { useQuery, useMutation } from '@tanstack/react-query'
import { useState, useEffect, useRef, useMemo } from 'react'
import { Film, Search, Plus, Check, X, Star, Trash2, RefreshCw, Eye, Bookmark, ImagePlus, Upload } from 'lucide-react'
import { api, MovieFilter, Movie } from '@/lib/api'
import { useI18n } from '@/hooks/useI18n'
import { PosterImage } from '@/components/PosterImage'

export const Route = createFileRoute('/movies/')({
  beforeLoad: ({ context }) => {
    if (!context.auth.isAuthenticated) {
      throw redirect({ to: '/login' })
    }
  },
  component: MoviesPage,
})

const ALPHABET = '#ABCDEFGHIJKLMNOPQRSTUVWXYZ'.split('')

function MoviesPage() {
  const { t } = useI18n()
  const [search, setSearch] = useState('')
  const [filter, setFilter] = useState<MovieFilter>({})
  const [selectedMovieId, setSelectedMovieId] = useState<string | null>(null)
  const [activeLetter, setActiveLetter] = useState<string | null>(null)
  const sectionRefs = useRef<Record<string, HTMLElement | null>>({})

  // Load ALL movies (high limit)
  const { data, isLoading } = useQuery({
    queryKey: ['movies', 'all', filter],
    queryFn: () => api.getMovies({ 
      ...filter, 
      limit: '10000',  // Load all
      sort_by: 'sort_title',
      sort_order: 'asc'
    }),
  })

  const movies = data?.items ?? []
  const total = data?.total ?? 0

  // Group movies by first letter
  const moviesByLetter = useMemo(() => {
    const grouped: Record<string, Movie[]> = {}
    
    for (const movie of movies) {
      const sortTitle = movie.sort_title || movie.title
      const firstChar = sortTitle.charAt(0).toUpperCase()
      const letter = /[A-Z]/.test(firstChar) ? firstChar : '#'
      
      if (!grouped[letter]) {
        grouped[letter] = []
      }
      grouped[letter].push(movie)
    }
    
    return grouped
  }, [movies])

  // Available letters (only those with movies)
  const availableLetters = useMemo(() => {
    return ALPHABET.filter(letter => moviesByLetter[letter]?.length > 0)
  }, [moviesByLetter])

  const handleSearch = (e: React.FormEvent) => {
    e.preventDefault()
    setFilter(prev => ({ ...prev, search: search || undefined }))
  }

  const handleFilterChange = (newFilter: Partial<MovieFilter>) => {
    setFilter(prev => ({ ...prev, ...newFilter }))
  }

  const scrollToLetter = (letter: string) => {
    const element = sectionRefs.current[letter]
    if (element) {
      element.scrollIntoView({ behavior: 'smooth', block: 'start' })
      setActiveLetter(letter)
    }
  }

  // Track active letter on scroll
  useEffect(() => {
    const handleScroll = () => {
      for (const letter of availableLetters) {
        const element = sectionRefs.current[letter]
        if (element) {
          const rect = element.getBoundingClientRect()
          if (rect.top <= 150 && rect.bottom > 150) {
            setActiveLetter(letter)
            break
          }
        }
      }
    }

    window.addEventListener('scroll', handleScroll)
    return () => window.removeEventListener('scroll', handleScroll)
  }, [availableLetters])

  // Close modal on Escape key
  useEffect(() => {
    const handleEscape = (e: KeyboardEvent) => {
      if (e.key === 'Escape') setSelectedMovieId(null)
    }
    window.addEventListener('keydown', handleEscape)
    return () => window.removeEventListener('keydown', handleEscape)
  }, [])

  // Prevent body scroll when modal is open
  useEffect(() => {
    if (selectedMovieId) {
      document.body.style.overflow = 'hidden'
    } else {
      document.body.style.overflow = ''
    }
    return () => { document.body.style.overflow = '' }
  }, [selectedMovieId])

  return (
    <div className="relative">
      {/* Alphabet Navigation - Fixed on right side */}
      {!search && availableLetters.length > 0 && (
        <nav className="fixed right-2 top-1/2 -translate-y-1/2 z-40 flex flex-col gap-0.5 bg-background/80 backdrop-blur rounded-full py-2 px-1 shadow-lg border">
          {ALPHABET.map(letter => {
            const hasMovies = moviesByLetter[letter]?.length > 0
            return (
              <button
                key={letter}
                onClick={() => hasMovies && scrollToLetter(letter)}
                disabled={!hasMovies}
                className={`w-6 h-6 text-xs font-medium rounded-full transition-all ${
                  activeLetter === letter
                    ? 'bg-primary text-primary-foreground'
                    : hasMovies
                      ? 'hover:bg-muted text-foreground'
                      : 'text-muted-foreground/30 cursor-default'
                }`}
              >
                {letter}
              </button>
            )
          })}
        </nav>
      )}

      <div className="space-y-6 pr-10">
        <div className="flex flex-col gap-4 sm:flex-row sm:items-center sm:justify-between">
          <div>
            <h1 className="text-2xl font-bold">{t('movies.title')}</h1>
            {total > 0 && (
              <p className="text-sm text-muted-foreground">
                {total} {t('movies.inCollection')}
              </p>
            )}
          </div>
          <Link
            to="/scan"
            className="flex items-center justify-center gap-2 rounded-md bg-primary px-4 py-2 text-sm text-primary-foreground hover:bg-primary/90"
          >
            <Plus className="h-4 w-4" />
            {t('movies.add')}
          </Link>
        </div>

        {/* Search & Filter */}
        <form onSubmit={handleSearch} className="flex gap-2">
          <div className="relative flex-1">
            <Search className="absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground" />
            <input
              type="search"
              placeholder={t('movies.searchPlaceholder')}
              value={search}
              onChange={e => setSearch(e.target.value)}
              className="w-full rounded-md border bg-background pl-9 pr-3 py-2 text-sm focus:border-primary focus:outline-none focus:ring-1 focus:ring-primary"
            />
          </div>
          <button
            type="submit"
            className="rounded-md bg-secondary px-4 py-2 text-sm hover:bg-secondary/80"
          >
            {t('movies.search')}
          </button>
        </form>

        {/* Filter pills */}
        <div className="flex flex-wrap gap-2">
          <button
            onClick={() => handleFilterChange({ watched: filter.watched === 'true' ? undefined : 'true' })}
            className={`flex items-center gap-1 rounded-full px-3 py-1 text-xs ${
              filter.watched === 'true'
                ? 'bg-primary text-primary-foreground'
                : 'bg-secondary hover:bg-secondary/80'
            }`}
          >
            <Check className="h-3 w-3" />
            {t('movies.watched')}
          </button>
          <button
            onClick={() => handleFilterChange({ watched: filter.watched === 'false' ? undefined : 'false' })}
            className={`flex items-center gap-1 rounded-full px-3 py-1 text-xs ${
              filter.watched === 'false'
                ? 'bg-primary text-primary-foreground'
                : 'bg-secondary hover:bg-secondary/80'
            }`}
          >
            <X className="h-3 w-3" />
            {t('movies.notWatched')}
          </button>
          <select
            value={filter.disc_type || ''}
            onChange={e => handleFilterChange({ disc_type: e.target.value || undefined })}
            className="rounded-full border bg-background px-3 py-1 text-xs"
          >
            <option value="">{t('movies.allFormats')}</option>
            <option value="Blu-ray">Blu-ray</option>
            <option value="DVD">DVD</option>
            <option value="uhdbluray">4K UHD</option>
          </select>
        </div>

        {/* Movies grouped by letter */}
        {isLoading ? (
          <div className="text-center py-12 text-muted-foreground">{t('common.loading')}</div>
        ) : movies.length === 0 ? (
          <div className="text-center py-12">
            <Film className="mx-auto h-12 w-12 text-muted-foreground" />
            <p className="mt-4 text-muted-foreground">{t('movies.notFound')}</p>
            <Link
              to="/scan"
              className="mt-4 inline-flex items-center gap-2 rounded-md bg-primary px-4 py-2 text-sm text-primary-foreground hover:bg-primary/90"
            >
              <Plus className="h-4 w-4" />
              {t('movies.addFirstMovie')}
            </Link>
          </div>
        ) : search ? (
          // Flat grid when searching
          <div className="grid gap-4 grid-cols-2 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5 xl:grid-cols-6">
            {movies.map(movie => (
              <MovieCard key={movie.id} movie={movie} onClick={() => setSelectedMovieId(movie.id)} />
            ))}
          </div>
        ) : (
          // Grouped by letter when not searching
          <div className="space-y-8">
            {availableLetters.map(letter => (
              <section
                key={letter}
                ref={el => { sectionRefs.current[letter] = el }}
                className="scroll-mt-24"
              >
                <h2 className="text-lg font-bold mb-4 sticky top-16 bg-background/95 backdrop-blur py-2 z-10 border-b">
                  {letter}
                  <span className="text-sm font-normal text-muted-foreground ml-2">
                    ({moviesByLetter[letter]?.length})
                  </span>
                </h2>
                <div className="grid gap-4 grid-cols-2 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5 xl:grid-cols-6">
                  {moviesByLetter[letter]?.map(movie => (
                    <MovieCard key={movie.id} movie={movie} onClick={() => setSelectedMovieId(movie.id)} />
                  ))}
                </div>
              </section>
            ))}
          </div>
        )}
      </div>

      {/* Movie Detail Modal */}
      {selectedMovieId && (
        <MovieDetailModal
          movieId={selectedMovieId}
          onClose={() => setSelectedMovieId(null)}
        />
      )}
    </div>
  )
}

function MovieCard({ movie, onClick }: { movie: Movie; onClick: () => void }) {
  return (
    <button
      onClick={onClick}
      className="group rounded-lg border bg-card overflow-hidden hover:border-primary text-left transition-all hover:shadow-lg"
    >
      <div className="aspect-[2/3] bg-muted flex items-center justify-center relative overflow-hidden">
        {movie.poster_path ? (
          <PosterImage
            posterPath={movie.poster_path}
            movieId={movie.id}
            size="w342"
            alt={movie.title}
            className="w-full h-full object-cover transition-transform group-hover:scale-105"
          />
        ) : (
          <Film className="h-8 w-8 text-muted-foreground" />
        )}
        {movie.watched && (
          <div className="absolute top-2 right-2 rounded-full bg-green-500 p-1">
            <Check className="h-3 w-3 text-white" />
          </div>
        )}
      </div>
      <div className="p-3">
        <h3 className="font-medium text-sm truncate group-hover:text-primary">
          {movie.title}
        </h3>
        <div className="flex items-center gap-2 text-xs text-muted-foreground">
          {movie.production_year && <span>{movie.production_year}</span>}
          {movie.disc_type && (
            <span className="rounded bg-secondary px-1">
              {(() => {
                const type = movie.disc_type.toLowerCase()
                if (type === 'bluray') return 'BD'
                if (type === 'uhdbluray') return '4K'
                if (type === 'dvd') return 'DVD'
                if (type === 'hddvd') return 'HD DVD'
                return movie.disc_type
              })()}
            </span>
          )}
        </div>
      </div>
    </button>
  )
}

function MovieDetailModal({ movieId, onClose }: { movieId: string; onClose: () => void }) {
  const { t } = useI18n()
  const [showPosterDialog, setShowPosterDialog] = useState(false)

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
      onClose()
    },
  })

  const refreshTmdbMutation = useMutation({
    mutationFn: () => api.refreshMovieTmdb(movieId),
    // WebSocket event will handle cache invalidation
  })

  const discTypeLabel = (type?: string) => {
    switch (type?.toLowerCase()) {
      case 'bluray': return 'Blu-ray'
      case 'uhdbluray': return '4K UHD'
      case 'dvd': return 'DVD'
      case 'hddvd': return 'HD DVD'
      default: return type || ''
    }
  }

  // posterUrl is now handled by PosterImage component

  // Star rating component
  const StarRating = ({ rating }: { rating?: number }) => {
    const stars = rating ? Math.round(rating / 2) : 0 // Convert 10-scale to 5-scale
    return (
      <div className="flex gap-0.5">
        {[1, 2, 3, 4, 5].map(i => (
          <Star
            key={i}
            className={`h-5 w-5 ${i <= stars ? 'text-yellow-400 fill-yellow-400' : 'text-gray-300'}`}
          />
        ))}
      </div>
    )
  }

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center p-4">
      {/* Backdrop */}
      <div
        className="absolute inset-0 bg-black/70"
        onClick={onClose}
      />

      {/* Modal */}
      <div className="relative bg-card rounded-lg shadow-2xl w-full max-w-2xl max-h-[90vh] overflow-hidden animate-in fade-in zoom-in-95 duration-200">
        {/* Header */}
        <div className="flex items-center justify-between px-4 py-3 border-b bg-muted/30">
          <h2 className="font-semibold truncate pr-4">
            {isLoading ? t('common.loading') : movie?.title || t('movies.movie')}
          </h2>
          <button
            onClick={onClose}
            className="rounded-md p-1 hover:bg-muted transition-colors"
          >
            <X className="h-5 w-5" />
          </button>
        </div>

        {isLoading ? (
          <div className="flex items-center justify-center h-80">
            <div className="text-muted-foreground">{t('common.loading')}</div>
          </div>
        ) : !movie ? (
          <div className="flex items-center justify-center h-80">
            <div className="text-muted-foreground">{t('movies.notFound')}</div>
          </div>
        ) : (
          <>
            {/* Content */}
            <div className="flex p-4 gap-4 max-h-[calc(90vh-120px)] overflow-auto">
              {/* Poster Section */}
              <div className="w-40 shrink-0 space-y-3">
                <div className="aspect-[2/3] rounded-lg overflow-hidden bg-muted shadow-lg relative group/poster">
                  {movie?.poster_path ? (
                    <PosterImage
                      posterPath={movie.poster_path}
                      movieId={movie?.id}
                      size="w500"
                      alt={movie.title}
                      className="w-full h-full object-cover"
                    />
                  ) : (
                    <div className="w-full h-full flex items-center justify-center">
                      <Film className="h-12 w-12 text-muted-foreground" />
                    </div>
                  )}
                  {/* Edit poster overlay */}
                  <button
                    onClick={() => setShowPosterDialog(true)}
                    className="absolute inset-0 bg-black/60 opacity-0 group-hover/poster:opacity-100 transition-opacity flex items-center justify-center"
                    title={t('movies.changePoster')}
                  >
                    <ImagePlus className="h-8 w-8 text-white" />
                  </button>
                </div>
                
                {/* Rating */}
                <div className="flex justify-center">
                  <StarRating rating={movie.personal_rating} />
                </div>

                {/* Quick Actions */}
                <div className="flex justify-center gap-4">
                  <button
                    onClick={() => toggleWatchedMutation.mutate()}
                    disabled={toggleWatchedMutation.isPending}
                    className={`p-2 rounded-full transition-colors ${
                      movie.watched
                        ? 'text-green-500 bg-green-500/10'
                        : 'text-muted-foreground hover:text-foreground hover:bg-muted'
                    }`}
                    title={movie.watched ? t('movies.watched') : t('movies.markAsWatched')}
                  >
                    <Eye className="h-5 w-5" />
                  </button>
                  <button
                    className="p-2 rounded-full text-muted-foreground hover:text-foreground hover:bg-muted transition-colors"
                    title={t('movies.bookmark')}
                  >
                    <Bookmark className="h-5 w-5" />
                  </button>
                </div>
              </div>

              {/* Poster Upload Dialog */}
              {showPosterDialog && (
                <PosterUploadDialog
                  movieId={movieId}
                  onClose={() => setShowPosterDialog(false)}
                  onSuccess={() => {
                    // WebSocket event will handle cache invalidation
                    setShowPosterDialog(false)
                  }}
                />
              )}

              {/* Details Section */}
              <div className="flex-1 min-w-0">
                {/* Title & Year */}
                <h3 className="text-xl font-bold">{movie.title}</h3>
                {movie.original_title && movie.original_title !== movie.title && (
                  <p className="text-sm text-muted-foreground">{movie.original_title}</p>
                )}
                
                <p className="text-muted-foreground mt-1">{movie.production_year}</p>
                
                {/* Format Badge */}
                {movie.disc_type && (
                  <span className="inline-block mt-2 px-3 py-1 rounded-full text-xs font-medium bg-purple-600 text-white">
                    {discTypeLabel(movie.disc_type)}
                  </span>
                )}

                {/* Details Section */}
                <div className="mt-4">
                  <h4 className="text-xs font-semibold text-muted-foreground uppercase tracking-wider mb-2">
                    {t('movies.details')}
                  </h4>
                  <div className="space-y-2 text-sm">
                    {movie.barcode && (
                      <div className="flex gap-3">
                        <span className="text-muted-foreground w-32 shrink-0"># {t('movies.barcode')}</span>
                        <span className="font-mono">{movie.barcode}</span>
                      </div>
                    )}
                    {movie.tmdb_id && (
                      <div className="flex gap-3">
                        <span className="text-muted-foreground w-32 shrink-0">TMDB ID</span>
                        <a 
                          href={`https://www.themoviedb.org/movie/${movie.tmdb_id}`}
                          target="_blank"
                          rel="noopener noreferrer"
                          className="text-primary hover:underline"
                        >
                          {movie.tmdb_id}
                        </a>
                      </div>
                    )}
                    {movie.imdb_id && (
                      <div className="flex gap-3">
                        <span className="text-muted-foreground w-32 shrink-0">IMDb ID</span>
                        <a 
                          href={`https://www.imdb.com/title/${movie.imdb_id}`}
                          target="_blank"
                          rel="noopener noreferrer"
                          className="text-primary hover:underline"
                        >
                          {movie.imdb_id}
                        </a>
                      </div>
                    )}
                    {movie.running_time && (
                      <div className="flex gap-3">
                        <span className="text-muted-foreground w-32 shrink-0">{t('movies.runningTime')}</span>
                        <span>{movie.running_time} {t('movies.minutes')}</span>
                      </div>
                    )}
                    {movie.director && (
                      <div className="flex gap-3">
                        <span className="text-muted-foreground w-32 shrink-0">{t('movies.director')}</span>
                        <span>{movie.director}</span>
                      </div>
                    )}
                    {movie.location && (
                      <div className="flex gap-3">
                        <span className="text-muted-foreground w-32 shrink-0">{t('movies.location')}</span>
                        <span>{movie.location}</span>
                      </div>
                    )}
                  </div>
                </div>

                {/* Genres */}
                {movie.genres && (
                  <div className="mt-4">
                    <h4 className="text-xs font-semibold text-muted-foreground uppercase tracking-wider mb-2">
                      {t('movies.genres')}
                    </h4>
                    <div className="flex flex-wrap gap-1">
                      {movie.genres.split(',').map(genre => (
                        <span
                          key={genre}
                          className="px-2 py-0.5 rounded-full text-xs bg-muted"
                        >
                          {genre.trim()}
                        </span>
                      ))}
                    </div>
                  </div>
                )}

                {/* Actors */}
                {movie.actors && (
                  <div className="mt-4">
                    <h4 className="text-xs font-semibold text-muted-foreground uppercase tracking-wider mb-2">
                      Darsteller
                    </h4>
                    <p className="text-sm">{movie.actors}</p>
                  </div>
                )}

                {/* Description */}
                {movie.description && (
                  <div className="mt-4">
                    <h4 className="text-xs font-semibold text-muted-foreground uppercase tracking-wider mb-2">
                      Beschreibung
                    </h4>
                    <p className="text-sm leading-relaxed">{movie.description}</p>
                  </div>
                )}

                {/* Notes */}
                {movie.notes && (
                  <div className="mt-4">
                    <h4 className="text-xs font-semibold text-muted-foreground uppercase tracking-wider mb-2">
                      Notizen
                    </h4>
                    <p className="text-sm text-muted-foreground">{movie.notes}</p>
                  </div>
                )}
              </div>
            </div>

            {/* Footer Actions */}
            <div className="flex items-center justify-end gap-2 px-4 py-3 border-t bg-muted/30">
              <button
                onClick={() => {
                  if (confirm('Film wirklich löschen?')) {
                    deleteMutation.mutate()
                  }
                }}
                disabled={deleteMutation.isPending}
                className="flex items-center gap-2 px-3 py-2 text-sm text-destructive hover:bg-destructive/10 rounded-md transition-colors"
              >
                <Trash2 className="h-4 w-4" />
                Löschen
              </button>
              
              <button
                onClick={() => refreshTmdbMutation.mutate()}
                disabled={refreshTmdbMutation.isPending}
                className="flex items-center gap-2 px-3 py-2 text-sm text-primary hover:bg-primary/10 rounded-md transition-colors"
              >
                <RefreshCw className={`h-4 w-4 ${refreshTmdbMutation.isPending ? 'animate-spin' : ''}`} />
                Aktualisieren
              </button>

              <Link
                to="/movies/$movieId"
                params={{ movieId }}
                onClick={onClose}
                className="flex items-center gap-2 px-4 py-2 text-sm bg-primary text-primary-foreground rounded-md hover:bg-primary/90 transition-colors"
              >
                Mehr Details
              </Link>
            </div>
          </>
        )}
      </div>
    </div>
  )
}

// Poster Upload Dialog Component
function PosterUploadDialog({ 
  movieId, 
  onClose, 
  onSuccess 
}: { 
  movieId: string
  onClose: () => void
  onSuccess: () => void 
}) {
  const { t } = useI18n()
  const [customPosterUrl, setCustomPosterUrl] = useState('')
  const [selectedFile, setSelectedFile] = useState<File | null>(null)
  const [previewUrl, setPreviewUrl] = useState<string | null>(null)
  const [isUploading, setIsUploading] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const fileInputRef = useRef<HTMLInputElement>(null)

  // Create preview URL when file is selected
  useEffect(() => {
    if (selectedFile) {
      const url = URL.createObjectURL(selectedFile)
      setPreviewUrl(url)
      return () => URL.revokeObjectURL(url)
    }
    setPreviewUrl(null)
  }, [selectedFile])

  const handleFileSelect = (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0]
    if (file) {
      // Validate file type
      if (!file.type.startsWith('image/')) {
        setError('Bitte wähle eine Bilddatei aus')
        return
      }
      // Validate file size (max 10MB)
      if (file.size > 10 * 1024 * 1024) {
        setError('Die Datei ist zu groß (max. 10MB)')
        return
      }
      setSelectedFile(file)
      setCustomPosterUrl('')
      setError(null)
    }
  }

  const handleUrlChange = (url: string) => {
    setCustomPosterUrl(url)
    setSelectedFile(null)
    setError(null)
  }

  const handleSubmit = async () => {
    setIsUploading(true)
    setError(null)

    try {
      if (selectedFile) {
        // Upload file
        await api.uploadMoviePoster(movieId, selectedFile)
        onSuccess()
        return
      }
      if (customPosterUrl.trim()) {
        // Set URL
        await api.updateMovie(movieId, { poster_path: customPosterUrl.trim() })
        onSuccess()
        return
      }
      setError(t('poster.pleaseSelect'))
      setIsUploading(false)
    } catch (err) {
      setError(err instanceof Error ? err.message : t('settings.unknownError'))
    } finally {
      setIsUploading(false)
    }
  }

  const displayPreview = previewUrl || (customPosterUrl ? customPosterUrl : null)

  return (
    <div className="fixed inset-0 z-[60] flex items-center justify-center p-4">
      <div className="absolute inset-0 bg-black/50" onClick={onClose} />
      <div className="relative bg-card rounded-lg shadow-xl w-full max-w-md p-4 space-y-4">
        <h3 className="font-semibold">{t('poster.setPoster')}</h3>
        
        {/* File Upload Section */}
        <div className="space-y-2">
          <label className="text-sm font-medium">{t('poster.uploadFile')}</label>
          <input
            ref={fileInputRef}
            type="file"
            accept="image/*"
            onChange={handleFileSelect}
            className="hidden"
          />
          <button
            onClick={() => fileInputRef.current?.click()}
            className="w-full flex items-center justify-center gap-2 rounded-md border-2 border-dashed bg-background px-4 py-6 text-sm hover:border-primary hover:bg-accent transition-colors"
          >
            <Upload className="h-5 w-5" />
            {selectedFile ? selectedFile.name : t('poster.selectImage')}
          </button>
        </div>

        <div className="flex items-center gap-2 text-sm text-muted-foreground">
          <div className="h-px flex-1 bg-border" />
          <span>{t('poster.or')}</span>
          <div className="h-px flex-1 bg-border" />
        </div>

        {/* URL Input Section */}
        <div className="space-y-2">
          <label className="text-sm font-medium">{t('poster.enterUrl')}</label>
          <input
            type="url"
            value={customPosterUrl}
            onChange={(e) => handleUrlChange(e.target.value)}
            placeholder="https://image.tmdb.org/t/p/w500/..."
            className="w-full rounded-md border bg-background px-3 py-2 text-sm focus:border-primary focus:outline-none focus:ring-1 focus:ring-primary"
          />
          <p className="text-xs text-muted-foreground">
            {t('poster.tip')}{' '}
            <a href="https://www.themoviedb.org" target="_blank" rel="noopener noreferrer" className="text-primary hover:underline">
              themoviedb.org
            </a>
            {' '}{t('poster.tipCopy')}
          </p>
        </div>

        {/* Preview */}
        {displayPreview && (
          <div className="flex justify-center">
            <div className="w-24 aspect-[2/3] rounded overflow-hidden bg-muted shadow">
              <img
                src={displayPreview}
                alt={t('poster.preview')}
                className="w-full h-full object-cover"
                onError={(e) => {
                  (e.target as HTMLImageElement).style.display = 'none'
                }}
              />
            </div>
          </div>
        )}

        {/* Error Message */}
        {error && (
          <div className="rounded-md bg-destructive/10 p-2 text-sm text-destructive">
            {error}
          </div>
        )}

        {/* Actions */}
        <div className="flex justify-end gap-2">
          <button
            onClick={onClose}
            className="px-3 py-2 text-sm rounded-md hover:bg-muted"
          >
            {t('common.cancel')}
          </button>
          <button
            onClick={handleSubmit}
            disabled={(!customPosterUrl.trim() && !selectedFile) || isUploading}
            className="px-3 py-2 text-sm rounded-md bg-primary text-primary-foreground hover:bg-primary/90 disabled:opacity-50"
          >
            {isUploading ? t('settings.saving') : t('common.save')}
          </button>
        </div>
      </div>
    </div>
  )
}

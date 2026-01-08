import { createFileRoute, redirect, Link } from '@tanstack/react-router'
import { useQuery } from '@tanstack/react-query'
import { useState, useEffect, useRef, useMemo } from 'react'
import { Film, Search, Plus, Check, X } from 'lucide-react'
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

  const isManualScrolling = useRef(false)

  const scrollToLetter = (letter: string) => {
    const element = sectionRefs.current[letter]
    if (element) {
      // Disable observer updates during manual scroll
      isManualScrolling.current = true
      setActiveLetter(letter)
      element.scrollIntoView({ behavior: 'smooth', block: 'start' })
      
      // Re-enable after scroll animation completes
      setTimeout(() => {
        isManualScrolling.current = false
      }, 800)
    }
  }

  // Track active letter using IntersectionObserver
  useEffect(() => {
    const observer = new IntersectionObserver(
      (entries) => {
        // Skip if manually scrolling
        if (isManualScrolling.current) return
        
        // Find the entry that's intersecting and closest to the top
        const visibleEntries = entries.filter(e => e.isIntersecting)
        if (visibleEntries.length > 0) {
          // Sort by position, pick the one closest to top
          visibleEntries.sort((a, b) => a.boundingClientRect.top - b.boundingClientRect.top)
          const topEntry = visibleEntries[0]
          const letter = topEntry.target.getAttribute('data-letter')
          if (letter) {
            setActiveLetter(letter)
          }
        }
      },
      {
        rootMargin: '-80px 0px -60% 0px', // Top offset for header, bottom cuts off lower part
        threshold: 0
      }
    )

    // Observe all section elements
    availableLetters.forEach(letter => {
      const element = sectionRefs.current[letter]
      if (element) {
        element.setAttribute('data-letter', letter)
        observer.observe(element)
      }
    })

    return () => observer.disconnect()
  }, [availableLetters])

  return (
    <div className="relative">
      {/* Alphabet Navigation - Slim pill overlay */}
      {!search && availableLetters.length > 0 && (
        <nav className="hidden md:flex fixed right-1 top-20 bottom-4 z-40 flex-col bg-black/10 dark:bg-white/20 backdrop-blur-sm rounded-full py-1 px-0">
          {ALPHABET.map(letter => {
            const hasMovies = moviesByLetter[letter]?.length > 0
            return (
              <button
                key={letter}
                onClick={() => hasMovies && scrollToLetter(letter)}
                disabled={!hasMovies}
                className={`w-3 flex-1 min-h-0 text-[8px] font-semibold rounded-full transition-all flex items-center justify-center ${
                  activeLetter === letter
                    ? 'bg-primary/20 text-primary-foreground'
                    : hasMovies
                      ? 'text-gray-600 dark:text-gray-300 hover:text-gray-900 dark:hover:text-white'
                      : 'text-gray-300 dark:text-gray-600 cursor-default'
                }`}
              >
                {letter}
              </button>
            )
          })}
        </nav>
      )}

      <div className="space-y-4 md:space-y-6 md:pr-10">
        <div className="flex flex-col gap-4 sm:flex-row sm:items-center sm:justify-between">
          <div>
            <h1 className="text-xl md:text-2xl font-bold">{t('movies.title')}</h1>
            {total > 0 && (
              <p className="text-xs md:text-sm text-muted-foreground">
                {total} {t('movies.inCollection')}
              </p>
            )}
          </div>
          <Link
            to="/scan"
            className="flex items-center justify-center gap-2 rounded-md bg-primary px-4 py-3 text-sm text-primary-foreground hover:bg-primary/90 active:bg-primary/80 min-h-touch"
          >
            <Plus className="h-4 w-4" />
            <span className="hidden sm:inline">{t('movies.add')}</span>
            <span className="sm:hidden">{t('common.add')}</span>
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
              className="w-full rounded-md border bg-background pl-9 pr-3 py-3 md:py-2 text-sm focus:border-primary focus:outline-none focus:ring-1 focus:ring-primary min-h-touch"
            />
          </div>
          <button
            type="submit"
            className="rounded-md bg-secondary px-4 py-3 md:py-2 text-sm hover:bg-secondary/80 active:bg-secondary/60 min-h-touch min-w-touch"
          >
            <span className="hidden sm:inline">{t('movies.search')}</span>
            <Search className="h-4 w-4 sm:hidden" />
          </button>
        </form>

        {/* Filter pills */}
        <div className="flex flex-wrap gap-2">
          <button
            onClick={() => handleFilterChange({ watched: filter.watched === 'true' ? undefined : 'true' })}
            className={`flex items-center gap-1 rounded-full px-3 py-2 md:py-1 text-xs min-h-touch ${
              filter.watched === 'true'
                ? 'bg-primary text-primary-foreground'
                : 'bg-secondary hover:bg-secondary/80 active:bg-secondary/60'
            }`}
          >
            <Check className="h-3 w-3" />
            {t('movies.watched')}
          </button>
          <button
            onClick={() => handleFilterChange({ watched: filter.watched === 'false' ? undefined : 'false' })}
            className={`flex items-center gap-1 rounded-full px-3 py-2 md:py-1 text-xs min-h-touch ${
              filter.watched === 'false'
                ? 'bg-primary text-primary-foreground'
                : 'bg-secondary hover:bg-secondary/80 active:bg-secondary/60'
            }`}
          >
            <X className="h-3 w-3" />
            {t('movies.notWatched')}
          </button>
          <select
            value={filter.disc_type || ''}
            onChange={e => handleFilterChange({ disc_type: e.target.value || undefined })}
            className="rounded-full border bg-background px-3 py-2 md:py-1 text-xs min-h-touch"
          >
            <option value="">{t('movies.allFormats')}</option>
            <option value="Blu-ray">Blu-ray</option>
            <option value="DVD">DVD</option>
            <option value="uhdbluray">4K UHD</option>
          </select>
        </div>

        {/* Movies grouped by letter */}
        {isLoading ? (
          <div className="text-center py-12 text-muted-foreground text-sm md:text-base">{t('common.loading')}</div>
        ) : movies.length === 0 ? (
          <div className="text-center py-12">
            <Film className="mx-auto h-12 w-12 text-muted-foreground" />
            <p className="mt-4 text-muted-foreground text-sm md:text-base">{t('movies.notFound')}</p>
            <Link
              to="/scan"
              className="mt-4 inline-flex items-center gap-2 rounded-md bg-primary px-4 py-3 text-sm text-primary-foreground hover:bg-primary/90 active:bg-primary/80 min-h-touch"
            >
              <Plus className="h-4 w-4" />
              {t('movies.addFirstMovie')}
            </Link>
          </div>
        ) : search ? (
          // Flat grid when searching
          <div className="grid gap-3 md:gap-4 grid-cols-2 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5 xl:grid-cols-6">
            {movies.map(movie => (
              <MovieCard key={movie.id} movie={movie} />
            ))}
          </div>
        ) : (
          // Grouped by letter when not searching
          <div className="space-y-6 md:space-y-8">
            {availableLetters.map(letter => (
              <section
                key={letter}
                ref={el => { sectionRefs.current[letter] = el }}
                className="scroll-mt-20 md:scroll-mt-24"
              >
                <h2 className="text-base md:text-lg font-bold mb-3 md:mb-4 sticky top-14 md:top-16 bg-background/95 backdrop-blur py-2 z-10 border-b">
                  {letter}
                  <span className="text-xs md:text-sm font-normal text-muted-foreground ml-2">
                    ({moviesByLetter[letter]?.length})
                  </span>
                </h2>
                <div className="grid gap-3 md:gap-4 grid-cols-2 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5 xl:grid-cols-6">
                  {moviesByLetter[letter]?.map(movie => (
                    <MovieCard key={movie.id} movie={movie} />
                  ))}
                </div>
              </section>
            ))}
          </div>
        )}
      </div>
    </div>
  )
}

function MovieCard({ movie }: { movie: Movie }) {
  return (
    <Link
      to="/movies/$movieId"
      params={{ movieId: movie.id }}
      className="group rounded-lg border bg-card overflow-hidden hover:border-primary active:border-primary text-left transition-all hover:shadow-lg active:shadow-md w-full block"
    >
      <div className="aspect-[2/3] bg-muted flex items-center justify-center relative overflow-hidden">
        <PosterImage
          posterPath={null}
          movieId={movie.id}
          size="w342"
          alt={movie.title}
          className="w-full h-full object-cover transition-transform group-hover:scale-105 group-active:scale-100"
          updatedAt={movie.updated_at}
        />
        {movie.watched && (
          <div className="absolute top-2 right-2 rounded-full bg-green-500 p-1">
            <Check className="h-3 w-3 text-white" />
          </div>
        )}
      </div>
      <div className="p-2 md:p-3">
        <h3 className="font-medium text-xs md:text-sm truncate group-active:text-primary">
          {movie.title}
        </h3>
        <div className="flex items-center gap-2 text-xs text-muted-foreground mt-1">
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
    </Link>
  )
}

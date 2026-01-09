import { createFileRoute, redirect, Link, useNavigate, getRouteApi } from '@tanstack/react-router'
import { useQuery } from '@tanstack/react-query'
import { useState, useEffect, useRef, useMemo, useCallback } from 'react'
import { Film, Search, Check, X, SlidersHorizontal } from 'lucide-react'
import { api, MovieFilter, Movie } from '@/lib/api'
import { useI18n } from '@/hooks/useI18n'
import { PosterImage } from '@/components/PosterImage'
import { FAB } from '@/components/FAB'
import { useSearchToolbar } from './__root'
import type { MoviesSearchParams } from './movies'

// Get parent route API for search params (defined in movies.tsx)
const moviesRouteApi = getRouteApi('/movies')

// Key for storing scroll position
const SCROLL_KEY = 'movies-list-scroll'

export const Route = createFileRoute('/movies/')({
  beforeLoad: ({ context }) => {
    if (!context.auth.isAuthenticated) {
      throw redirect({ to: '/login' })
    }
  },
  component: MoviesPage,
})

const ALPHABET = '#ABCDEFGHIJKLMNOPQRSTUVWXYZ'.split('')

type CardSize = 'small' | 'medium' | 'large'

// Get card size from localStorage
function getStoredCardSize(): CardSize {
  if (typeof window === 'undefined') return 'medium'
  return (localStorage.getItem('cardSize') as CardSize) || 'medium'
}

function MoviesPage() {
  const { t } = useI18n()
  const navigate = useNavigate()
  const searchParams = moviesRouteApi.useSearch()
  
  // Local search input state (for typing before submitting)
  const [searchInput, setSearchInput] = useState(searchParams.search || '')
  const [activeLetter, setActiveLetter] = useState<string | null>(null)
  const [cardSize, setCardSize] = useState<CardSize>(getStoredCardSize)
  const { showToolbar, setShowToolbar, setHasActiveFilter } = useSearchToolbar()
  const [showFilterDropdown, setShowFilterDropdown] = useState(false)
  const sectionRefs = useRef<Record<string, HTMLElement | null>>({})
  const searchInputRef = useRef<HTMLInputElement>(null)
  const filterDropdownRef = useRef<HTMLDivElement>(null)

  // Build filter from URL params
  const filter: MovieFilter = useMemo(() => ({
    search: searchParams.search,
    watched: searchParams.watched,
    disc_type: searchParams.disc_type,
    is_collection: searchParams.is_collection,
  }), [searchParams])

  // Check if any filter is active
  const hasActiveFilter = searchParams.watched !== undefined || 
    searchParams.disc_type !== undefined || 
    searchParams.is_collection !== undefined || 
    searchParams.search !== undefined

  // Update the context when filter state changes
  useEffect(() => {
    setHasActiveFilter(hasActiveFilter)
  }, [hasActiveFilter, setHasActiveFilter])

  // Update URL params helper
  const updateSearchParams = useCallback((updates: Partial<MoviesSearchParams>) => {
    const newParams = { ...searchParams, ...updates }
    // Remove undefined values
    Object.keys(newParams).forEach(key => {
      if (newParams[key as keyof MoviesSearchParams] === undefined) {
        delete newParams[key as keyof MoviesSearchParams]
      }
    })
    navigate({ 
      to: '/movies',
      search: newParams,
      replace: true // Don't add to history for filter changes
    })
  }, [searchParams, navigate])

  // Listen for cardSize changes from localStorage (set in Me page)
  useEffect(() => {
    const handleStorage = () => {
      setCardSize(getStoredCardSize())
    }
    window.addEventListener('storage', handleStorage)
    // Also check on focus in case changed in same tab
    const handleFocus = () => setCardSize(getStoredCardSize())
    window.addEventListener('focus', handleFocus)
    return () => {
      window.removeEventListener('storage', handleStorage)
      window.removeEventListener('focus', handleFocus)
    }
  }, [])

  // Focus search input when toolbar is shown
  useEffect(() => {
    if (showToolbar && searchInputRef.current) {
      searchInputRef.current.focus()
    }
  }, [showToolbar])

  // Close filter dropdown when clicking outside
  useEffect(() => {
    const handleClickOutside = (e: MouseEvent) => {
      if (filterDropdownRef.current && !filterDropdownRef.current.contains(e.target as Node)) {
        setShowFilterDropdown(false)
      }
    }
    if (showFilterDropdown) {
      document.addEventListener('mousedown', handleClickOutside)
    }
    return () => document.removeEventListener('mousedown', handleClickOutside)
  }, [showFilterDropdown])

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

  // Save scroll position on scroll - listen on the main scrollable container
  useEffect(() => {
    const mainElement = document.querySelector('main')
    if (!mainElement) return
    
    const handleScroll = () => {
      sessionStorage.setItem(SCROLL_KEY, mainElement.scrollTop.toString())
    }
    
    mainElement.addEventListener('scroll', handleScroll, { passive: true })
    return () => mainElement.removeEventListener('scroll', handleScroll)
  }, [])

  // Restore scroll position when coming back from movie detail
  useEffect(() => {
    const shouldRestore = sessionStorage.getItem('movies-should-restore-scroll')
    const savedScroll = sessionStorage.getItem(SCROLL_KEY)
    
    if (!isLoading && movies.length > 0 && shouldRestore === 'true' && savedScroll) {
      const scrollY = parseInt(savedScroll, 10)
      sessionStorage.removeItem('movies-should-restore-scroll')
      
      const mainElement = document.querySelector('main')
      if (!mainElement) return
      
      // Use multiple attempts to ensure scroll works after DOM settles
      const attemptScroll = (attempts: number) => {
        if (attempts <= 0) return
        requestAnimationFrame(() => {
          mainElement.scrollTop = scrollY
          if (Math.abs(mainElement.scrollTop - scrollY) > 50 && attempts > 1) {
            setTimeout(() => attemptScroll(attempts - 1), 50)
          }
        })
      }
      
      setTimeout(() => attemptScroll(5), 50)
    } else if (shouldRestore === 'true') {
      sessionStorage.removeItem('movies-should-restore-scroll')
    }
  }, [isLoading, movies.length])

  const handleSearch = (e: React.FormEvent) => {
    e.preventDefault()
    updateSearchParams({ search: searchInput || undefined })
  }

  const handleFilterChange = (newFilter: Partial<MoviesSearchParams>) => {
    updateSearchParams(newFilter)
  }

  const clearAllFilters = () => {
    setSearchInput('')
    navigate({ to: '/movies', search: {}, replace: true })
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

  // Card size grid classes
  const gridClasses = {
    small: 'grid-cols-4 sm:grid-cols-5 md:grid-cols-6 lg:grid-cols-8 xl:grid-cols-10',
    medium: 'grid-cols-3 sm:grid-cols-4 md:grid-cols-5 lg:grid-cols-6 xl:grid-cols-8',
    large: 'grid-cols-2 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5 xl:grid-cols-6',
  }

  return (
    <div className="relative">
      {/* Sticky Toolbar - Search & Filters (toggleable) */}
      {showToolbar && (
        <div 
          className="fixed left-0 right-0 z-30 bg-card border-b animate-in slide-in-from-top-2 duration-200"
          style={{ 
            top: 'calc(3.5rem + env(safe-area-inset-top, 0px))', // Directly below header (h-14 = 3.5rem)
          }}
        >
          <div className="container px-4 pr-10 md:pr-14 py-2">
            {/* Single row: Search + Filter button + Done */}
            <div className="flex items-center gap-2">
              {/* Search input */}
              <form onSubmit={handleSearch} className="relative flex-1">
                <Search className="absolute left-2.5 top-1/2 h-3.5 md:h-4 w-3.5 md:w-4 -translate-y-1/2 text-muted-foreground pointer-events-none" />
                <input
                  ref={searchInputRef}
                  type="text"
                  placeholder={t('movies.searchPlaceholder')}
                  value={searchInput}
                  onChange={e => setSearchInput(e.target.value)}
                  className="w-full rounded-full border-0 bg-muted/60 pl-7 md:pl-9 pr-3 py-1.5 md:py-2 text-xs md:text-sm focus:bg-muted focus:outline-none"
                />
                {searchInput && (
                  <button
                    type="button"
                    onClick={() => {
                      setSearchInput('')
                      updateSearchParams({ search: undefined })
                    }}
                    className="absolute right-1 top-1/2 -translate-y-1/2 p-0.5 text-muted-foreground hover:text-foreground"
                  >
                    <X className="h-4 w-4" />
                  </button>
                )}
              </form>

              {/* Filter button with dropdown */}
              <div className="relative" ref={filterDropdownRef}>
                <button
                  type="button"
                  onClick={() => setShowFilterDropdown(!showFilterDropdown)}
                  className="relative flex items-center justify-center h-8 w-8 text-muted-foreground hover:text-foreground"
                >
                  <SlidersHorizontal className="h-4 w-4" />
                  {/* Badge for active filters */}
                  {(searchParams.watched !== undefined || searchParams.disc_type !== undefined || searchParams.is_collection !== undefined) && (
                    <span className="absolute top-0.5 right-0.5 h-2 w-2 rounded-full bg-primary" />
                  )}
                </button>

                {/* Filter dropdown */}
                {showFilterDropdown && (
                  <div className="absolute right-0 top-full mt-2 bg-card border rounded-lg shadow-lg p-3 min-w-[220px] z-50 animate-in fade-in slide-in-from-top-2 duration-150">
                    {/* Watched filter */}
                    <div className="mb-3">
                      <p className="text-xs text-muted-foreground mb-2">{t('movies.watched')}</p>
                      <div className="flex bg-muted rounded-full w-fit">
                        <button
                          onClick={() => handleFilterChange({ watched: searchParams.watched === 'true' ? undefined : 'true' })}
                          className={`px-3 py-1.5 m-0.5 text-xs rounded-full transition-colors ${searchParams.watched === 'true' ? 'bg-background shadow-sm' : 'text-muted-foreground'}`}
                        >✓</button>
                        <button
                          onClick={() => handleFilterChange({ watched: searchParams.watched === 'false' ? undefined : 'false' })}
                          className={`px-3 py-1.5 m-0.5 text-xs rounded-full transition-colors ${searchParams.watched === 'false' ? 'bg-background shadow-sm' : 'text-muted-foreground'}`}
                        >○</button>
                      </div>
                    </div>

                    {/* Type filter (Film/Collection) */}
                    <div className="mb-3">
                      <p className="text-xs text-muted-foreground mb-2">{t('movies.type')}</p>
                      <div className="flex bg-muted rounded-full w-fit">
                        <button
                          onClick={() => handleFilterChange({ is_collection: undefined })}
                          className={`px-2.5 py-1.5 m-0.5 text-xs rounded-full transition-colors ${
                            searchParams.is_collection === undefined ? 'bg-background shadow-sm' : 'text-muted-foreground'
                          }`}
                        >{t('movies.allTypes')}</button>
                        <button
                          onClick={() => handleFilterChange({ is_collection: searchParams.is_collection === 'false' ? undefined : 'false' })}
                          className={`px-2.5 py-1.5 m-0.5 text-xs rounded-full transition-colors ${
                            searchParams.is_collection === 'false' ? 'bg-background shadow-sm' : 'text-muted-foreground'
                          }`}
                        >{t('movies.movie')}</button>
                        <button
                          onClick={() => handleFilterChange({ is_collection: searchParams.is_collection === 'true' ? undefined : 'true' })}
                          className={`px-2.5 py-1.5 m-0.5 text-xs rounded-full transition-colors ${
                            searchParams.is_collection === 'true' ? 'bg-background shadow-sm' : 'text-muted-foreground'
                          }`}
                        >{t('movies.collection')}</button>
                      </div>
                    </div>

                    {/* Format filter */}
                    <div className="mb-3">
                      <p className="text-xs text-muted-foreground mb-2">{t('movies.format')}</p>
                      <div className="flex bg-muted rounded-full w-fit">
                        {[['', '∗'], ['Blu-ray', 'BD'], ['DVD', 'DVD'], ['uhdbluray', '4K']].map(([val, label]) => (
                          <button
                            key={val}
                            onClick={() => handleFilterChange({ disc_type: (!val && !searchParams.disc_type) || searchParams.disc_type === val ? undefined : val || undefined })}
                            className={`px-2.5 py-1.5 m-0.5 text-xs rounded-full transition-colors ${
                              (val === '' && !searchParams.disc_type) || searchParams.disc_type === val ? 'bg-background shadow-sm' : 'text-muted-foreground'
                            }`}
                          >{label}</button>
                        ))}
                      </div>
                    </div>

                    {/* Clear filters */}
                    {(searchParams.watched !== undefined || searchParams.disc_type !== undefined || searchParams.is_collection !== undefined) && (
                      <button
                        onClick={() => {
                          clearAllFilters()
                          setShowFilterDropdown(false)
                        }}
                        className="w-full text-xs text-destructive hover:underline text-center pt-2 border-t"
                      >
                        {t('movies.clearFilters')}
                      </button>
                    )}
                  </div>
                )}
              </div>

              {/* Done/Close button */}
              <button
                type="button"
                onClick={() => {
                  setShowToolbar(false)
                  setShowFilterDropdown(false)
                }}
                className="shrink-0 text-sm text-destructive font-medium"
              >
                {t('common.done')}
              </button>
            </div>
          </div>
        </div>
      )}

      {/* Alphabet Navigation - Vertical on right (both mobile and desktop) */}
      {!searchParams.search && availableLetters.length > 0 && (
        <nav className="fixed right-1 md:right-4 z-40 flex flex-col"
          style={{
            top: showToolbar ? 'calc(6.5rem + env(safe-area-inset-top, 0px))' : 'calc(5rem + env(safe-area-inset-top, 0px))',
            bottom: 'calc(5rem + env(safe-area-inset-bottom, 0px))',
          }}
        >
          {ALPHABET.map(letter => {
            const hasMovies = moviesByLetter[letter]?.length > 0
            return (
              <button
                key={letter}
                onClick={() => hasMovies && scrollToLetter(letter)}
                disabled={!hasMovies}
                className={`w-4 md:w-5 flex-1 min-h-0 text-[10px] md:text-[10px] font-semibold transition-all flex items-center justify-center ${
                  activeLetter === letter
                    ? 'text-primary font-bold scale-125'
                    : hasMovies
                      ? 'text-gray-400 dark:text-gray-500 hover:text-gray-700 dark:hover:text-gray-300'
                      : 'text-gray-200 dark:text-gray-700 cursor-default'
                }`}
              >
                {letter}
              </button>
            )
          })}
        </nav>
      )}

      {/* Content with right padding for alphabet nav */}
      <div className={`pr-6 md:pr-10 ${showToolbar ? 'pt-12' : ''}`}>
        {/* Title and count */}
        <div className="py-4">
          <h1 className="text-xl md:text-2xl font-bold">{t('movies.title')}</h1>
          {total > 0 && (
            <p className="text-xs md:text-sm text-muted-foreground">
              {total} {t('movies.inCollection')}
            </p>
          )}
        </div>

        {/* Movies grouped by letter */}
        {isLoading ? (
          <div className="text-center py-12 text-muted-foreground text-sm md:text-base">{t('common.loading')}</div>
        ) : movies.length === 0 ? (
          <div className="text-center py-12">
            <Film className="mx-auto h-12 w-12 text-muted-foreground" />
            <p className="mt-4 text-muted-foreground text-sm md:text-base">{t('movies.notFound')}</p>
          </div>
        ) : searchParams.search ? (
          // Flat grid when searching
          <div className={`grid gap-3 md:gap-4 ${gridClasses[cardSize]}`}>
            {movies.map(movie => (
              <MovieCard key={movie.id} movie={movie} size={cardSize} />
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
                <h2 className="text-base md:text-lg font-bold mb-3 md:mb-4 sticky top-0 bg-background/95 backdrop-blur py-2 z-10 border-b">
                  {letter}
                  <span className="text-xs md:text-sm font-normal text-muted-foreground ml-2">
                    ({moviesByLetter[letter]?.length})
                  </span>
                </h2>
                <div className={`grid gap-3 md:gap-4 ${gridClasses[cardSize]}`}>
                  {moviesByLetter[letter]?.map(movie => (
                    <MovieCard key={movie.id} movie={movie} size={cardSize} />
                  ))}
                </div>
              </section>
            ))}
          </div>
        )}
      </div>

      {/* Floating Action Button */}
      <FAB showScrollTop />
    </div>
  )
}

function MovieCard({ movie, size }: { movie: Movie; size: CardSize }) {
  const showDetails = size !== 'small'
  
  const handleClick = () => {
    // Mark that we should restore scroll when coming back
    sessionStorage.setItem('movies-should-restore-scroll', 'true')
  }
  
  return (
    <Link
      to="/movies/$movieId"
      params={{ movieId: movie.id }}
      onClick={handleClick}
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
          <div className={`absolute top-1 right-1 rounded-full bg-green-500 p-0.5 ${size === 'small' ? 'top-0.5 right-0.5' : 'top-2 right-2 p-1'}`}>
            <Check className={size === 'small' ? 'h-2 w-2 text-white' : 'h-3 w-3 text-white'} />
          </div>
        )}
      </div>
      {showDetails && (
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
      )}
    </Link>
  )
}

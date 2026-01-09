import { useRef, useMemo, useState, useEffect, useImperativeHandle, forwardRef } from 'react'
import { useVirtualizer } from '@tanstack/react-virtual'
import { Link } from '@tanstack/react-router'
import { Check } from 'lucide-react'
import { Movie } from '@/lib/api'
import { PosterImage } from './PosterImage'

type CardSize = 'small' | 'medium' | 'large'

interface VirtualizedMovieGridProps {
  movies: Movie[]
  cardSize: CardSize
  onMovieClick?: () => void
}

// Get columns based on card size and window width
function getColumns(cardSize: CardSize, width: number): number {
  // Breakpoints: sm:640, md:768, lg:1024, xl:1280
  const configs = {
    small: { xs: 4, sm: 5, md: 6, lg: 8, xl: 10 },
    medium: { xs: 3, sm: 4, md: 5, lg: 6, xl: 8 },
    large: { xs: 2, sm: 3, md: 4, lg: 5, xl: 6 },
  }
  const config = configs[cardSize]
  
  if (width >= 1280) return config.xl
  if (width >= 1024) return config.lg
  if (width >= 768) return config.md
  if (width >= 640) return config.sm
  return config.xs
}

// Estimate row height based on card size
function getRowHeight(cardSize: CardSize, columnWidth: number): number {
  // Aspect ratio 2:3 for poster + padding + details
  const posterHeight = columnWidth * 1.5
  const showDetails = cardSize !== 'small'
  const detailsHeight = showDetails ? 60 : 0
  const gap = 16 // gap-3 or gap-4
  return posterHeight + detailsHeight + gap
}

export function VirtualizedMovieGrid({ movies, cardSize, onMovieClick }: VirtualizedMovieGridProps) {
  const parentRef = useRef<HTMLDivElement>(null)
  const [containerWidth, setContainerWidth] = useState(800)
  
  // Track container width for responsive columns
  useEffect(() => {
    if (!parentRef.current) return
    
    const updateWidth = () => {
      if (parentRef.current) {
        setContainerWidth(parentRef.current.offsetWidth)
      }
    }
    
    updateWidth()
    
    const observer = new ResizeObserver(updateWidth)
    observer.observe(parentRef.current)
    
    return () => observer.disconnect()
  }, [])
  
  const columns = useMemo(() => getColumns(cardSize, containerWidth), [cardSize, containerWidth])
  const columnWidth = useMemo(() => (containerWidth - (columns - 1) * 16) / columns, [containerWidth, columns])
  const rowHeight = useMemo(() => getRowHeight(cardSize, columnWidth), [cardSize, columnWidth])
  
  // Group movies into rows
  const rows = useMemo(() => {
    const result: Movie[][] = []
    for (let i = 0; i < movies.length; i += columns) {
      result.push(movies.slice(i, i + columns))
    }
    return result
  }, [movies, columns])
  
  const scrollElementRef = useRef<Element | null>(null)
  
  // Get scroll element once on mount
  useEffect(() => {
    scrollElementRef.current = document.querySelector('main')
  }, [])
  
  const virtualizer = useVirtualizer({
    count: rows.length,
    getScrollElement: () => scrollElementRef.current,
    estimateSize: () => rowHeight,
    overscan: 3, // Render 3 extra rows above/below viewport
    scrollPaddingStart: 80, // Account for sticky header
  })
  
  const virtualRows = virtualizer.getVirtualItems()
  
  return (
    <div 
      ref={parentRef}
      className="w-full"
      style={{ height: `${virtualizer.getTotalSize()}px`, position: 'relative' }}
    >
      {virtualRows.map((virtualRow) => {
        const rowMovies = rows[virtualRow.index]
        return (
          <div
            key={virtualRow.key}
            style={{
              position: 'absolute',
              top: 0,
              left: 0,
              width: '100%',
              height: `${virtualRow.size}px`,
              transform: `translateY(${virtualRow.start}px)`,
            }}
          >
            <div className="grid gap-3 md:gap-4" style={{ gridTemplateColumns: `repeat(${columns}, 1fr)` }}>
              {rowMovies.map((movie) => (
                <MovieCard key={movie.id} movie={movie} size={cardSize} onClick={onMovieClick} />
              ))}
            </div>
          </div>
        )
      })}
    </div>
  )
}

// Grouped version for alphabet sections
interface VirtualizedMovieGridGroupedProps {
  moviesByLetter: Record<string, Movie[]>
  availableLetters: string[]
  cardSize: CardSize
  onMovieClick?: () => void
}

export interface VirtualizedMovieGridGroupedHandle {
  scrollToLetter: (letter: string) => void
}

export const VirtualizedMovieGridGrouped = forwardRef<VirtualizedMovieGridGroupedHandle, VirtualizedMovieGridGroupedProps>(
  function VirtualizedMovieGridGrouped({ 
    moviesByLetter, 
    availableLetters, 
    cardSize, 
    onMovieClick,
  }, ref) {
    const parentRef = useRef<HTMLDivElement>(null)
    const [containerWidth, setContainerWidth] = useState(800)
    
    // Track container width for responsive columns
    useEffect(() => {
      if (!parentRef.current) return
      
      const updateWidth = () => {
        if (parentRef.current) {
          setContainerWidth(parentRef.current.offsetWidth)
        }
      }
      
      updateWidth()
      
      const observer = new ResizeObserver(updateWidth)
      observer.observe(parentRef.current)
      
      return () => observer.disconnect()
    }, [])
    
    const columns = useMemo(() => getColumns(cardSize, containerWidth), [cardSize, containerWidth])
    const columnWidth = useMemo(() => (containerWidth - (columns - 1) * 16) / columns, [containerWidth, columns])
    const rowHeight = useMemo(() => getRowHeight(cardSize, columnWidth), [cardSize, columnWidth])
    
    // Build flat list of items (headers + movie rows)
    interface RowItem {
      type: 'header' | 'movies'
      letter?: string
      count?: number
      movies?: Movie[]
    }
    
    const { items, letterIndices } = useMemo(() => {
      const result: RowItem[] = []
      const indices: Record<string, number> = {}
      
      for (const letter of availableLetters) {
        const letterMovies = moviesByLetter[letter] || []
        
        // Store index of header for this letter
        indices[letter] = result.length
        
        // Add header
        result.push({ type: 'header', letter, count: letterMovies.length })
        
        // Add movie rows
        for (let i = 0; i < letterMovies.length; i += columns) {
          result.push({ type: 'movies', letter, movies: letterMovies.slice(i, i + columns) })
        }
      }
      
      return { items: result, letterIndices: indices }
    }, [moviesByLetter, availableLetters, columns])
    
    const scrollElementRef = useRef<Element | null>(null)
    
    // Get scroll element once on mount
    useEffect(() => {
      scrollElementRef.current = document.querySelector('main')
    }, [])
    
    const virtualizer = useVirtualizer({
      count: items.length,
      getScrollElement: () => scrollElementRef.current,
      estimateSize: (index) => {
        const item = items[index]
        return item.type === 'header' ? 48 : rowHeight // Header is ~48px
      },
      overscan: 5,
      scrollPaddingStart: 80, // Account for sticky header
    })
    
    // Expose scrollToLetter function via ref
    useImperativeHandle(ref, () => ({
      scrollToLetter: (letter: string) => {
        const index = letterIndices[letter]
        if (index !== undefined) {
          // Scroll to the header with some offset for the sticky toolbar
          virtualizer.scrollToIndex(index, { align: 'start', behavior: 'smooth' })
        }
      }
    }), [letterIndices, virtualizer])
    
    const virtualItems = virtualizer.getVirtualItems()
    
    return (
      <div 
        ref={parentRef}
        className="w-full"
        style={{ height: `${virtualizer.getTotalSize()}px`, position: 'relative' }}
      >
        {virtualItems.map((virtualItem) => {
          const item = items[virtualItem.index]
          
          if (item.type === 'header') {
            return (
              <div
                key={virtualItem.key}
                data-letter={item.letter}
                style={{
                  position: 'absolute',
                  top: 0,
                  left: 0,
                  width: '100%',
                  height: `${virtualItem.size}px`,
                  transform: `translateY(${virtualItem.start}px)`,
                }}
              >
                <h2 className="text-base md:text-lg font-bold sticky top-0 bg-background/95 backdrop-blur py-2 z-10 border-b">
                  {item.letter}
                  <span className="text-xs md:text-sm font-normal text-muted-foreground ml-2">
                    ({item.count})
                  </span>
                </h2>
              </div>
            )
          }
          
          return (
            <div
              key={virtualItem.key}
              style={{
                position: 'absolute',
                top: 0,
                left: 0,
                width: '100%',
                height: `${virtualItem.size}px`,
                transform: `translateY(${virtualItem.start}px)`,
              }}
            >
              <div className="grid gap-3 md:gap-4" style={{ gridTemplateColumns: `repeat(${columns}, 1fr)` }}>
                {item.movies?.map((movie) => (
                  <MovieCard key={movie.id} movie={movie} size={cardSize} onClick={onMovieClick} />
                ))}
              </div>
            </div>
          )
        })}
      </div>
    )
  }
)

function MovieCard({ movie, size, onClick }: { movie: Movie; size: CardSize; onClick?: () => void }) {
  const showDetails = size !== 'small'
  
  const handleClick = () => {
    // Mark that we should restore scroll when coming back
    sessionStorage.setItem('movies-should-restore-scroll', 'true')
    onClick?.()
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
          useThumbnail={true}
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

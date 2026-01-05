import { useState, useEffect, useRef } from 'react'
import { imageQueue } from '@/lib/imageQueue'

interface PosterImageProps {
  posterPath: string | null | undefined
  movieId?: string
  size?: 'w92' | 'w342' | 'w500'
  alt: string
  className?: string
  updatedAt?: string // Timestamp to force reload when movie is updated
}

export function PosterImage({ posterPath, movieId, size = 'w342', alt, className = '', updatedAt }: PosterImageProps) {
  const [imageUrl, setImageUrl] = useState<string | null>(null)
  const [imageError, setImageError] = useState(false)
  const [shouldLoad, setShouldLoad] = useState(false)
  const containerRef = useRef<HTMLDivElement>(null)
  const blobUrlRef = useRef<string | null>(null)
  const mountedRef = useRef(true)
  const lastUpdatedRef = useRef<string | undefined>(updatedAt)

  // Intersection Observer for lazy loading with prefetch
  useEffect(() => {
    if (!containerRef.current) return

    // Check if element is already visible (e.g., when navigating back to page)
    const checkVisibility = () => {
      const rect = containerRef.current?.getBoundingClientRect()
      if (rect) {
        const isVisible = rect.top < window.innerHeight + 1000 && rect.bottom > -1000
        if (isVisible) {
          setShouldLoad(true)
          return true
        }
      }
      return false
    }

    // Check immediately if already visible
    if (checkVisibility()) {
      return
    }

    const observer = new IntersectionObserver(
      (entries) => {
        entries.forEach((entry) => {
          if (entry.isIntersecting) {
            setShouldLoad(true)
            observer.disconnect()
          }
        })
      },
      {
        // Start loading when element is within 10 viewport heights
        rootMargin: '1000px 0px', // ~10 viewport heights (assuming ~100px per item)
        threshold: 0,
      }
    )

    observer.observe(containerRef.current)

    return () => {
      observer.disconnect()
    }
  }, [])

  // Load image when shouldLoad becomes true or when updatedAt changes
  useEffect(() => {
    if (!shouldLoad) return

    // Check if movie was updated (updatedAt changed)
    const wasUpdated = updatedAt && updatedAt !== lastUpdatedRef.current
    if (wasUpdated) {
      lastUpdatedRef.current = updatedAt
      // Reset image state to force reload
      setImageUrl(null)
      setImageError(false)
      
      // Reset blob URL reference (URL is cached in imageQueue)
      blobUrlRef.current = null
    }

    mountedRef.current = true

    // Reset blob URL reference if not already done (URL is cached in imageQueue)
    if (!wasUpdated && blobUrlRef.current) {
      blobUrlRef.current = null
    }

    // If movieId is provided, always try to load from database first
    // Use queue for sequential loading to avoid blocking UI
    // Add cache-busting query param if movie was updated
    if (movieId) {
      const currentMovieId = movieId // Capture for cleanup check
      const url = wasUpdated 
        ? `/api/v1/movies/${movieId}/poster?t=${Date.now()}`
        : `/api/v1/movies/${movieId}/poster`
      
      imageQueue.load(url)
        .then(url => {
          // Only update if component is still mounted and movieId hasn't changed
          if (mountedRef.current && currentMovieId === movieId) {
            if (url) {
              blobUrlRef.current = url
              setImageUrl(url)
              setImageError(false)
            } else {
              // If no poster in DB, fall back to posterPath logic
              handlePosterPathFallback()
            }
          } else if (url) {
            // Don't revoke - URL is cached in imageQueue and may be reused
            // The cache manages blob URL lifecycle
          }
        })
        .catch(() => {
          // If loading from DB fails, fall back to posterPath logic
          if (mountedRef.current && currentMovieId === movieId) {
            handlePosterPathFallback()
          }
        })
    } else {
      // Fallback: handle posterPath if no movieId
      handlePosterPathFallback()
    }

    // Fallback: handle posterPath if no movieId
    function handlePosterPathFallback() {
      if (!posterPath) {
        setImageUrl(null)
        setImageError(true)
        return
      }

      // If it starts with http, it's a full URL (external, no auth needed)
      if (posterPath.startsWith('http')) {
        setImageUrl(posterPath)
        setImageError(false)
        return
      }

      // If it starts with /uploads, it's a local file (legacy, no auth needed)
      if (posterPath.startsWith('/uploads')) {
        setImageUrl(posterPath)
        setImageError(false)
        return
      }

      // Otherwise it's a TMDB path (external, no auth needed)
      if (posterPath && !posterPath.startsWith('http')) {
        setImageUrl(`https://image.tmdb.org/t/p/${size}${posterPath}`)
        setImageError(false)
      } else {
        setImageUrl(null)
        setImageError(true)
      }
    }

    // Cleanup on unmount
    // Note: We don't revoke blob URLs here because they're cached in imageQueue
    // and may be reused by other components. The cache manages blob URL lifecycle.
    return () => {
      mountedRef.current = false
      // Don't revoke blob URLs - they're managed by the imageQueue cache
      blobUrlRef.current = null
    }
  }, [shouldLoad, posterPath, movieId, size, updatedAt])

  // Extract size classes from className to apply to container
  const containerClasses = className.includes('w-full') && className.includes('h-full') 
    ? className 
    : `${className} w-full h-full`

  return (
    <div 
      ref={containerRef} 
      className={containerClasses}
      style={{ position: 'relative' }}
    >
      {!shouldLoad ? (
        // Placeholder while waiting to load
        <div className="absolute inset-0 bg-muted flex items-center justify-center">
          <span className="text-muted-foreground text-xs opacity-50">...</span>
        </div>
      ) : !imageUrl || imageError ? (
        // No image or error
        <div className="absolute inset-0 bg-muted flex items-center justify-center">
          <span className="text-muted-foreground text-sm">No image</span>
        </div>
      ) : (
        // Image loaded
        <img
          src={imageUrl}
          alt={alt}
          className="w-full h-full object-cover"
          onError={() => setImageError(true)}
        />
      )}
    </div>
  )
}


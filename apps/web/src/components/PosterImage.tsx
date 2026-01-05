import { useState, useEffect } from 'react'
import { loadAuthenticatedImage } from '@/lib/imageLoader'

interface PosterImageProps {
  posterPath: string | null | undefined
  movieId?: string
  size?: 'w92' | 'w342' | 'w500'
  alt: string
  className?: string
}

export function PosterImage({ posterPath, movieId, size = 'w342', alt, className = '' }: PosterImageProps) {
  const [imageUrl, setImageUrl] = useState<string | null>(null)
  const [imageError, setImageError] = useState(false)

  useEffect(() => {
    if (!posterPath) {
      setImageUrl(null)
      setImageError(false)
      return
    }

    // If it starts with http, it's a full URL (external, no auth needed)
    if (posterPath.startsWith('http')) {
      setImageUrl(posterPath)
      setImageError(false)
      return
    }

    // If it's "db", the image is stored in the database and needs auth
    if (posterPath === 'db' && movieId) {
      loadAuthenticatedImage(`/api/v1/movies/${movieId}/poster`)
        .then(url => {
          setImageUrl(url)
          setImageError(!url)
        })
        .catch(() => {
          setImageUrl(null)
          setImageError(true)
        })
      return
    }

    // If it starts with /uploads, it's a local file (legacy, no auth needed)
    if (posterPath.startsWith('/uploads')) {
      setImageUrl(posterPath)
      setImageError(false)
      return
    }

    // Otherwise it's a TMDB path (external, no auth needed)
    setImageUrl(`https://image.tmdb.org/t/p/${size}${posterPath}`)
    setImageError(false)

    // Cleanup blob URL on unmount or path change
    return () => {
      if (imageUrl && imageUrl.startsWith('blob:')) {
        URL.revokeObjectURL(imageUrl)
      }
    }
  }, [posterPath, movieId, size])

  if (!imageUrl || imageError) {
    return (
      <div className={`bg-muted flex items-center justify-center ${className}`}>
        <span className="text-muted-foreground text-sm">No image</span>
      </div>
    )
  }

  return (
    <img
      src={imageUrl}
      alt={alt}
      className={className}
      onError={() => setImageError(true)}
      loading="lazy"
    />
  )
}


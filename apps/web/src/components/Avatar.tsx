import { useMemo, useState, useEffect } from 'react'
import { User } from '@/lib/api'
import { loadAuthenticatedImage } from '@/lib/imageLoader'

interface AvatarProps {
  user: User | null
  size?: 'sm' | 'md' | 'lg' | 'xl'
  className?: string
}

const sizeClasses = {
  sm: 'h-5 w-5 text-xs',
  md: 'h-10 w-10 text-sm',
  lg: 'h-12 w-12 text-base',
  xl: 'h-16 w-16 text-lg',
}

export function Avatar({ user, size = 'md', className = '' }: AvatarProps) {
  if (!user) {
    return (
      <div
        className={`${sizeClasses[size]} rounded-full bg-muted flex items-center justify-center ${className}`}
      >
        <span className="text-muted-foreground">?</span>
      </div>
    )
  }

  // Get initials from username
  const getInitials = (username: string): string => {
    const parts = username.trim().split(/\s+/)
    if (parts.length >= 2) {
      return (parts[0][0] + parts[parts.length - 1][0]).toUpperCase()
    }
    if (username.length >= 2) {
      return username.substring(0, 2).toUpperCase()
    }
    return username.substring(0, 1).toUpperCase()
  }

  const initials = getInitials(user.username)

  // Get avatar URL - for "db" images, we need to load with authentication
  const avatarPath = useMemo(() => {
    const path = user.avatar_path
    if (!path) return null
    // If it starts with http, it's a full URL (external)
    if (path.startsWith('http')) return path
    // If it's "db", the image is stored in the database and needs auth
    if (path === 'db') {
      const timestamp = user.updated_at ? new Date(user.updated_at).getTime() : Date.now()
      return `/api/v1/auth/avatar/${user.id}?t=${timestamp}`
    }
    // If it starts with /uploads, it's a local file (legacy)
    if (path.startsWith('/uploads')) {
      const timestamp = user.updated_at ? new Date(user.updated_at).getTime() : Date.now()
      return `${path}?t=${timestamp}`
    }
    return null
  }, [user.avatar_path, user.updated_at, user.id])

  const [imageUrl, setImageUrl] = useState<string | null>(null)
  const [imageError, setImageError] = useState(false)

  useEffect(() => {
    if (!avatarPath) {
      setImageUrl(null)
      setImageError(false)
      return
    }

    // If it's an external URL, use it directly
    if (avatarPath.startsWith('http')) {
      setImageUrl(avatarPath)
      setImageError(false)
      return
    }

    // For authenticated images, load with fetch
    loadAuthenticatedImage(avatarPath)
      .then(url => {
        setImageUrl(url)
        setImageError(!url)
      })
      .catch(() => {
        setImageUrl(null)
        setImageError(true)
      })

    // Cleanup blob URL on unmount or path change
    return () => {
      if (imageUrl && imageUrl.startsWith('blob:')) {
        URL.revokeObjectURL(imageUrl)
      }
    }
  }, [avatarPath])

  if (imageUrl && !imageError) {
    return (
      <div className={`${sizeClasses[size]} rounded-full bg-transparent overflow-hidden ${className}`}>
        <img
          src={imageUrl}
          alt={user.username}
          className="h-full w-full rounded-full object-cover"
          onError={() => setImageError(true)}
        />
      </div>
    )
  }

  // Show initials
  return (
    <div
      className={`${sizeClasses[size]} rounded-full bg-primary text-primary-foreground flex items-center justify-center font-medium ${className}`}
      title={user.username}
    >
      {initials}
    </div>
  )
}


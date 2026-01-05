import { useMemo } from 'react'
import { User } from '@/lib/api'

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

  // Get avatar URL with cache-busting based on updated_at timestamp
  // This ensures the image reloads when the user is updated (e.g., after avatar upload)
  const avatarUrl = useMemo(() => {
    const avatarPath = user.avatar_path
    if (!avatarPath) return null
    // If it starts with http, it's a full URL
    if (avatarPath.startsWith('http')) return avatarPath
    // If it starts with /uploads, it's a local file
    // Add cache-busting parameter based on updated_at to force reload when user is updated
    if (avatarPath.startsWith('/uploads')) {
      // Use updated_at timestamp if available, otherwise use current time
      const timestamp = user.updated_at ? new Date(user.updated_at).getTime() : Date.now()
      return `${avatarPath}?t=${timestamp}`
    }
    return null
  }, [user.avatar_path, user.updated_at])

  if (avatarUrl) {
    return (
      <div className={`${sizeClasses[size]} rounded-full bg-transparent overflow-hidden ${className}`}>
        <img
          src={avatarUrl}
          alt={user.username}
          className="h-full w-full rounded-full object-cover"
          onError={(e) => {
            // Fallback to initials if image fails to load
            const target = e.target as HTMLImageElement
            target.style.display = 'none'
            const parent = target.parentElement
            if (parent) {
              const fallback = document.createElement('div')
              fallback.className = `${sizeClasses[size]} rounded-full bg-primary text-primary-foreground flex items-center justify-center font-medium`
              fallback.textContent = initials
              parent.appendChild(fallback)
            }
          }}
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


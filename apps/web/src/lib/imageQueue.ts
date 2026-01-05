// Queue for sequential image loading to avoid blocking the UI
class ImageLoadQueue {
  private queue: Array<{ url: string; resolve: (url: string | null) => void; reject: (error: Error) => void }> = []
  private loading = false
  private cache = new Map<string, string | null>() // Cache blob URLs by image URL
  private pending = new Map<string, Array<{ resolve: (url: string | null) => void; reject: (error: Error) => void }>>() // Track pending requests

  async load(url: string): Promise<string | null> {
    // Check cache first
    if (this.cache.has(url)) {
      return Promise.resolve(this.cache.get(url)!)
    }

    // Check if already loading this URL
    if (this.pending.has(url)) {
      return new Promise((resolve, reject) => {
        this.pending.get(url)!.push({ resolve, reject })
      })
    }

    // New request
    return new Promise((resolve, reject) => {
      this.pending.set(url, [{ resolve, reject }])
      this.queue.push({ url, resolve, reject })
      this.processQueue()
    })
  }

  // Invalidate cache for a specific movie poster
  invalidateMoviePoster(movieId: string) {
    const url = `/api/v1/movies/${movieId}/poster`
    const cached = this.cache.get(url)
    
    // Revoke blob URL if it exists
    if (cached && cached.startsWith('blob:')) {
      URL.revokeObjectURL(cached)
    }
    
    // Remove from cache
    this.cache.delete(url)
  }

  // Invalidate all cached images
  invalidateAll() {
    // Revoke all blob URLs
    for (const [_, blobUrl] of this.cache.entries()) {
      if (blobUrl && blobUrl.startsWith('blob:')) {
        URL.revokeObjectURL(blobUrl)
      }
    }
    this.cache.clear()
  }

  private async processQueue() {
    if (this.loading || this.queue.length === 0) {
      return
    }

    this.loading = true

    while (this.queue.length > 0) {
      const item = this.queue.shift()!
      
      // Skip if already cached
      if (this.cache.has(item.url)) {
        const cached = this.cache.get(item.url)!
        item.resolve(cached)
        // Resolve all pending requests for this URL
        const pending = this.pending.get(item.url)
        if (pending) {
          pending.forEach(p => p.resolve(cached))
          this.pending.delete(item.url)
        }
        continue
      }

      try {
        const token = localStorage.getItem('token')
        const response = await fetch(item.url, {
          headers: token ? { Authorization: `Bearer ${token}` } : {},
        })

        if (!response.ok) {
          // Cache null result to avoid retrying
          this.cache.set(item.url, null)
          item.resolve(null)
          // Resolve all pending requests for this URL
          const pending = this.pending.get(item.url)
          if (pending) {
            pending.forEach(p => p.resolve(null))
            this.pending.delete(item.url)
          }
          continue
        }

        const blob = await response.blob()
        const blobUrl = URL.createObjectURL(blob)
        // Cache the blob URL
        this.cache.set(item.url, blobUrl)
        item.resolve(blobUrl)
        // Resolve all pending requests for this URL
        const pending = this.pending.get(item.url)
        if (pending) {
          pending.forEach(p => p.resolve(blobUrl))
          this.pending.delete(item.url)
        }
      } catch (error) {
        // Cache error as null to avoid retrying
        this.cache.set(item.url, null)
        const err = error instanceof Error ? error : new Error('Failed to load image')
        item.reject(err)
        // Reject all pending requests for this URL
        const pending = this.pending.get(item.url)
        if (pending) {
          pending.forEach(p => p.reject(err))
          this.pending.delete(item.url)
        }
      }

      // Small delay between loads to avoid overwhelming the browser
      await new Promise(resolve => setTimeout(resolve, 50))
    }

    this.loading = false
  }
}

export const imageQueue = new ImageLoadQueue()


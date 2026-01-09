// Check if running in Tauri
const isTauri = typeof window !== 'undefined' && '__TAURI__' in window

// API base URL configuration (same as api.ts)
const API_BASE = isTauri
  ? 'http://127.0.0.1:3000/api/v1'
  : '/api/v1'

// Cache for Tauri fetch function
let tauriFetchFn: typeof fetch | null = null

// Use Tauri's fetch in Tauri production (bypasses WebView CORS restrictions)
async function tauriFetch(url: string, options?: RequestInit): Promise<Response> {
  // Only try Tauri fetch in Tauri production mode
  if (isTauri && import.meta.env.PROD) {
    try {
      // Cache the Tauri fetch function
      if (!tauriFetchFn) {
        const module = await import('@tauri-apps/plugin-http')
        tauriFetchFn = module.fetch as typeof fetch
      }
      return tauriFetchFn(url, options)
    } catch {
      // Plugin not available, fall back to regular fetch
      console.warn('Tauri HTTP plugin not available, using regular fetch')
    }
  }
  return fetch(url, options)
}

// Normalize URL to include base path if needed
function normalizeUrl(url: string): string {
  // If URL is already absolute, use it as-is
  if (url.startsWith('http://') || url.startsWith('https://')) {
    return url
  }
  // If URL starts with /api/v1, prepend base URL for Tauri
  if (url.startsWith('/api/v1')) {
    return isTauri ? `http://127.0.0.1:3000${url}` : url
  }
  // Otherwise, assume it's relative to API_BASE
  return `${API_BASE}${url.startsWith('/') ? url : '/' + url}`
}

// Queue for parallel image loading with concurrency limit
class ImageLoadQueue {
  private queue: Array<{ url: string; resolve: (url: string | null) => void; reject: (error: Error) => void }> = []
  private activeLoads = 0
  private maxConcurrent = 6 // Load up to 6 images in parallel
  private cache = new Map<string, string | null>() // Cache blob URLs by image URL
  private pending = new Map<string, Array<{ resolve: (url: string | null) => void; reject: (error: Error) => void }>>() // Track pending requests

  async load(url: string): Promise<string | null> {
    // Normalize URL for consistent caching
    const normalizedUrl = normalizeUrl(url)
    
    // Check cache first (using normalized URL as key)
    if (this.cache.has(normalizedUrl)) {
      return Promise.resolve(this.cache.get(normalizedUrl)!)
    }

    // Check if already loading this URL
    if (this.pending.has(normalizedUrl)) {
      return new Promise((resolve, reject) => {
        this.pending.get(normalizedUrl)!.push({ resolve, reject })
      })
    }

    // New request (store normalized URL)
    return new Promise((resolve, reject) => {
      this.pending.set(normalizedUrl, [{ resolve, reject }])
      this.queue.push({ url: normalizedUrl, resolve, reject })
      this.processQueue()
    })
  }

  // Invalidate cache for a specific movie poster
  invalidateMoviePoster(movieId: string) {
    const url = `/api/v1/movies/${movieId}/poster`
    const normalizedUrl = normalizeUrl(url)
    
    // Try both normalized and original URL (for backwards compatibility)
    const cached = this.cache.get(normalizedUrl) || this.cache.get(url)
    
    // Revoke blob URL if it exists
    if (cached && cached.startsWith('blob:')) {
      URL.revokeObjectURL(cached)
    }
    
    // Remove from cache (both versions)
    this.cache.delete(normalizedUrl)
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

  private processQueue() {
    // Start loading more items if we have capacity
    while (this.activeLoads < this.maxConcurrent && this.queue.length > 0) {
      const item = this.queue.shift()!
      
      // Skip if already cached (item.url is already normalized)
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

      this.activeLoads++
      this.loadImage(item)
    }
  }

  private async loadImage(item: { url: string; resolve: (url: string | null) => void; reject: (error: Error) => void }) {
    try {
      const token = localStorage.getItem('token')
      const response = await tauriFetch(item.url, {
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
      } else {
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
    } finally {
      this.activeLoads--
      // Process more items from queue
      this.processQueue()
    }
  }
}

export const imageQueue = new ImageLoadQueue()


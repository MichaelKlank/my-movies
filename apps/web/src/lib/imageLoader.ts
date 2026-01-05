import { api } from './api'

// Check if running in Tauri
const isTauri = typeof window !== 'undefined' && '__TAURI__' in window

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
  // Otherwise, assume it's relative to API base
  const API_BASE = isTauri ? 'http://127.0.0.1:3000/api/v1' : '/api/v1'
  return `${API_BASE}${url.startsWith('/') ? url : '/' + url}`
}

/**
 * Load an authenticated image and return a blob URL
 * This is needed because <img> tags cannot send Authorization headers
 */
export async function loadAuthenticatedImage(url: string): Promise<string | null> {
  try {
    const token = api.getToken()
    if (!token) {
      return null
    }

    const normalizedUrl = normalizeUrl(url)
    const response = await tauriFetch(normalizedUrl, {
      headers: {
        'Authorization': `Bearer ${token}`,
      },
    })

    if (!response.ok) {
      return null
    }

    const blob = await response.blob()
    return URL.createObjectURL(blob)
  } catch (error) {
    console.error('Failed to load authenticated image:', error)
    return null
  }
}


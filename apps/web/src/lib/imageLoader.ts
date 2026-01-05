import { api } from './api'

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

    const response = await fetch(url, {
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


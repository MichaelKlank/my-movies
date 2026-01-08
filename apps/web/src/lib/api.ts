// Check if running in Tauri
const isTauri = typeof window !== 'undefined' && '__TAURI__' in window

// API base URL configuration:
// - In Tauri: Use absolute URL to localhost (embedded server)
// - In web dev: Use relative URL (Vite proxy)
// - In web prod: Use relative URL (same-origin server)
const API_BASE = isTauri
  ? 'http://127.0.0.1:3000/api/v1'
  : '/api/v1'

// Cache for Tauri fetch function
let tauriFetchFn: typeof fetch | null = null

// Use Tauri's fetch in Tauri production (bypasses WebView CORS restrictions)
// Falls back to regular fetch in browser or if plugin not available
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

type RequestOptions = {
  method?: string
  body?: unknown
  headers?: Record<string, string>
}

class ApiClient {
  private token: string | null = null

  setToken(token: string | null) {
    this.token = token
    if (token) {
      localStorage.setItem('token', token)
      return
    }
    localStorage.removeItem('token')
  }

  getToken(): string | null {
    if (!this.token) {
      this.token = localStorage.getItem('token')
    }
    return this.token
  }

  private async request<T>(endpoint: string, options: RequestOptions = {}): Promise<T> {
    const { method = 'GET', body, headers = {} } = options

    const requestHeaders: Record<string, string> = {
      ...headers,
    }

    if (body && !(body instanceof FormData)) {
      requestHeaders['Content-Type'] = 'application/json'
    }

    const token = this.getToken()
    if (token) {
      requestHeaders['Authorization'] = `Bearer ${token}`
    }

    const response = await tauriFetch(`${API_BASE}${endpoint}`, {
      method,
      headers: requestHeaders,
      body: body instanceof FormData ? body : body ? JSON.stringify(body) : undefined,
    })

    if (!response.ok) {
      const error = await response.json().catch(() => ({ error: 'Unknown error' }))
      throw new Error(error.error || `HTTP ${response.status}`)
    }

    if (response.status === 204) {
      return undefined as T
    }

    return response.json()
  }

  // Auth
  async login(username: string, password: string) {
    const result = await this.request<{ token: string; user: User }>('/auth/login', {
      method: 'POST',
      body: { username, password },
    })
    this.setToken(result.token)
    return result
  }

  async register(username: string, email: string, password: string) {
    const result = await this.request<{ token: string; user: User }>('/auth/register', {
      method: 'POST',
      body: { username, email, password },
    })
    this.setToken(result.token)
    return result
  }

  async me() {
    return this.request<User>('/auth/me')
  }

  logout() {
    this.setToken(null)
  }

  // Password Reset
  async forgotPassword(email: string) {
    return this.request<{ message: string }>('/auth/forgot-password', {
      method: 'POST',
      body: { email },
    })
  }

  async resetPassword(token: string, password: string) {
    return this.request<{ message: string }>('/auth/reset-password', {
      method: 'POST',
      body: { token, password },
    })
  }

  // Movies
  async getMovies(params?: MovieFilter) {
    const query = params ? '?' + new URLSearchParams(
      Object.fromEntries(Object.entries(params).filter(([_, v]) => v !== undefined)) as Record<string, string>
    ).toString() : ''
    return this.request<PaginatedResponse<Movie>>(`/movies${query}`)
  }

  async getMovie(id: string) {
    return this.request<Movie>(`/movies/${id}`)
  }

  async createMovie(data: CreateMovie) {
    return this.request<Movie>('/movies', { method: 'POST', body: data })
  }

  async updateMovie(id: string, data: Partial<Movie>) {
    return this.request<Movie>(`/movies/${id}`, { method: 'PUT', body: data })
  }

  async deleteMovie(id: string) {
    return this.request<void>(`/movies/${id}`, { method: 'DELETE' })
  }

  async refreshMovieTmdb(id: string, force: boolean = false) {
    const url = force 
      ? `/movies/${id}/refresh-tmdb?force=true`
      : `/movies/${id}/refresh-tmdb`
    return this.request<Movie>(url, { method: 'POST' })
  }

  async uploadMoviePoster(id: string, file: File) {
    const formData = new FormData()
    formData.append('file', file)
    
    const token = this.getToken()
    const response = await tauriFetch(`${API_BASE}/movies/${id}/upload-poster`, {
      method: 'POST',
      headers: {
        ...(token ? { Authorization: `Bearer ${token}` } : {}),
      },
      body: formData,
    })
    
    if (!response.ok) {
      const error = await response.json().catch(() => ({ error: 'Upload failed' }))
      throw new Error(error.error || 'Upload failed')
    }
    
    return response.json() as Promise<{ message: string; movie: Movie }>
  }

  async setPosterFromUrl(id: string, url: string) {
    return this.request<{ message: string; movie: Movie }>(`/movies/${id}/set-poster-url`, {
      method: 'POST',
      body: { url },
    })
  }

  async checkMovieDuplicates(title: string, barcode?: string, tmdb_id?: number) {
    const params = new URLSearchParams({ title })
    if (barcode) params.set('barcode', barcode)
    if (tmdb_id) params.set('tmdb_id', tmdb_id.toString())
    return this.request<DuplicateCheckResult>(`/movies/check-duplicates?${params}`)
  }

  async findAllDuplicates() {
    return this.request<DuplicateGroupsResult>('/movies/duplicates')
  }

  // Series
  async getSeries(params?: SeriesFilter) {
    const query = params ? '?' + new URLSearchParams(params as Record<string, string>).toString() : ''
    return this.request<Series[]>(`/series${query}`)
  }

  async getSeriesById(id: string) {
    return this.request<Series>(`/series/${id}`)
  }

  async createSeries(data: CreateSeries) {
    return this.request<Series>('/series', { method: 'POST', body: data })
  }

  async updateSeries(id: string, data: Partial<Series>) {
    return this.request<Series>(`/series/${id}`, { method: 'PUT', body: data })
  }

  async deleteSeries(id: string) {
    return this.request<void>(`/series/${id}`, { method: 'DELETE' })
  }

  // Barcode & TMDB
  async lookupBarcode(barcode: string) {
    return this.request<BarcodeResult>('/scan', { method: 'POST', body: { barcode } })
  }

  async searchTmdbMovies(query: string, year?: number) {
    const params = new URLSearchParams({ query })
    if (year) params.set('year', year.toString())
    return this.request<TmdbSearchResult[]>(`/tmdb/search/movies?${params}`)
  }

  async searchTmdbTv(query: string) {
    return this.request<TmdbSearchResult[]>(`/tmdb/search/tv?query=${encodeURIComponent(query)}`)
  }

  async getTmdbMovie(id: number) {
    return this.request<TmdbMovieDetails>(`/tmdb/movies/${id}`)
  }

  async getTmdbTv(id: number) {
    return this.request<TmdbTvDetails>(`/tmdb/tv/${id}`)
  }

  // Import
  async importCsv(file: File) {
    const formData = new FormData()
    formData.append('file', file)
    return this.request<ImportResult>('/import/csv', { method: 'POST', body: formData })
  }

  async enrichMoviesTmdb(force: boolean = false) {
    const query = force ? '?force=true' : ''
    return this.request<EnrichResult>(`/import/enrich-tmdb${query}`, { method: 'POST' })
  }

  async cancelEnrichTmdb() {
    return this.request<{ message: string }>('/import/enrich-tmdb/cancel', { method: 'POST' })
  }

  // Settings (admin only)
  async getSettings() {
    return this.request<SettingStatus[]>('/settings')
  }

  async updateSetting(key: string, value: string) {
    return this.request<SettingStatus>(`/settings/${key}`, {
      method: 'PUT',
      body: { value },
    })
  }

  async testTmdb() {
    return this.request<TmdbTestResult>('/settings/test/tmdb', { method: 'POST' })
  }

  // User Management (admin only)
  async getUsers() {
    return this.request<UserWithDate[]>('/users')
  }

  async updateUserRole(userId: string, role: 'admin' | 'user') {
    return this.request<UserWithDate>(`/users/${userId}/role`, {
      method: 'PUT',
      body: { role },
    })
  }

  // User Settings
  async updateLanguage(language: string | null) {
    return this.request<User>('/auth/language', {
      method: 'PUT',
      body: { language },
    })
  }

  async updateIncludeAdult(includeAdult: boolean) {
    return this.request<User>('/auth/include-adult', {
      method: 'PUT',
      body: { include_adult: includeAdult },
    })
  }

  async uploadAvatar(file: File) {
    const formData = new FormData()
    formData.append('file', file)
    return this.request<{ message: string; user: User }>('/auth/avatar', {
      method: 'POST',
      body: formData,
    })
  }

  async deleteAvatar() {
    return this.request<{ message: string; user: User }>('/auth/avatar', {
      method: 'DELETE',
    })
  }

  async deleteUser(userId: string) {
    return this.request<{ message: string }>(`/users/${userId}`, { method: 'DELETE' })
  }

  async adminSetPassword(userId: string, password: string) {
    return this.request<{ message: string }>(`/users/${userId}/password`, {
      method: 'PUT',
      body: { password },
    })
  }

  async adminCreateUser(username: string, email: string, password?: string) {
    return this.request<{ user: UserWithDate; reset_token: string | null }>('/users', {
      method: 'POST',
      body: { username, email, password: password || null },
    })
  }
}

export const api = new ApiClient()

// Types
export interface User {
  id: string
  username: string
  email: string
  role: 'admin' | 'user'
  language?: string | null
  include_adult: boolean
  avatar_path?: string | null
  created_at: string
  updated_at: string
}

export interface Movie {
  id: string
  user_id: string
  title: string
  original_title?: string
  sort_title?: string
  barcode?: string
  tmdb_id?: number
  imdb_id?: string
  description?: string
  tagline?: string
  production_year?: number
  release_date?: string
  running_time?: number
  director?: string
  actors?: string
  genres?: string
  disc_type?: string
  watched: boolean
  personal_rating?: number
  location?: string
  notes?: string
  poster_path?: string
  edition?: string
  budget?: number
  revenue?: number
  created_at: string
  updated_at: string
}

export interface CreateMovie {
  title: string
  barcode?: string
  tmdb_id?: number
  original_title?: string
  disc_type?: string
  production_year?: number
  poster_path?: string
}

export interface MovieFilter {
  search?: string
  genre?: string
  disc_type?: string
  watched?: string
  sort_by?: string
  sort_order?: string
  limit?: string
  offset?: string
}

export interface Series {
  id: string
  user_id: string
  title: string
  original_title?: string
  barcode?: string
  tmdb_id?: number
  description?: string
  network?: string
  episodes_count?: number
  watched: boolean
  personal_rating?: number
  created_at: string
  updated_at: string
}

export interface CreateSeries {
  title: string
  barcode?: string
  tmdb_id?: number
  disc_type?: string
}

export interface SeriesFilter {
  search?: string
  genre?: string
  network?: string
  watched?: string
  limit?: string
  offset?: string
}

export interface BarcodeResult {
  barcode: string
  title?: string
  vendor?: string
  tmdb_results: TmdbSearchResult[]
}

export interface TmdbSearchResult {
  id: number
  title: string
  year?: string
  poster_url?: string
  poster_path?: string
}

export interface TmdbMovieDetails {
  id: number
  title: string
  original_title?: string
  tagline?: string
  overview?: string
  poster_path?: string
  release_date?: string
  runtime?: number
  vote_average?: number
  budget?: number
  revenue?: number
  imdb_id?: string
  genres?: { id: number; name: string }[]
  production_companies?: { id: number; name: string }[]
}

export interface TmdbTvDetails {
  id: number
  name: string
  original_name?: string
  tagline?: string
  overview?: string
  poster_path?: string
  first_air_date?: string
  number_of_episodes?: number
  number_of_seasons?: number
  status?: string
  networks?: { id: number; name: string }[]
  genres?: { id: number; name: string }[]
}

export interface ImportResult {
  movies_imported: number
  series_imported: number
  collections_imported: number
  errors: string[]
}

export interface EnrichResult {
  total: number
  enriched: number
  errors: string[]
}

export interface PaginatedResponse<T> {
  items: T[]
  total: number
  limit: number
  offset: number
}

export interface DuplicateCheckResult {
  has_duplicates: boolean
  duplicates: Movie[]
}

export interface DuplicateGroupsResult {
  duplicate_groups: Movie[][]
  total_groups: number
}

export interface SettingStatus {
  key: string
  env_var: string
  description: string
  is_configured: boolean
  source: 'environment' | 'database' | 'none'
  value_preview?: string
}

export interface TmdbTestResult {
  success: boolean
  message: string
}

export interface UserWithDate extends User {
  created_at: string
}

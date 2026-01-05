import { createFileRoute, redirect, useNavigate } from '@tanstack/react-router'
import { useState, useEffect, useRef } from 'react'
import { useMutation } from '@tanstack/react-query'
import { ScanLine, Keyboard, Search, Plus, X } from 'lucide-react'
import { api, BarcodeResult, TmdbSearchResult } from '@/lib/api'
import { browserScanner, isTauri, tauriScanner } from '@/lib/scanner'
import { useI18n } from '@/hooks/useI18n'

export const Route = createFileRoute('/scan')({
  beforeLoad: ({ context }) => {
    if (!context.auth.isAuthenticated) {
      throw redirect({ to: '/login' })
    }
  },
  component: ScanPage,
})

type ScanMode = 'camera' | 'manual' | 'search'

function ScanPage() {
  const { t } = useI18n()
  const [mode, setMode] = useState<ScanMode>('manual')
  const [barcode, setBarcode] = useState('')
  const [searchQuery, setSearchQuery] = useState('')
  const [scanResult, setScanResult] = useState<BarcodeResult | null>(null)
  const [searchResults, setSearchResults] = useState<TmdbSearchResult[]>([])
  const [isScanning, setIsScanning] = useState(false)
  const [error, setError] = useState('')
  const scannerRef = useRef<HTMLDivElement>(null)
  const navigate = useNavigate()

  // Barcode lookup mutation
  const lookupMutation = useMutation({
    mutationFn: (code: string) => api.lookupBarcode(code),
    onSuccess: (result) => {
      setScanResult(result)
      setError('')
    },
    onError: (err) => {
      setError(err instanceof Error ? err.message : t('scan.lookupFailed'))
    },
  })

  // TMDB search mutation
  const searchMutation = useMutation({
    mutationFn: (query: string) => api.searchTmdbMovies(query),
    onSuccess: (results) => {
      setSearchResults(results)
      setError('')
    },
    onError: (err) => {
      setError(err instanceof Error ? err.message : t('scan.searchFailed'))
    },
  })

  // Create movie mutation
  const createMovieMutation = useMutation({
    mutationFn: (tmdbResult: TmdbSearchResult) => 
      api.createMovie({
        title: tmdbResult.title,
        tmdb_id: tmdbResult.id,
        production_year: tmdbResult.year ? parseInt(tmdbResult.year) : undefined,
        barcode: scanResult?.barcode,
        poster_path: tmdbResult.poster_path,
      }),
    onSuccess: (movie) => {
      // WebSocket event will handle cache invalidation
      navigate({ to: '/movies/$movieId', params: { movieId: movie.id } })
    },
    onError: (err) => {
      setError(err instanceof Error ? err.message : t('scan.createFailed'))
    },
  })

  // Camera scanning setup
  useEffect(() => {
    if (mode === 'camera' && scannerRef.current) {
      setIsScanning(true)
      
      if (isTauri()) {
        // Use Tauri native scanner
        tauriScanner.scan()
          .then((result) => {
            setBarcode(result.barcode)
            lookupMutation.mutate(result.barcode)
            setMode('manual')
          })
          .catch((err) => {
            setError(err.message)
          })
          .finally(() => {
            setIsScanning(false)
          })
      } else {
        // Use browser scanner
        browserScanner.start(
          (result) => {
            setBarcode(result.barcode)
            lookupMutation.mutate(result.barcode)
            browserScanner.stop()
            setIsScanning(false)
            setMode('manual')
          },
          (err) => {
            if (!err.message.includes('No MultiFormat')) {
              console.error('Scanner error:', err)
            }
          }
        ).catch((err) => {
          setError(err.message || t('scan.cameraFailed'))
          setIsScanning(false)
          setMode('manual')
        })
      }
    }

    return () => {
      if (mode === 'camera' && !isTauri()) {
        browserScanner.stop()
      }
    }
  }, [mode])

  const handleManualLookup = (e: React.FormEvent) => {
    e.preventDefault()
    if (barcode.trim()) {
      lookupMutation.mutate(barcode.trim())
    }
  }

  const handleSearch = (e: React.FormEvent) => {
    e.preventDefault()
    if (searchQuery.trim()) {
      searchMutation.mutate(searchQuery.trim())
    }
  }

  return (
    <div className="space-y-6">
      <h1 className="text-2xl font-bold">{t('scan.title')}</h1>

      {/* Mode selector */}
      <div className="flex gap-2">
        <button
          onClick={() => setMode('camera')}
          className={`flex items-center gap-2 rounded-md px-4 py-2 text-sm ${
            mode === 'camera' ? 'bg-primary text-primary-foreground' : 'bg-secondary hover:bg-secondary/80'
          }`}
        >
          <ScanLine className="h-4 w-4" />
          {t('scan.camera')}
        </button>
        <button
          onClick={() => setMode('manual')}
          className={`flex items-center gap-2 rounded-md px-4 py-2 text-sm ${
            mode === 'manual' ? 'bg-primary text-primary-foreground' : 'bg-secondary hover:bg-secondary/80'
          }`}
        >
          <Keyboard className="h-4 w-4" />
          {t('scan.enterBarcode')}
        </button>
        <button
          onClick={() => setMode('search')}
          className={`flex items-center gap-2 rounded-md px-4 py-2 text-sm ${
            mode === 'search' ? 'bg-primary text-primary-foreground' : 'bg-secondary hover:bg-secondary/80'
          }`}
        >
          <Search className="h-4 w-4" />
          {t('scan.searchTmdb')}
        </button>
      </div>

      {error && (
        <div className="rounded-md bg-destructive/10 p-3 text-sm text-destructive">
          {error}
        </div>
      )}

      {/* Camera mode */}
      {mode === 'camera' && (
        <div className="space-y-4">
          <div
            ref={scannerRef}
            id="scanner-container"
            className="aspect-video max-w-xl rounded-lg bg-muted overflow-hidden"
          />
          {isScanning && (
            <p className="text-center text-muted-foreground">
              {t('scan.holdBarcode')}
            </p>
          )}
          <button
            onClick={() => {
              browserScanner.stop()
              setIsScanning(false)
              setMode('manual')
            }}
            className="flex items-center gap-2 rounded-md bg-secondary px-4 py-2 text-sm hover:bg-secondary/80"
          >
            <X className="h-4 w-4" />
            {t('common.cancel')}
          </button>
        </div>
      )}

      {/* Manual barcode entry */}
      {mode === 'manual' && (
        <form onSubmit={handleManualLookup} className="space-y-4">
          <div className="flex gap-2">
            <input
              type="text"
              placeholder={t('scan.enterEanPlaceholder')}
              value={barcode}
              onChange={(e) => setBarcode(e.target.value)}
              className="flex-1 rounded-md border bg-background px-3 py-2 text-sm focus:border-primary focus:outline-none focus:ring-1 focus:ring-primary"
              autoFocus
            />
            <button
              type="submit"
              disabled={lookupMutation.isPending}
              className="rounded-md bg-primary px-4 py-2 text-sm text-primary-foreground hover:bg-primary/90 disabled:opacity-50"
            >
              {lookupMutation.isPending ? t('scan.searching') : t('common.search')}
            </button>
          </div>
        </form>
      )}

      {/* TMDB search */}
      {mode === 'search' && (
        <form onSubmit={handleSearch} className="space-y-4">
          <div className="flex gap-2">
            <input
              type="text"
              placeholder={t('scan.enterMovieTitle')}
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              className="flex-1 rounded-md border bg-background px-3 py-2 text-sm focus:border-primary focus:outline-none focus:ring-1 focus:ring-primary"
              autoFocus
            />
            <button
              type="submit"
              disabled={searchMutation.isPending}
              className="rounded-md bg-primary px-4 py-2 text-sm text-primary-foreground hover:bg-primary/90 disabled:opacity-50"
            >
              {searchMutation.isPending ? t('scan.searching') : t('common.search')}
            </button>
          </div>
        </form>
      )}

      {/* Barcode lookup results */}
      {scanResult && scanResult.tmdb_results.length > 0 && (
        <div className="space-y-4">
          <h2 className="font-semibold">
            {t('scan.foundForBarcode')}: {scanResult.barcode}
            {scanResult.title && <span className="text-muted-foreground"> ({scanResult.title})</span>}
          </h2>
          <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
            {scanResult.tmdb_results.map((result) => (
              <TmdbResultCard
                key={result.id}
                result={result}
                onSelect={() => createMovieMutation.mutate(result)}
                isLoading={createMovieMutation.isPending}
              />
            ))}
          </div>
        </div>
      )}

      {/* TMDB search results */}
      {searchResults.length > 0 && (
        <div className="space-y-4">
          <h2 className="font-semibold">{t('scan.searchResults')}</h2>
          <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
            {searchResults.map((result) => (
              <TmdbResultCard
                key={result.id}
                result={result}
                onSelect={() => createMovieMutation.mutate(result)}
                isLoading={createMovieMutation.isPending}
              />
            ))}
          </div>
        </div>
      )}
    </div>
  )
}

function TmdbResultCard({
  result,
  onSelect,
  isLoading,
}: {
  result: TmdbSearchResult
  onSelect: () => void
  isLoading: boolean
}) {
  const { t } = useI18n()
  return (
    <div className="rounded-lg border bg-card overflow-hidden">
      <div className="aspect-[2/3] bg-muted">
        {result.poster_url ? (
          <img
            src={result.poster_url}
            alt={result.title}
            className="h-full w-full object-cover"
          />
        ) : (
          <div className="h-full w-full flex items-center justify-center">
            <ScanLine className="h-8 w-8 text-muted-foreground" />
          </div>
        )}
      </div>
      <div className="p-3 space-y-2">
        <h3 className="font-medium">{result.title}</h3>
        {result.year && <p className="text-sm text-muted-foreground">{result.year}</p>}
        <button
          onClick={onSelect}
          disabled={isLoading}
          className="flex w-full items-center justify-center gap-2 rounded-md bg-primary px-3 py-2 text-sm text-primary-foreground hover:bg-primary/90 disabled:opacity-50"
        >
          <Plus className="h-4 w-4" />
          {t('movies.add')}
        </button>
      </div>
    </div>
  )
}

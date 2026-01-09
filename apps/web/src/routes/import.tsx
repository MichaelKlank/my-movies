import { PosterImage } from '@/components/PosterImage'
import { useI18n } from '@/hooks/useI18n'
import { api, ImportResult, Movie } from '@/lib/api'
import { wsClient, WsMessage } from '@/lib/ws'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { createFileRoute, Link, redirect } from '@tanstack/react-router'
import { AlertCircle, Check, ChevronDown, Copy, Download, FileUp, Image, RefreshCw, Trash2, Upload } from 'lucide-react'
import { useEffect, useRef, useState } from 'react'

export const Route = createFileRoute('/import')({
  beforeLoad: ({ context }) => {
    if (!context.auth.isAuthenticated) {
      throw redirect({ to: '/login' })
    }
  },
  component: ImportPage,
})

interface TmdbEnrichProgress {
  current: number
  total: number
  enriched: number
  errors_count: number
}

interface TmdbEnrichComplete {
  total: number
  enriched: number
  errors: string[]
}

function ImportPage() {
  const { t } = useI18n()
  const [selectedFile, setSelectedFile] = useState<File | null>(null)
  const [result, setResult] = useState<ImportResult | null>(null)
  const [enrichProgress, setEnrichProgress] = useState<TmdbEnrichProgress | null>(null)
  const [enrichComplete, setEnrichComplete] = useState<TmdbEnrichComplete | null>(null)
  const [isEnriching, setIsEnriching] = useState(false)
  const [showRefreshMenu, setShowRefreshMenu] = useState(false)
  const [autoEnrichTmdb, setAutoEnrichTmdb] = useState(() => {
    // Load preference from localStorage
    const saved = localStorage.getItem('autoEnrichTmdb')
    return saved !== null ? saved === 'true' : true // Default: enabled
  })
  const fileInputRef = useRef<HTMLInputElement>(null)
  const queryClient = useQueryClient()
  
  // Save preference when changed
  const handleAutoEnrichChange = (checked: boolean) => {
    setAutoEnrichTmdb(checked)
    localStorage.setItem('autoEnrichTmdb', String(checked))
  }

  // Check if TMDB API key is configured (admin only, fails silently for non-admins)
  const { data: settings } = useQuery({
    queryKey: ['settings'],
    queryFn: () => api.getSettings(),
    retry: false, // Don't retry if forbidden (non-admin)
  })
  // Only disable if we explicitly know it's NOT configured
  const tmdbSetting = settings?.find(s => s.key === 'tmdb_api_key')
  const tmdbApiConfigured = !settings ? true : (tmdbSetting?.is_configured ?? true)

  // Check if enrichment is already running (on page load/reload)
  useEffect(() => {
    const checkStatus = async () => {
      try {
        const status = await api.getEnrichStatus()
        if (status.is_running) {
          setIsEnriching(true)
          setEnrichProgress({
            current: status.current ?? 0,
            total: status.total ?? 0,
            enriched: status.updated ?? 0,
            errors_count: status.errors_count ?? 0,
          })
          setEnrichComplete(null)
        }
      } catch (e) {
        // Ignore errors - status check is optional
        console.debug('Could not check enrich status:', e)
      }
    }
    checkStatus()
  }, [])

  // Subscribe to WebSocket events for TMDB enrichment
  useEffect(() => {
    const unsubscribe = wsClient.subscribe((message: WsMessage) => {
      switch (message.type) {
        case 'tmdb_enrich_started':
          setIsEnriching(true)
          setEnrichProgress({ current: 0, total: (message.payload as { total: number }).total, enriched: 0, errors_count: 0 })
          setEnrichComplete(null)
          break
        case 'tmdb_enrich_progress':
          setEnrichProgress(message.payload as TmdbEnrichProgress)
          break
        case 'tmdb_enrich_complete':
          setIsEnriching(false)
          setEnrichProgress(null)
          setEnrichComplete(message.payload as TmdbEnrichComplete)
          queryClient.invalidateQueries({ queryKey: ['movies'] })
          break
        case 'tmdb_enrich_cancelled':
          setIsEnriching(false)
          setEnrichProgress(null)
          setEnrichComplete({
            total: (message.payload as { total: number }).total,
            enriched: (message.payload as { enriched: number }).enriched,
            errors: [t('import.cancelled')]
          })
          queryClient.invalidateQueries({ queryKey: ['movies'] })
          break
      }
    })

    return unsubscribe
  }, [queryClient, t])

  const importMutation = useMutation({
    mutationFn: (file: File) => api.importCsv(file),
    onSuccess: (data) => {
      setResult(data)
      setSelectedFile(null)
      setEnrichComplete(null)
      setEnrichProgress(null)
      // WebSocket event (collection_imported) will handle cache invalidation
      
      // Auto-trigger TMDB enrichment if enabled and there are imported items
      const totalImported = data.movies_imported + data.series_imported + data.collections_imported
      if (autoEnrichTmdb && totalImported > 0 && tmdbApiConfigured) {
        // Small delay to let the UI update first
        setTimeout(() => {
          enrichMutation.mutate(false) // false = only missing data
        }, 500)
      }
    },
  })

  const enrichMutation = useMutation({
    mutationFn: (force: boolean) => api.enrichMoviesTmdb(force),
    onSuccess: () => {
      // Status wird über WebSocket aktualisiert
    },
    onError: () => {
      setIsEnriching(false)
    }
  })

  const cancelMutation = useMutation({
    mutationFn: () => api.cancelEnrichTmdb(),
  })

  const handleFileSelect = (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0]
    if (file) {
      setSelectedFile(file)
      setResult(null)
      setEnrichComplete(null)
    }
  }

  const handleImport = () => {
    if (selectedFile) {
      importMutation.mutate(selectedFile)
    }
  }

  const progressPercent = enrichProgress 
    ? Math.round((enrichProgress.current / enrichProgress.total) * 100) 
    : 0

  return (
    <div className="space-y-4 md:space-y-6 max-w-2xl">
      <h1 className="text-xl md:text-2xl font-bold">{t('import.title')}</h1>

      <div className="rounded-lg border bg-card p-4 md:p-6 space-y-4">
        <div className="flex items-start gap-4">
          <Upload className="h-8 w-8 text-muted-foreground shrink-0 mt-1" />
          <div className="flex-1">
            <h2 className="font-semibold">CSV Import (Neue Sammlung)</h2>
            <p className="mt-1 text-sm text-muted-foreground">
              Importiere Filme aus einer CSV-Datei (z.B. von DVD Profiler oder anderen Programmen).
            </p>
          </div>
        </div>

        <input
          ref={fileInputRef}
          type="file"
          accept=".csv"
          onChange={handleFileSelect}
          className="hidden"
        />

        <div className="flex flex-col gap-3">
          <button
            onClick={() => fileInputRef.current?.click()}
            className="flex items-center justify-center gap-2 rounded-md border border-dashed bg-background px-4 py-6 text-sm hover:border-primary hover:bg-accent active:bg-accent/80 min-h-touch"
          >
            <FileUp className="h-5 w-5" />
            <span className="truncate">{selectedFile ? selectedFile.name : 'CSV-Datei auswählen'}</span>
          </button>

          {selectedFile && (
            <>
              {/* Auto-TMDB Checkbox */}
              <label className="flex items-center gap-3 p-3 rounded-md bg-muted/50 cursor-pointer hover:bg-muted/70">
                <input
                  type="checkbox"
                  checked={autoEnrichTmdb}
                  onChange={(e) => handleAutoEnrichChange(e.target.checked)}
                  disabled={!tmdbApiConfigured}
                  className="h-4 w-4 rounded border-gray-300 text-primary focus:ring-primary"
                />
                <div className="flex-1">
                  <span className="text-sm font-medium">TMDB Daten automatisch laden</span>
                  <p className="text-xs text-muted-foreground">
                    Nach dem Import werden Poster und Details automatisch von TMDB geladen
                  </p>
                </div>
              </label>
              
              <button
                onClick={handleImport}
                disabled={importMutation.isPending}
                className="flex items-center justify-center gap-2 rounded-md bg-primary px-4 py-3 text-sm text-primary-foreground hover:bg-primary/90 active:bg-primary/80 disabled:opacity-50 min-h-touch"
              >
                {importMutation.isPending ? t('import.importing') : t('import.title')}
              </button>
            </>
          )}
        </div>

        {importMutation.isError && (
          <div className="flex items-start gap-2 rounded-md bg-destructive/10 p-3 text-sm text-destructive">
            <AlertCircle className="h-4 w-4 mt-0.5 shrink-0" />
            <span>{importMutation.error instanceof Error ? importMutation.error.message : t('import.importFailed')}</span>
          </div>
        )}

        {result && (
          <div className="space-y-3">
            <div className="flex items-start gap-2 rounded-md bg-green-500/10 p-3 text-sm text-green-700">
              <Check className="h-4 w-4 mt-0.5 shrink-0" />
              <div>
                <p className="font-medium">{t('import.importSuccess')}</p>
                <ul className="mt-1 text-muted-foreground">
                  <li>{result.movies_imported} {t('movies.title')} {t('import.imported')}</li>
                  <li>{result.series_imported} {t('series.title')} {t('import.imported')}</li>
                  <li>{result.collections_imported} {t('collections.title', { defaultValue: 'Collections' })} {t('import.imported')}</li>
                </ul>
              </div>
            </div>

            {result.errors.length > 0 && (
              <div className="rounded-md bg-yellow-500/10 p-3 text-sm">
                <p className="font-medium text-yellow-700">
                  {result.errors.length} {t('import.errors')}:
                </p>
                <ul className="mt-2 max-h-40 overflow-auto text-xs text-muted-foreground">
                  {result.errors.slice(0, 10).map((error, i) => (
                    <li key={i}>{error}</li>
                  ))}
                  {result.errors.length > 10 && (
                    <li>... {t('import.andMore')} {result.errors.length - 10} {t('import.more')}</li>
                  )}
                </ul>
              </div>
            )}
          </div>
        )}
      </div>

      {/* ZIP Import Section (Backup Restore) */}
      <ZipImportSection autoEnrichTmdb={autoEnrichTmdb} tmdbApiConfigured={tmdbApiConfigured} />

      {/* TMDB Enrichment Section */}
      <div className={`rounded-lg border bg-card p-6 space-y-4 ${!tmdbApiConfigured ? 'opacity-60' : ''}`}>
        <div className="flex items-start gap-4">
          <Image className="h-8 w-8 text-muted-foreground shrink-0 mt-1" />
          <div className="flex-1">
            <h2 className="font-semibold">{t('import.loadTmdbData')}</h2>
            <p className="mt-1 text-sm text-muted-foreground">
              {t('import.loadTmdbDataDesc')}
            </p>
            {!tmdbApiConfigured && (
              <p className="mt-2 text-sm text-yellow-600">
                <Link to="/settings" className="underline hover:no-underline">
                  {t('settings.tmdbApiKey')}
                </Link> {t('users.notConfigured').toLowerCase()}
              </p>
            )}
          </div>
        </div>

        {/* Dropdown Menu Button */}
        <div className="relative">
          <button
            onClick={() => setShowRefreshMenu(!showRefreshMenu)}
            disabled={isEnriching || enrichMutation.isPending || !tmdbApiConfigured}
            className="flex items-center justify-center gap-2 w-full rounded-md bg-secondary px-4 py-3 text-sm font-medium hover:bg-secondary/80 active:bg-secondary/60 disabled:opacity-50 min-h-touch"
          >
            <RefreshCw className={`h-4 w-4 ${isEnriching ? 'animate-spin' : ''}`} />
            {isEnriching ? t('import.loadingTmdbData') : t('import.loadTmdbData')}
            <ChevronDown className={`h-4 w-4 transition-transform ${showRefreshMenu ? 'rotate-180' : ''}`} />
          </button>

          {/* Desktop dropdown */}
          {showRefreshMenu && (
            <div className="hidden sm:block absolute top-full left-0 right-0 mt-1 bg-card border rounded-md shadow-lg z-10">
              <button
                onClick={() => {
                  enrichMutation.mutate(false)
                  setShowRefreshMenu(false)
                }}
                disabled={enrichMutation.isPending}
                className="w-full text-left px-4 py-3 text-sm hover:bg-muted active:bg-muted/80 transition-colors min-h-touch"
              >
                {t('import.loadMissingData')}
              </button>
              <button
                onClick={() => {
                  enrichMutation.mutate(true)
                  setShowRefreshMenu(false)
                }}
                disabled={enrichMutation.isPending}
                className="w-full text-left px-4 py-3 text-sm hover:bg-muted active:bg-muted/80 transition-colors border-t min-h-touch"
              >
                {t('import.reloadAllData')}
              </button>
            </div>
          )}
        </div>

        {/* Mobile dialog for TMDB refresh */}
        {showRefreshMenu && (
          <div 
            className="sm:hidden fixed inset-0 bg-black/50 z-50"
            onClick={() => setShowRefreshMenu(false)}
          >
            <div 
              className="fixed left-4 right-4 top-1/2 -translate-y-1/2 bg-card rounded-xl shadow-2xl overflow-hidden"
              onClick={(e) => e.stopPropagation()}
            >
              <div className="px-4 py-3 border-b bg-muted/50">
                <h3 className="text-sm font-semibold text-center">
                  {t('import.loadTmdbData')}
                </h3>
              </div>
              <button
                onClick={() => {
                  enrichMutation.mutate(false)
                  setShowRefreshMenu(false)
                }}
                disabled={enrichMutation.isPending}
                className="w-full text-left px-4 py-4 text-sm hover:bg-muted active:bg-muted/80 transition-colors min-h-touch"
              >
                {t('import.loadMissingData')}
              </button>
              <button
                onClick={() => {
                  enrichMutation.mutate(true)
                  setShowRefreshMenu(false)
                }}
                disabled={enrichMutation.isPending}
                className="w-full text-left px-4 py-4 text-sm hover:bg-muted active:bg-muted/80 transition-colors border-t min-h-touch"
              >
                {t('import.reloadAllData')}
              </button>
              <button
                onClick={() => setShowRefreshMenu(false)}
                className="w-full text-center px-4 py-4 text-sm font-medium text-destructive hover:bg-muted active:bg-muted/80 transition-colors border-t min-h-touch"
              >
                {t('common.cancel')}
              </button>
            </div>
          </div>
        )}

        {/* Progress Bar */}
        {enrichProgress && (
          <div className="space-y-3">
            <div className="space-y-2">
              <div className="flex justify-between text-sm">
                <span className="text-muted-foreground">
                  {enrichProgress.current} {t('import.moviesProcessed')} {enrichProgress.total}
                </span>
                <span className="font-medium">{progressPercent}%</span>
              </div>
              <div className="h-2 rounded-full bg-muted overflow-hidden">
                <div 
                  className="h-full bg-primary transition-all duration-300"
                  style={{ width: `${progressPercent}%` }}
                />
              </div>
              <div className="flex gap-4 text-xs text-muted-foreground">
                <span className="text-green-600">{enrichProgress.enriched} {t('import.updated')}</span>
                {enrichProgress.errors_count > 0 && (
                  <span className="text-yellow-600">{enrichProgress.errors_count} {t('import.notFound')}</span>
                )}
              </div>
            </div>
            {/* Cancel Button */}
            <button
              onClick={() => cancelMutation.mutate()}
              disabled={cancelMutation.isPending}
              className="flex items-center justify-center gap-2 w-full rounded-md bg-destructive/10 text-destructive px-4 py-2 text-sm font-medium hover:bg-destructive/20 active:bg-destructive/30 disabled:opacity-50"
            >
              {cancelMutation.isPending ? t('common.loading') : t('common.cancel')}
            </button>
          </div>
        )}

        {enrichMutation.isError && !isEnriching && (
          <div className="flex items-start gap-2 rounded-md bg-destructive/10 p-3 text-sm text-destructive">
            <AlertCircle className="h-4 w-4 mt-0.5 shrink-0" />
            <span>{enrichMutation.error instanceof Error ? enrichMutation.error.message : t('import.errorLoadingTmdb')}</span>
          </div>
        )}

        {enrichComplete && (
          <div className="space-y-3">
            <div className="flex items-start gap-2 rounded-md bg-green-500/10 p-3 text-sm text-green-700">
              <Check className="h-4 w-4 mt-0.5 shrink-0" />
              <div>
                <p className="font-medium">{t('import.tmdbEnrichComplete')}</p>
                <p className="text-muted-foreground">
                  {enrichComplete.enriched} {t('import.moviesUpdated')} {enrichComplete.total}
                </p>
              </div>
            </div>

            {enrichComplete.errors.length > 0 && (
              <div className="rounded-md bg-yellow-500/10 p-3 text-sm">
                <p className="font-medium text-yellow-700">
                  {enrichComplete.errors.length} {t('import.moviesNotFound')}:
                </p>
                <ul className="mt-2 max-h-40 overflow-auto text-xs text-muted-foreground">
                  {enrichComplete.errors.slice(0, 20).map((error, i) => (
                    <li key={i}>{error}</li>
                  ))}
                  {enrichComplete.errors.length > 20 && (
                    <li>... {t('import.andMore')} {enrichComplete.errors.length - 20} {t('import.more')}</li>
                  )}
                </ul>
              </div>
            )}
          </div>
        )}
      </div>

      {/* Duplicates Section */}
      <DuplicatesSection />

      {/* Export Section */}
      <ExportSection />

      {/* Danger Zone - Delete All */}
      <DeleteAllSection />

      <div className="rounded-lg border bg-card p-6">
        <h3 className="font-semibold">{t('import.supportedFormats')}</h3>
        <p className="mt-2 text-sm text-muted-foreground">
          {t('import.csvColumnsDesc')}
        </p>
        <ul className="mt-2 text-sm text-muted-foreground list-disc list-inside">
          <li>{t('import.csvColumns')}</li>
        </ul>
      </div>
    </div>
  )
}

function ZipImportSection({ autoEnrichTmdb, tmdbApiConfigured }: { autoEnrichTmdb: boolean; tmdbApiConfigured: boolean }) {
  const [selectedFile, setSelectedFile] = useState<File | null>(null)
  const [importResult, setImportResult] = useState<{ imported: number; skipped: number; posters_restored: number; errors: string[] } | null>(null)
  const fileInputRef = useRef<HTMLInputElement>(null)
  const queryClient = useQueryClient()

  const importMutation = useMutation({
    mutationFn: (file: File) => api.importZip(file),
    onSuccess: (data) => {
      setImportResult(data)
      setSelectedFile(null)
      queryClient.invalidateQueries({ queryKey: ['movies'] })
      
      // Auto-trigger TMDB enrichment if enabled (only for movies without poster)
      if (autoEnrichTmdb && data.imported > 0 && tmdbApiConfigured && data.posters_restored < data.imported) {
        setTimeout(() => {
          api.enrichMoviesTmdb(false)
        }, 500)
      }
    },
  })

  const handleFileSelect = (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0]
    if (file) {
      setSelectedFile(file)
      setImportResult(null)
    }
  }

  return (
    <div className="rounded-lg border bg-card p-6 space-y-4">
      <div className="flex items-start gap-4">
        <FileUp className="h-8 w-8 text-muted-foreground shrink-0 mt-1" />
        <div className="flex-1">
          <h2 className="font-semibold">ZIP Import (Backup wiederherstellen)</h2>
          <p className="mt-1 text-sm text-muted-foreground">
            Importiere ein zuvor exportiertes ZIP-Backup. Alle Metadaten und Poster werden wiederhergestellt.
            Bereits vorhandene Filme (nach Barcode) werden übersprungen.
          </p>
        </div>
      </div>

      <input
        ref={fileInputRef}
        type="file"
        accept=".zip"
        onChange={handleFileSelect}
        className="hidden"
      />

      <div className="flex flex-col gap-3">
        <button
          onClick={() => fileInputRef.current?.click()}
          className="flex items-center justify-center gap-2 rounded-md border border-dashed bg-background px-4 py-8 text-sm hover:border-primary hover:bg-accent active:bg-accent/80 min-h-touch"
        >
          <FileUp className="h-5 w-5" />
          <span className="truncate">{selectedFile ? selectedFile.name : 'ZIP-Datei auswählen'}</span>
        </button>

        {selectedFile && (
          <button
            onClick={() => importMutation.mutate(selectedFile)}
            disabled={importMutation.isPending}
            className="flex items-center justify-center gap-2 rounded-md bg-primary px-4 py-3 text-sm text-primary-foreground hover:bg-primary/90 active:bg-primary/80 disabled:opacity-50 min-h-touch"
          >
            {importMutation.isPending ? (
              <>
                <RefreshCw className="h-4 w-4 animate-spin" />
                Importiere...
              </>
            ) : (
              <>
                <Upload className="h-4 w-4" />
                ZIP importieren
              </>
            )}
          </button>
        )}
      </div>

      {importMutation.isError && (
        <div className="flex items-start gap-2 rounded-md bg-destructive/10 p-3 text-sm text-destructive">
          <AlertCircle className="h-4 w-4 mt-0.5 shrink-0" />
          <span>{importMutation.error instanceof Error ? importMutation.error.message : 'Import fehlgeschlagen'}</span>
        </div>
      )}

      {importResult && (
        <div className="space-y-3">
          <div className="flex items-start gap-2 rounded-md bg-green-500/10 p-3 text-sm text-green-700">
            <Check className="h-4 w-4 mt-0.5 shrink-0" />
            <div>
              <p className="font-medium">Import abgeschlossen</p>
              <ul className="mt-1 text-muted-foreground">
                <li>{importResult.imported} Filme importiert</li>
                <li>{importResult.posters_restored} Poster wiederhergestellt</li>
                <li>{importResult.skipped} übersprungen (bereits vorhanden)</li>
              </ul>
            </div>
          </div>

          {importResult.errors.length > 0 && (
            <div className="rounded-md bg-yellow-500/10 p-3 text-sm">
              <p className="font-medium text-yellow-700">
                {importResult.errors.length} Fehler:
              </p>
              <ul className="mt-2 max-h-40 overflow-auto text-xs text-muted-foreground">
                {importResult.errors.slice(0, 10).map((error, i) => (
                  <li key={i}>{error}</li>
                ))}
                {importResult.errors.length > 10 && (
                  <li>... und {importResult.errors.length - 10} weitere</li>
                )}
              </ul>
            </div>
          )}
        </div>
      )}
    </div>
  )
}

function ExportSection() {
  const [isExporting, setIsExporting] = useState(false)

  const { data: moviesResponse } = useQuery({
    queryKey: ['movies'],
    queryFn: () => api.getMovies(),
  })

  const movieCount = moviesResponse?.total ?? 0

  const handleExport = async () => {
    setIsExporting(true)
    try {
      const blob = await api.exportMovies()
      const url = URL.createObjectURL(blob)
      const a = document.createElement('a')
      a.href = url
      a.download = `my-movies-backup-${new Date().toISOString().slice(0, 10)}.zip`
      document.body.appendChild(a)
      a.click()
      document.body.removeChild(a)
      URL.revokeObjectURL(url)
    } catch (error) {
      console.error('Export failed:', error)
      alert('Export fehlgeschlagen')
    } finally {
      setIsExporting(false)
    }
  }

  return (
    <div className="rounded-lg border bg-card p-6 space-y-4">
      <div className="flex items-start gap-4">
        <Download className="h-8 w-8 text-muted-foreground shrink-0 mt-1" />
        <div className="flex-1">
          <h2 className="font-semibold">Sammlung exportieren (Vollständiges Backup)</h2>
          <p className="mt-1 text-sm text-muted-foreground">
            Exportiere deine gesamte Filmsammlung als ZIP-Datei. Alle Metadaten und Poster werden gesichert
            und können vollständig wiederhergestellt werden.
          </p>
        </div>
      </div>

      <button
        onClick={handleExport}
        disabled={isExporting || movieCount === 0}
        className="flex items-center justify-center gap-2 w-full rounded-md bg-secondary px-4 py-3 text-sm font-medium hover:bg-secondary/80 active:bg-secondary/60 disabled:opacity-50 min-h-touch"
      >
        {isExporting ? (
          <>
            <RefreshCw className="h-4 w-4 animate-spin" />
            Exportiere...
          </>
        ) : (
          <>
            <Download className="h-4 w-4" />
            Als ZIP exportieren ({movieCount} Filme)
          </>
        )}
      </button>
    </div>
  )
}

function DeleteAllSection() {
  const { t } = useI18n()
  const [showConfirm, setShowConfirm] = useState(false)
  const [confirmText, setConfirmText] = useState('')
  const queryClient = useQueryClient()

  const { data: moviesResponse } = useQuery({
    queryKey: ['movies'],
    queryFn: () => api.getMovies(),
  })

  const movieCount = moviesResponse?.total ?? 0

  const deleteAllMutation = useMutation({
    mutationFn: () => api.deleteAllMovies(),
    onSuccess: (data) => {
      setShowConfirm(false)
      setConfirmText('')
      queryClient.invalidateQueries({ queryKey: ['movies'] })
      queryClient.invalidateQueries({ queryKey: ['duplicates'] })
      alert(t('import.deleteAll.success', { count: data.deleted }))
    },
  })

  const confirmWord = t('import.deleteAll.confirmWord')
  const canDelete = confirmText.toLowerCase() === confirmWord.toLowerCase()

  return (
    <div className="rounded-lg border border-destructive/30 bg-card p-6 space-y-4">
      <div className="flex items-start gap-4">
        <Trash2 className="h-8 w-8 text-destructive shrink-0 mt-1" />
        <div className="flex-1">
          <h2 className="font-semibold text-destructive">{t('import.deleteAll.title')}</h2>
          <p className="mt-1 text-sm text-muted-foreground">
            {t('import.deleteAll.description')}
          </p>
        </div>
      </div>

      {!showConfirm ? (
        <button
          onClick={() => setShowConfirm(true)}
          disabled={movieCount === 0}
          className="flex items-center justify-center gap-2 w-full rounded-md bg-destructive/10 text-destructive px-4 py-3 text-sm font-medium hover:bg-destructive/20 active:bg-destructive/30 disabled:opacity-50 min-h-touch"
        >
          <Trash2 className="h-4 w-4" />
          {t('import.deleteAll.button', { count: movieCount })}
        </button>
      ) : (
        <div className="space-y-3 p-4 rounded-md bg-destructive/5 border border-destructive/20">
          <p className="text-sm font-medium text-destructive">
            {t('import.deleteAll.confirmQuestion', { count: movieCount })}
          </p>
          <p className="text-xs text-muted-foreground">
            {t('import.deleteAll.typeToConfirm', { word: confirmWord })}
          </p>
          <input
            type="text"
            value={confirmText}
            onChange={(e) => setConfirmText(e.target.value)}
            placeholder={confirmWord}
            className="w-full px-3 py-2 text-sm rounded-md border bg-background focus:outline-none focus:ring-2 focus:ring-destructive"
            autoFocus
          />
          <div className="flex gap-2">
            <button
              onClick={() => {
                setShowConfirm(false)
                setConfirmText('')
              }}
              className="flex-1 px-4 py-2 text-sm rounded-md bg-secondary hover:bg-secondary/80"
            >
              {t('common.cancel')}
            </button>
            <button
              onClick={() => deleteAllMutation.mutate()}
              disabled={!canDelete || deleteAllMutation.isPending}
              className="flex-1 flex items-center justify-center gap-2 px-4 py-2 text-sm rounded-md bg-destructive text-destructive-foreground hover:bg-destructive/90 disabled:opacity-50"
            >
              {deleteAllMutation.isPending ? (
                <RefreshCw className="h-4 w-4 animate-spin" />
              ) : (
                <Trash2 className="h-4 w-4" />
              )}
              {t('import.deleteAll.confirmButton')}
            </button>
          </div>
        </div>
      )}

      {deleteAllMutation.isError && (
        <div className="flex items-start gap-2 rounded-md bg-destructive/10 p-3 text-sm text-destructive">
          <AlertCircle className="h-4 w-4 mt-0.5 shrink-0" />
          <span>{deleteAllMutation.error instanceof Error ? deleteAllMutation.error.message : t('import.deleteAll.error')}</span>
        </div>
      )}
    </div>
  )
}

function DuplicatesSection() {
  const { t } = useI18n()
  const [showDuplicates, setShowDuplicates] = useState(false)
  const [selectedIds, setSelectedIds] = useState<Set<string>>(new Set())
  // Track which movie to keep in each group (by group index -> movie id)
  const [keepIds, setKeepIds] = useState<Map<number, string>>(new Map())
  const [isDeleting, setIsDeleting] = useState(false)
  const queryClient = useQueryClient()

  const { data: duplicatesData, isLoading, refetch } = useQuery({
    queryKey: ['duplicates'],
    queryFn: () => api.findAllDuplicates(),
    enabled: showDuplicates,
    staleTime: 0, // Always refetch when enabled
  })

  const deleteMutation = useMutation({
    mutationFn: (id: string) => api.deleteMovie(id),
    onSuccess: () => {
      // WebSocket event (movie_deleted) will handle cache invalidation for ['movies']
      // Duplicates query needs manual invalidation as it's not handled by WebSocket
      queryClient.invalidateQueries({ queryKey: ['duplicates'] })
    },
  })

  const duplicateGroups = duplicatesData?.duplicate_groups ?? []

  // Get which movie to keep for a group (default: first one)
  const getKeepIdForGroup = (groupIndex: number, group: Movie[]) => {
    return keepIds.get(groupIndex) || group[0]?.id
  }

  // Get all IDs to delete (all except the "keep" one in each group)
  const allDuplicateIds = duplicateGroups.flatMap((group, groupIndex) => {
    const keepId = getKeepIdForGroup(groupIndex, group)
    return group.filter((m: Movie) => m.id !== keepId).map((m: Movie) => m.id)
  })

  const toggleSelection = (id: string) => {
    setSelectedIds(prev => {
      const next = new Set(prev)
      if (next.has(id)) {
        next.delete(id)
      } else {
        next.add(id)
      }
      return next
    })
  }

  const selectAllDuplicates = () => {
    // Select all movies except the ones we're keeping
    setSelectedIds(new Set(allDuplicateIds))
  }

  const clearSelection = () => {
    setSelectedIds(new Set())
    setKeepIds(new Map()) // Reset keep selections too
  }

  const deleteSelected = async () => {
    if (selectedIds.size === 0) return
    
    const count = selectedIds.size
    if (!confirm(t('import.duplicates.confirmDeleteSelected', { count }))) return

    setIsDeleting(true)
    try {
      // Delete sequentially to avoid issues
      for (const id of selectedIds) {
        await api.deleteMovie(id)
      }
      setSelectedIds(new Set())
      // WebSocket events (movie_deleted) will handle cache invalidation for ['movies']
      // Duplicates query needs manual invalidation as it's not handled by WebSocket
      queryClient.invalidateQueries({ queryKey: ['duplicates'] })
    } finally {
      setIsDeleting(false)
    }
  }

  const deleteAllDuplicates = async () => {
    if (allDuplicateIds.length === 0) return
    
    const count = allDuplicateIds.length
    if (!confirm(t('import.duplicates.confirmDeleteAll', { count }))) return

    setIsDeleting(true)
    try {
      for (const id of allDuplicateIds) {
        await api.deleteMovie(id)
      }
      setSelectedIds(new Set())
      // WebSocket events (movie_deleted) will handle cache invalidation for ['movies']
      // Duplicates query needs manual invalidation as it's not handled by WebSocket
      queryClient.invalidateQueries({ queryKey: ['duplicates'] })
    } finally {
      setIsDeleting(false)
    }
  }

  return (
    <div className="rounded-lg border bg-card p-6 space-y-4">
      <div className="flex items-start gap-4">
        <Copy className="h-8 w-8 text-muted-foreground shrink-0 mt-1" />
        <div className="flex-1">
          <h2 className="font-semibold">{t('import.duplicates.title')}</h2>
          <p className="mt-1 text-sm text-muted-foreground">
            {t('import.duplicates.description')}
          </p>
        </div>
      </div>

      <button
        onClick={() => {
          setShowDuplicates(true)
          setSelectedIds(new Set())
          refetch()
        }}
        disabled={isLoading || isDeleting}
        className="flex items-center justify-center gap-2 w-full rounded-md bg-secondary px-4 py-3 text-sm font-medium hover:bg-secondary/80 disabled:opacity-50"
      >
        {isLoading ? (
          <>
            <RefreshCw className="h-4 w-4 animate-spin" />
            {t('import.duplicates.searching')}
          </>
        ) : (
          <>
            <Copy className="h-4 w-4" />
            {t('import.duplicates.search')}
          </>
        )}
      </button>

      {showDuplicates && !isLoading && (
        <div className="space-y-4">
          {duplicateGroups.length === 0 ? (
            <div className="flex items-start gap-2 rounded-md bg-green-500/10 p-3 text-sm text-green-700">
              <Check className="h-4 w-4 mt-0.5 shrink-0" />
              <span>{t('import.duplicates.noDuplicates')}</span>
            </div>
          ) : (
            <>
              <div className="flex items-center justify-between gap-4">
                <div className="flex items-start gap-2 rounded-md bg-yellow-500/10 p-3 text-sm text-yellow-700 flex-1">
                  <AlertCircle className="h-4 w-4 mt-0.5 shrink-0" />
                  <span>{t('import.duplicates.found', { groups: duplicateGroups.length, count: allDuplicateIds.length })}</span>
                </div>
              </div>

              {/* Action Bar */}
              <div className="flex flex-wrap items-center gap-2 p-3 rounded-md bg-muted/50">
                <button
                  onClick={selectAllDuplicates}
                  className="text-xs px-2 py-1 rounded bg-secondary hover:bg-secondary/80"
                >
                  {t('import.duplicates.selectAll', { count: allDuplicateIds.length })}
                </button>
                {selectedIds.size > 0 && (
                  <>
                    <button
                      onClick={clearSelection}
                      className="text-xs px-2 py-1 rounded bg-secondary hover:bg-secondary/80"
                    >
                      {t('import.duplicates.clearSelection')}
                    </button>
                    <span className="text-xs text-muted-foreground">
                      {t('import.duplicates.selected', { count: selectedIds.size })}
                    </span>
                  </>
                )}
                <div className="flex-1" />
                {selectedIds.size > 0 && (
                  <button
                    onClick={deleteSelected}
                    disabled={isDeleting}
                    className="flex items-center gap-1 text-xs px-3 py-1.5 rounded bg-destructive text-destructive-foreground hover:bg-destructive/90 disabled:opacity-50"
                  >
                    {isDeleting ? (
                      <RefreshCw className="h-3 w-3 animate-spin" />
                    ) : (
                      <Trash2 className="h-3 w-3" />
                    )}
                    {t('import.duplicates.deleteSelected', { count: selectedIds.size })}
                  </button>
                )}
                <button
                  onClick={deleteAllDuplicates}
                  disabled={isDeleting}
                  className="flex items-center gap-1 text-xs px-3 py-1.5 rounded bg-destructive text-destructive-foreground hover:bg-destructive/90 disabled:opacity-50"
                >
                  {isDeleting ? (
                    <RefreshCw className="h-3 w-3 animate-spin" />
                  ) : (
                    <Trash2 className="h-3 w-3" />
                  )}
                  {t('import.duplicates.deleteAll')}
                </button>
              </div>

              <div className="space-y-4 max-h-[500px] overflow-auto">
                {duplicateGroups.map((group, groupIndex) => {
                  const keepId = getKeepIdForGroup(groupIndex, group)
                  
                  return (
                    <div key={groupIndex} className="rounded-md border p-4 space-y-3">
                      <h4 className="font-medium text-sm">
                        {t('import.duplicates.group', { number: groupIndex + 1 })}: {group[0]?.title}
                        <span className="text-muted-foreground font-normal ml-2">
                          ({t('import.duplicates.entries', { count: group.length })})
                        </span>
                      </h4>
                      <div className="space-y-2">
                        {group.map((movie: Movie) => {
                          const isKeep = movie.id === keepId
                          const isSelected = selectedIds.has(movie.id)
                          
                          return (
                            <div
                              key={movie.id}
                              className={`flex items-center gap-3 p-2 rounded transition-colors ${
                                isSelected 
                                  ? 'bg-destructive/10 border border-destructive/30' 
                                  : isKeep 
                                    ? 'bg-green-500/5' 
                                    : 'bg-yellow-500/5 hover:bg-yellow-500/10'
                              }`}
                            >
                              {/* Radio button to select which one to keep */}
                              <input
                                type="radio"
                                name={`keep-group-${groupIndex}`}
                                checked={isKeep}
                                onChange={() => {
                                  setKeepIds(prev => new Map(prev).set(groupIndex, movie.id))
                                  // Remove this movie from selected (can't delete what we're keeping)
                                  setSelectedIds(prev => {
                                    const next = new Set(prev)
                                    next.delete(movie.id)
                                    return next
                                  })
                                }}
                                className="h-4 w-4 border-gray-300 text-green-600 focus:ring-green-500"
                                title={t('import.duplicates.keepThis')}
                              />
                              {/* Checkbox to select for deletion (only if not the "keep" one) */}
                              {!isKeep ? (
                                <input
                                  type="checkbox"
                                  checked={isSelected}
                                  onChange={() => toggleSelection(movie.id)}
                                  className="h-4 w-4 rounded border-gray-300"
                                />
                              ) : (
                                <div className="w-4 h-4 flex items-center justify-center">
                                  <Check className="h-3 w-3 text-green-600" />
                                </div>
                              )}
                              <div className="w-10 h-14 rounded overflow-hidden bg-muted flex-shrink-0">
                                <PosterImage
                                  posterPath={null}
                                  movieId={movie.id}
                                  size="w92"
                                  alt={movie.title}
                                  className="w-full h-full object-cover"
                                  updatedAt={movie.updated_at}
                                />
                              </div>
                              <div className="flex-1 min-w-0">
                                <p className="font-medium text-sm truncate">
                                  {movie.title}
                                  {isKeep && (
                                    <span className="ml-2 text-xs text-green-600 font-normal">({t('import.duplicates.keep')})</span>
                                  )}
                                </p>
                                <p className="text-xs text-muted-foreground">
                                  {movie.production_year && `${movie.production_year} · `}
                                  {movie.disc_type && `${movie.disc_type} · `}
                                  {movie.barcode && `${movie.barcode}`}
                                </p>
                              </div>
                              <div className="flex items-center gap-2">
                                <Link
                                  to="/movies/$movieId"
                                  params={{ movieId: movie.id }}
                                  className="text-xs text-primary hover:underline"
                                >
                                  {t('movies.details')}
                                </Link>
                                {!isKeep && (
                                  <button
                                    onClick={() => {
                                      if (confirm(t('import.duplicates.confirmDelete', { title: movie.title }))) {
                                        deleteMutation.mutate(movie.id)
                                      }
                                    }}
                                    disabled={deleteMutation.isPending || isDeleting}
                                    className="p-1 text-destructive hover:bg-destructive/10 rounded"
                                    title={t('common.delete')}
                                  >
                                    <Trash2 className="h-4 w-4" />
                                  </button>
                                )}
                              </div>
                            </div>
                          )
                        })}
                      </div>
                    </div>
                  )
                })}
              </div>
            </>
          )}
        </div>
      )}
    </div>
  )
}

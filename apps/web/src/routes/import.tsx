import { PosterImage } from '@/components/PosterImage'
import { useI18n } from '@/hooks/useI18n'
import { api, ImportResult, Movie } from '@/lib/api'
import { wsClient, WsMessage } from '@/lib/ws'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { createFileRoute, Link, redirect } from '@tanstack/react-router'
import { AlertCircle, Check, ChevronDown, Copy, FileUp, Image, RefreshCw, Trash2, Upload } from 'lucide-react'
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
  const fileInputRef = useRef<HTMLInputElement>(null)
  const queryClient = useQueryClient()

  // Check if TMDB API key is configured (admin only, fails silently for non-admins)
  const { data: settings } = useQuery({
    queryKey: ['settings'],
    queryFn: () => api.getSettings(),
    retry: false, // Don't retry if forbidden (non-admin)
  })
  // Only disable if we explicitly know it's NOT configured
  const tmdbSetting = settings?.find(s => s.key === 'tmdb_api_key')
  const tmdbApiConfigured = !settings ? true : (tmdbSetting?.is_configured ?? true)

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
        <div className="text-center">
          <Upload className="mx-auto h-12 w-12 text-muted-foreground" />
          <h2 className="mt-4 text-base md:text-lg font-semibold">{t('import.csvImport')}</h2>
          <p className="mt-2 text-xs md:text-sm text-muted-foreground">
            {t('import.csvImportDesc')}
          </p>
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
            className="flex items-center justify-center gap-2 rounded-md border border-dashed bg-background px-4 py-8 text-sm hover:border-primary hover:bg-accent active:bg-accent/80 min-h-touch"
          >
            <FileUp className="h-5 w-5" />
            <span className="truncate">{selectedFile ? selectedFile.name : t('import.selectFile')}</span>
          </button>

          {selectedFile && (
            <button
              onClick={handleImport}
              disabled={importMutation.isPending}
              className="flex items-center justify-center gap-2 rounded-md bg-primary px-4 py-3 text-sm text-primary-foreground hover:bg-primary/90 active:bg-primary/80 disabled:opacity-50 min-h-touch"
            >
              {importMutation.isPending ? t('import.importing') : t('import.title')}
            </button>
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

function DuplicatesSection() {
  const [showDuplicates, setShowDuplicates] = useState(false)
  const [selectedIds, setSelectedIds] = useState<Set<string>>(new Set())
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

  // Get all duplicate IDs (excluding first in each group)
  const allDuplicateIds = duplicateGroups.flatMap(group => 
    group.slice(1).map((m: Movie) => m.id)
  )

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
    setSelectedIds(new Set(allDuplicateIds))
  }

  const clearSelection = () => {
    setSelectedIds(new Set())
  }

  const deleteSelected = async () => {
    if (selectedIds.size === 0) return
    
    const count = selectedIds.size
    if (!confirm(`${count} ausgewählte Duplikate wirklich löschen?`)) return

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
    if (!confirm(`Alle ${count} Duplikate löschen? (Der erste Eintrag jeder Gruppe wird behalten)`)) return

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
          <h2 className="font-semibold">Duplikate finden</h2>
          <p className="mt-1 text-sm text-muted-foreground">
            Sucht nach doppelten Einträgen basierend auf Barcode, TMDB ID oder Titel.
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
            Suche Duplikate...
          </>
        ) : (
          <>
            <Copy className="h-4 w-4" />
            Duplikate suchen
          </>
        )}
      </button>

      {showDuplicates && !isLoading && (
        <div className="space-y-4">
          {duplicateGroups.length === 0 ? (
            <div className="flex items-start gap-2 rounded-md bg-green-500/10 p-3 text-sm text-green-700">
              <Check className="h-4 w-4 mt-0.5 shrink-0" />
              <span>Keine Duplikate gefunden!</span>
            </div>
          ) : (
            <>
              <div className="flex items-center justify-between gap-4">
                <div className="flex items-start gap-2 rounded-md bg-yellow-500/10 p-3 text-sm text-yellow-700 flex-1">
                  <AlertCircle className="h-4 w-4 mt-0.5 shrink-0" />
                  <span>{duplicateGroups.length} Gruppen, {allDuplicateIds.length} Duplikate</span>
                </div>
              </div>

              {/* Action Bar */}
              <div className="flex flex-wrap items-center gap-2 p-3 rounded-md bg-muted/50">
                <button
                  onClick={selectAllDuplicates}
                  className="text-xs px-2 py-1 rounded bg-secondary hover:bg-secondary/80"
                >
                  Alle auswählen ({allDuplicateIds.length})
                </button>
                {selectedIds.size > 0 && (
                  <>
                    <button
                      onClick={clearSelection}
                      className="text-xs px-2 py-1 rounded bg-secondary hover:bg-secondary/80"
                    >
                      Auswahl aufheben
                    </button>
                    <span className="text-xs text-muted-foreground">
                      {selectedIds.size} ausgewählt
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
                    Ausgewählte löschen ({selectedIds.size})
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
                  Alle Duplikate löschen
                </button>
              </div>

              <div className="space-y-4 max-h-[500px] overflow-auto">
                {duplicateGroups.map((group, groupIndex) => (
                  <div key={groupIndex} className="rounded-md border p-4 space-y-3">
                    <h4 className="font-medium text-sm">
                      Gruppe {groupIndex + 1}: {group[0]?.title}
                      <span className="text-muted-foreground font-normal ml-2">
                        ({group.length} Einträge)
                      </span>
                    </h4>
                    <div className="space-y-2">
                      {group.map((movie: Movie, movieIndex: number) => {
                        const isDuplicate = movieIndex > 0
                        const isSelected = selectedIds.has(movie.id)
                        
                        return (
                          <div
                            key={movie.id}
                            className={`flex items-center gap-3 p-2 rounded transition-colors ${
                              isSelected 
                                ? 'bg-destructive/10 border border-destructive/30' 
                                : isDuplicate 
                                  ? 'bg-yellow-500/5 hover:bg-yellow-500/10' 
                                  : 'bg-green-500/5'
                            }`}
                          >
                            {isDuplicate ? (
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
                                {!isDuplicate && (
                                  <span className="ml-2 text-xs text-green-600 font-normal">(Behalten)</span>
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
                                Details
                              </Link>
                              {isDuplicate && (
                                <button
                                  onClick={() => {
                                    if (confirm(`"${movie.title}" wirklich löschen?`)) {
                                      deleteMutation.mutate(movie.id)
                                    }
                                  }}
                                  disabled={deleteMutation.isPending || isDeleting}
                                  className="p-1 text-destructive hover:bg-destructive/10 rounded"
                                  title="Löschen"
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
                ))}
              </div>
            </>
          )}
        </div>
      )}
    </div>
  )
}

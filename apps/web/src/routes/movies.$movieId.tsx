import { PosterImage } from '@/components/PosterImage'
import { useI18n } from '@/hooks/useI18n'
import { api, CollectionAnalysisResult, SelectedMovie } from '@/lib/api'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { createFileRoute, redirect, useNavigate, useRouter } from '@tanstack/react-router'
import { ArrowLeft, Calendar, Check, ChevronDown, Clock, Disc, Edit2, ImagePlus, Link as LinkIcon, MapPin, RefreshCw, Star, Trash2, Upload, X, Package, Loader2, AlertTriangle, Film } from 'lucide-react'
import { useRef, useState, DragEvent } from 'react'

export const Route = createFileRoute('/movies/$movieId')({
  beforeLoad: ({ context }) => {
    if (!context.auth.isAuthenticated) {
      throw redirect({ to: '/login' })
    }
  },
  component: MovieDetailPage,
})

const DISC_TYPES = [
  { value: 'DVD', label: 'DVD' },
  { value: 'Blu-ray', label: 'Blu-ray' },
  { value: 'uhdbluray', label: '4K UHD Blu-ray' },
  { value: 'hddvd', label: 'HD DVD' },
] as const

function MovieDetailPage() {
  const { movieId } = Route.useParams()
  const navigate = useNavigate()
  const router = useRouter()
  const queryClient = useQueryClient()
  const [showRefreshMenu, setShowRefreshMenu] = useState(false)
  const [showDiscTypeMenu, setShowDiscTypeMenu] = useState(false)
  const [showPosterModal, setShowPosterModal] = useState(false)
  const [posterPreview, setPosterPreview] = useState<string | null>(null)
  const [posterFile, setPosterFile] = useState<File | null>(null)
  const [posterUrl, setPosterUrl] = useState('')
  const [isDragging, setIsDragging] = useState(false)
  const fileInputRef = useRef<HTMLInputElement>(null)
  
  // Collection analysis state
  const [showCollectionModal, setShowCollectionModal] = useState(false)
  const [collectionAnalysis, setCollectionAnalysis] = useState<CollectionAnalysisResult | null>(null)
  const [selectedTitles, setSelectedTitles] = useState<Set<number>>(new Set())

  const { data: movie, isLoading } = useQuery({
    queryKey: ['movie', movieId],
    queryFn: () => api.getMovie(movieId),
  })

  const toggleWatchedMutation = useMutation({
    mutationFn: () => api.updateMovie(movieId, { watched: !movie?.watched }),
    // WebSocket event will handle cache invalidation
  })

  const deleteMutation = useMutation({
    mutationFn: () => api.deleteMovie(movieId),
    onSuccess: () => {
      // WebSocket event will handle cache invalidation
      navigate({ to: '/movies' })
    },
  })

  const refreshTmdbMutation = useMutation({
    mutationFn: (force: boolean) => api.refreshMovieTmdb(movieId, force),
    onSuccess: () => {
      // Invalidate cache to show updated poster (especially for collections)
      queryClient.invalidateQueries({ queryKey: ['movie', movieId] })
      queryClient.invalidateQueries({ queryKey: ['movies'] })
    },
  })

  const uploadPosterMutation = useMutation({
    mutationFn: (file: File) => api.uploadMoviePoster(movieId, file),
    onSuccess: () => {
      closePosterModal()
      queryClient.invalidateQueries({ queryKey: ['movie', movieId] })
    },
  })

  const setPosterFromUrlMutation = useMutation({
    mutationFn: (url: string) => api.setPosterFromUrl(movieId, url),
    onSuccess: () => {
      closePosterModal()
      queryClient.invalidateQueries({ queryKey: ['movie', movieId] })
    },
  })

  const updateDiscTypeMutation = useMutation({
    mutationFn: (discType: string) => api.updateMovie(movieId, { disc_type: discType }),
    onSuccess: () => {
      setShowDiscTypeMenu(false)
      // WebSocket event will handle cache invalidation
    },
  })

  const toggleCollectionMutation = useMutation({
    mutationFn: (isCollection: boolean) => api.updateMovie(movieId, { is_collection: isCollection }),
    // WebSocket event will handle cache invalidation
  })

  const analyzeCollectionMutation = useMutation({
    mutationFn: () => api.analyzeCollection(movieId),
    onSuccess: (result) => {
      setCollectionAnalysis(result)
      // Select all by default
      setSelectedTitles(new Set(result.extracted_titles.map((_, i) => i)))
    },
  })

  const splitCollectionMutation = useMutation({
    mutationFn: ({ selectedMovies, collectionPosterPath }: { selectedMovies: SelectedMovie[], collectionPosterPath?: string }) => 
      api.splitCollection(movieId, selectedMovies, true, collectionPosterPath),
    onSuccess: () => {
      setShowCollectionModal(false)
      setCollectionAnalysis(null)
      queryClient.invalidateQueries({ queryKey: ['movies'] })
      queryClient.invalidateQueries({ queryKey: ['movie', movieId] })
    },
  })

  const { t } = useI18n()

  const handleAnalyzeCollection = () => {
    setShowCollectionModal(true)
    analyzeCollectionMutation.mutate()
  }

  const handleSplitCollection = () => {
    if (!collectionAnalysis) return
    
    const selectedMovies: SelectedMovie[] = collectionAnalysis.extracted_titles
      .filter((_, i) => selectedTitles.has(i))
      .filter((item) => !item.is_tv_series) // Skip TV series for now - they need different handling
      .map((item) => ({
        title: item.title,
        tmdb_id: item.tmdb_match?.id,
      }))
    
    if (selectedMovies.length === 0) return
    
    // Get collection poster path: prefer TMDB collection poster, fallback to first movie's poster
    const collectionPosterPath = collectionAnalysis.tmdb_collection?.poster_path 
      || collectionAnalysis.extracted_titles[0]?.tmdb_match?.poster_path
    
    splitCollectionMutation.mutate({ selectedMovies, collectionPosterPath })
  }

  const toggleTitleSelection = (index: number) => {
    setSelectedTitles(prev => {
      const next = new Set(prev)
      if (next.has(index)) {
        next.delete(index)
      } else {
        next.add(index)
      }
      return next
    })
  }

  const closePosterModal = () => {
    setShowPosterModal(false)
    setPosterPreview(null)
    setPosterFile(null)
    setPosterUrl('')
    setIsDragging(false)
    if (fileInputRef.current) fileInputRef.current.value = ''
  }

  const handleFileSelect = (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0]
    if (!file) return
    processFile(file)
  }

  const processFile = (file: File) => {
    if (!file.type.startsWith('image/')) {
      alert(t('poster.invalidFileType'))
      return
    }

    setPosterFile(file)
    const reader = new FileReader()
    reader.onloadend = () => {
      setPosterPreview(reader.result as string)
    }
    reader.readAsDataURL(file)
  }

  const handleDragOver = (e: DragEvent<HTMLDivElement>) => {
    e.preventDefault()
    setIsDragging(true)
  }

  const handleDragLeave = (e: DragEvent<HTMLDivElement>) => {
    e.preventDefault()
    setIsDragging(false)
  }

  const handleDrop = (e: DragEvent<HTMLDivElement>) => {
    e.preventDefault()
    setIsDragging(false)
    
    const file = e.dataTransfer.files?.[0]
    if (file) {
      processFile(file)
    }
  }

  const handleUploadPoster = () => {
    // Prefer file upload if a file is selected, otherwise use URL
    if (posterFile) {
      uploadPosterMutation.mutate(posterFile)
    } else if (posterUrl.trim()) {
      setPosterFromUrlMutation.mutate(posterUrl.trim())
    }
  }

  const isUploading = uploadPosterMutation.isPending || setPosterFromUrlMutation.isPending
  const canUpload = posterFile || posterUrl.trim()

  if (isLoading) {
    return <div className="text-center py-12 text-muted-foreground">{t('common.loading')}</div>
  }

  if (!movie) {
    return <div className="text-center py-12 text-muted-foreground">{t('movies.notFound')}</div>
  }

  const discTypeLabel = (type?: string) => {
    switch (type?.toLowerCase()) {
      case 'bluray': return 'Blu-ray'
      case 'uhdbluray': return '4K UHD Blu-ray'
      case 'dvd': return 'DVD'
      case 'hddvd': return 'HD DVD'
      default: return type || ''
    }
  }

  return (
    <div className="space-y-4 md:space-y-6 relative">
      {/* Floating back button */}
      <button
        onClick={() => router.history.back()}
        className="fixed z-40 flex items-center gap-2 rounded-full bg-card/90 backdrop-blur-sm border shadow-lg px-4 py-2 text-sm font-medium hover:bg-card active:bg-accent transition-colors left-4 md:left-8"
        style={{
          top: 'calc(4.5rem + env(safe-area-inset-top, 0px))',
        }}
      >
        <ArrowLeft className="h-4 w-4" />
        <span className="hidden sm:inline">{t('common.back')}</span>
      </button>

      {/* Spacer for the floating button */}
      <div className="h-6" />

      <div className="flex flex-col lg:grid lg:grid-cols-[350px_1fr] gap-6 md:gap-8">
        {/* Poster */}
        <div className="space-y-4">
          <div className="relative group">
            <div className="aspect-[2/3] max-w-xs mx-auto lg:max-w-none rounded-lg bg-muted flex items-center justify-center overflow-hidden shadow-lg">
              <PosterImage
                posterPath={null}
                movieId={movie.id}
                size="w500"
                alt={movie.title}
                className="w-full h-full object-cover"
                updatedAt={movie.updated_at}
              />
            </div>
            
            {/* Poster change overlay */}
            <div className="absolute inset-0 max-w-xs mx-auto lg:max-w-none rounded-lg bg-black/60 opacity-0 group-hover:opacity-100 transition-opacity flex items-center justify-center">
              <button
                onClick={() => setShowPosterModal(true)}
                className="flex flex-col items-center gap-2 text-white hover:scale-105 transition-transform"
              >
                <ImagePlus className="h-8 w-8" />
                <span className="text-sm font-medium">{t('movies.changePoster')}</span>
              </button>
            </div>
          </div>

          {/* Hidden file input */}
          <input
            ref={fileInputRef}
            type="file"
            accept="image/*"
            onChange={handleFileSelect}
            className="hidden"
          />

          {/* Poster Upload Modal */}
          {showPosterModal && (
            <div 
              className="fixed inset-0 bg-black/50 z-50 flex items-center justify-center p-4"
              onClick={closePosterModal}
            >
              <div 
                className="bg-card rounded-xl shadow-2xl w-full max-w-md overflow-hidden"
                onClick={(e) => e.stopPropagation()}
              >
                {/* Modal Header */}
                <div className="flex items-center justify-between px-4 py-3 border-b bg-muted/50">
                  <h3 className="text-sm font-semibold">{t('movies.changePoster')}</h3>
                  <button
                    onClick={closePosterModal}
                    className="p-1 hover:bg-muted rounded-md transition-colors"
                  >
                    <X className="h-4 w-4" />
                  </button>
                </div>

                {/* Modal Body */}
                <div className="p-4 space-y-4">
                  {/* Dropzone */}
                  <div
                    onDragOver={handleDragOver}
                    onDragLeave={handleDragLeave}
                    onDrop={handleDrop}
                    onClick={() => fileInputRef.current?.click()}
                    className={`border-2 border-dashed rounded-lg p-6 text-center cursor-pointer transition-colors ${
                      isDragging 
                        ? 'border-primary bg-primary/10' 
                        : 'border-muted-foreground/30 hover:border-primary hover:bg-muted/50'
                    }`}
                  >
                    {posterPreview ? (
                      <div className="space-y-3">
                        <img
                          src={posterPreview}
                          alt="Preview"
                          className="h-40 mx-auto rounded-lg object-contain"
                        />
                        <p className="text-sm text-muted-foreground">
                          {posterFile?.name}
                        </p>
                        <button
                          type="button"
                          onClick={(e) => {
                            e.stopPropagation()
                            setPosterPreview(null)
                            setPosterFile(null)
                            if (fileInputRef.current) fileInputRef.current.value = ''
                          }}
                          className="text-sm text-destructive hover:underline"
                        >
                          {t('poster.removeImage')}
                        </button>
                      </div>
                    ) : (
                      <div className="space-y-2">
                        <ImagePlus className="h-10 w-10 mx-auto text-muted-foreground" />
                        <p className="text-sm font-medium">{t('poster.dropHere')}</p>
                        <p className="text-xs text-muted-foreground">{t('poster.orClickToSelect')}</p>
                      </div>
                    )}
                  </div>

                  {/* Divider */}
                  <div className="flex items-center gap-3">
                    <div className="flex-1 border-t" />
                    <span className="text-xs text-muted-foreground">{t('poster.or')}</span>
                    <div className="flex-1 border-t" />
                  </div>

                  {/* URL Input */}
                  <div className="space-y-2">
                    <label className="text-sm font-medium flex items-center gap-2">
                      <LinkIcon className="h-4 w-4" />
                      {t('poster.imageUrl')}
                    </label>
                    <input
                      type="url"
                      value={posterUrl}
                      onChange={(e) => setPosterUrl(e.target.value)}
                      placeholder="https://..."
                      disabled={!!posterFile}
                      className="w-full rounded-md border bg-background px-4 py-3 text-sm focus:border-primary focus:outline-none focus:ring-1 focus:ring-primary disabled:opacity-50 disabled:cursor-not-allowed"
                    />
                    <p className="text-xs text-muted-foreground">{t('poster.urlHint')}</p>
                  </div>
                </div>

                {/* Modal Footer */}
                <div className="flex gap-2 px-4 py-3 border-t bg-muted/30">
                  <button
                    onClick={closePosterModal}
                    className="flex-1 rounded-md bg-secondary px-4 py-3 text-sm hover:bg-secondary/80 min-h-touch"
                  >
                    {t('common.cancel')}
                  </button>
                  <button
                    onClick={handleUploadPoster}
                    disabled={!canUpload || isUploading}
                    className="flex-1 flex items-center justify-center gap-2 rounded-md bg-primary px-4 py-3 text-sm text-primary-foreground hover:bg-primary/90 disabled:opacity-50 min-h-touch"
                  >
                    {isUploading ? (
                      <RefreshCw className="h-4 w-4 animate-spin" />
                    ) : (
                      <Upload className="h-4 w-4" />
                    )}
                    {t('common.save')}
                  </button>
                </div>
              </div>
            </div>
          )}

          {/* Collection Analysis Modal */}
          {showCollectionModal && (
            <div 
              className="fixed inset-0 bg-black/50 z-50 flex items-center justify-center p-4"
              onClick={() => {
                setShowCollectionModal(false)
                setCollectionAnalysis(null)
              }}
            >
              <div 
                className="bg-card rounded-xl shadow-2xl w-full max-w-lg max-h-[80vh] overflow-hidden flex flex-col"
                onClick={(e) => e.stopPropagation()}
              >
                {/* Modal Header */}
                <div className="flex items-center justify-between px-4 py-3 border-b bg-muted/50 shrink-0">
                  <h3 className="text-sm font-semibold flex items-center gap-2">
                    <Package className="h-4 w-4" />
                    {t('movies.analyzeCollection')}
                  </h3>
                  <button
                    onClick={() => {
                      setShowCollectionModal(false)
                      setCollectionAnalysis(null)
                    }}
                    className="p-1 hover:bg-muted rounded-md transition-colors"
                  >
                    <X className="h-4 w-4" />
                  </button>
                </div>

                {/* Modal Body */}
                <div className="p-4 overflow-y-auto flex-1">
                  {analyzeCollectionMutation.isPending ? (
                    <div className="flex flex-col items-center justify-center py-12 space-y-4">
                      <Loader2 className="h-8 w-8 animate-spin text-primary" />
                      <p className="text-sm text-muted-foreground">{t('movies.analyzingCollection')}</p>
                    </div>
                  ) : collectionAnalysis ? (
                    <div className="space-y-4">
                      {/* Analysis Result */}
                      {collectionAnalysis.is_collection ? (
                        <>
                          {/* Confidence Badge */}
                          <div className="flex items-center gap-2">
                            {collectionAnalysis.confidence >= 0.8 ? (
                              <div className="flex items-center gap-2 text-green-600 bg-green-100 dark:bg-green-900/30 px-3 py-1.5 rounded-full text-sm">
                                <Check className="h-4 w-4" />
                                {t('movies.collectionConfidenceHigh')}
                              </div>
                            ) : collectionAnalysis.confidence >= 0.5 ? (
                              <div className="flex items-center gap-2 text-yellow-600 bg-yellow-100 dark:bg-yellow-900/30 px-3 py-1.5 rounded-full text-sm">
                                <AlertTriangle className="h-4 w-4" />
                                {t('movies.collectionConfidenceMedium')}
                              </div>
                            ) : (
                              <div className="flex items-center gap-2 text-orange-600 bg-orange-100 dark:bg-orange-900/30 px-3 py-1.5 rounded-full text-sm">
                                <AlertTriangle className="h-4 w-4" />
                                {t('movies.collectionConfidenceLow')}
                              </div>
                            )}
                            {collectionAnalysis.tmdb_collection && (
                              <span className="text-xs text-muted-foreground">
                                TMDB: {collectionAnalysis.tmdb_collection.name}
                              </span>
                            )}
                          </div>

                          {/* Found Movies List */}
                          <div className="space-y-2">
                            <p className="text-sm font-medium">
                              {t('movies.foundMovies', { count: collectionAnalysis.total_movies })}
                            </p>
                            <div className="space-y-2 max-h-[300px] overflow-y-auto">
                              {collectionAnalysis.extracted_titles.map((item, index) => {
                                const hasMatch = item.tmdb_match || item.tmdb_tv_match
                                const posterPath = item.tmdb_match?.poster_path || item.tmdb_tv_match?.poster_path
                                const matchId = item.tmdb_match?.id || item.tmdb_tv_match?.id
                                const matchYear = item.tmdb_match?.year || (item.tmdb_tv_match?.first_air_date ? item.tmdb_tv_match.first_air_date.substring(0, 4) : null)
                                
                                return (
                                  <label
                                    key={index}
                                    className={`flex items-start gap-3 p-3 rounded-lg border cursor-pointer transition-colors ${
                                      selectedTitles.has(index)
                                        ? 'border-primary bg-primary/5'
                                        : 'border-muted-foreground/20 hover:border-muted-foreground/40'
                                    }`}
                                  >
                                    <input
                                      type="checkbox"
                                      checked={selectedTitles.has(index)}
                                      onChange={() => toggleTitleSelection(index)}
                                      className="mt-1 h-4 w-4 rounded border-gray-300 text-primary focus:ring-primary"
                                    />
                                    <div className="flex-1 min-w-0">
                                      <div className="flex items-center gap-2">
                                        {item.is_tv_series ? (
                                          <span className={`text-xs px-1.5 py-0.5 rounded ${hasMatch ? 'bg-blue-500 text-white' : 'bg-muted text-muted-foreground'}`}>TV</span>
                                        ) : hasMatch ? (
                                          <Film className="h-4 w-4 text-green-500 shrink-0" />
                                        ) : (
                                          <Film className="h-4 w-4 text-muted-foreground shrink-0" />
                                        )}
                                        <span className="font-medium text-sm truncate">{item.title}</span>
                                      </div>
                                      {hasMatch && (
                                        <p className="text-xs text-muted-foreground mt-1">
                                          TMDB {item.is_tv_series ? 'TV' : 'Movie'} ID: {matchId}
                                          {matchYear && ` • ${matchYear}`}
                                        </p>
                                      )}
                                      {item.description_excerpt && !hasMatch && (
                                        <p className="text-xs text-muted-foreground mt-1 line-clamp-2">
                                          {item.description_excerpt}
                                        </p>
                                      )}
                                    </div>
                                    {posterPath && (
                                      <img
                                        src={`https://image.tmdb.org/t/p/w92${posterPath}`}
                                        alt={item.title}
                                        className="h-16 w-11 rounded object-cover shrink-0"
                                      />
                                    )}
                                  </label>
                                )
                              })}
                            </div>
                          </div>

                          {/* Selection Info */}
                          <p className="text-xs text-muted-foreground text-center">
                            {t('movies.selectedForSplit', { count: selectedTitles.size, total: collectionAnalysis.total_movies })}
                          </p>
                          {collectionAnalysis.extracted_titles.some(item => item.is_tv_series) && (
                            <p className="text-xs text-blue-600 dark:text-blue-400 text-center">
                              TV-Serien können nicht in Einzelfilme aufgeteilt werden.
                            </p>
                          )}
                        </>
                      ) : collectionAnalysis.extracted_titles.length === 1 && collectionAnalysis.extracted_titles[0].is_tv_series ? (
                        // Single TV series detected
                        <div className="space-y-4">
                          <div className="flex items-center gap-2 text-blue-600 bg-blue-100 dark:bg-blue-900/30 px-3 py-1.5 rounded-full text-sm w-fit">
                            <span className="font-medium">TV Serie erkannt</span>
                          </div>
                          {(() => {
                            const tvItem = collectionAnalysis.extracted_titles[0]
                            const tvMatch = tvItem.tmdb_tv_match
                            return tvMatch ? (
                              <div className="flex gap-4 p-4 rounded-lg border bg-card">
                                {tvMatch.poster_path && (
                                  <img
                                    src={`https://image.tmdb.org/t/p/w154${tvMatch.poster_path}`}
                                    alt={tvMatch.name}
                                    className="h-32 w-auto rounded object-cover shrink-0"
                                  />
                                )}
                                <div className="flex-1 min-w-0">
                                  <h4 className="font-semibold text-lg">{tvMatch.name}</h4>
                                  {tvMatch.original_name && tvMatch.original_name !== tvMatch.name && (
                                    <p className="text-sm text-muted-foreground">{tvMatch.original_name}</p>
                                  )}
                                  <p className="text-xs text-muted-foreground mt-1">
                                    TMDB TV ID: {tvMatch.id}
                                    {tvMatch.first_air_date && ` • ${tvMatch.first_air_date.substring(0, 4)}`}
                                    {tvMatch.vote_average && ` • ★ ${tvMatch.vote_average.toFixed(1)}`}
                                  </p>
                                  {tvMatch.overview && (
                                    <p className="text-sm text-muted-foreground mt-2 line-clamp-3">
                                      {tvMatch.overview}
                                    </p>
                                  )}
                                </div>
                              </div>
                            ) : (
                              <p className="text-sm text-muted-foreground">
                                TMDB konnte keine passende Serie finden.
                              </p>
                            )
                          })()}
                          <p className="text-xs text-muted-foreground text-center">
                            Dies ist eine TV-Serie, kein Film. Die Daten können über TMDB TV aktualisiert werden.
                          </p>
                        </div>
                      ) : (
                        <div className="text-center py-8 space-y-3">
                          <Package className="h-12 w-12 mx-auto text-muted-foreground" />
                          <p className="text-sm text-muted-foreground">{t('movies.notACollection')}</p>
                        </div>
                      )}
                    </div>
                  ) : analyzeCollectionMutation.isError ? (
                    <div className="text-center py-8 space-y-3">
                      <AlertTriangle className="h-12 w-12 mx-auto text-destructive" />
                      <p className="text-sm text-destructive">{t('movies.analyzeError')}</p>
                    </div>
                  ) : null}
                </div>

                {/* Modal Footer */}
                {collectionAnalysis?.is_collection && (
                  <div className="flex gap-2 px-4 py-3 border-t bg-muted/30 shrink-0">
                    <button
                      onClick={() => {
                        setShowCollectionModal(false)
                        setCollectionAnalysis(null)
                      }}
                      className="flex-1 rounded-md bg-secondary px-4 py-3 text-sm hover:bg-secondary/80 min-h-touch"
                    >
                      {t('common.cancel')}
                    </button>
                    <button
                      onClick={handleSplitCollection}
                      disabled={selectedTitles.size === 0 || splitCollectionMutation.isPending}
                      className="flex-1 flex items-center justify-center gap-2 rounded-md bg-primary px-4 py-3 text-sm text-primary-foreground hover:bg-primary/90 disabled:opacity-50 min-h-touch"
                    >
                      {splitCollectionMutation.isPending ? (
                        <Loader2 className="h-4 w-4 animate-spin" />
                      ) : (
                        <Package className="h-4 w-4" />
                      )}
                      {t('movies.splitCollection')}
                    </button>
                  </div>
                )}
              </div>
            </div>
          )}

          {/* Quick Info Cards */}
          <div className="grid grid-cols-2 gap-2">
            {movie.production_year && (
              <div className="rounded-lg bg-card border p-3 text-center">
                <Calendar className="h-4 w-4 mx-auto mb-1 text-muted-foreground" />
                <p className="text-sm font-medium">{movie.production_year}</p>
                <p className="text-xs text-muted-foreground">{t('movies.year')}</p>
              </div>
            )}
            {movie.running_time && (
              <div className="rounded-lg bg-card border p-3 text-center">
                <Clock className="h-4 w-4 mx-auto mb-1 text-muted-foreground" />
                <p className="text-sm font-medium">{movie.running_time} {t('movies.minutes')}</p>
                <p className="text-xs text-muted-foreground">{t('movies.runningTime')}</p>
              </div>
            )}
            {movie.personal_rating && (
              <div className="rounded-lg bg-card border p-3 text-center">
                <Star className="h-4 w-4 mx-auto mb-1 text-yellow-500" />
                <p className="text-sm font-medium">{movie.personal_rating}/10</p>
                <p className="text-xs text-muted-foreground">{t('movies.rating')}</p>
              </div>
            )}
            
            {/* Disc Type Card - Always shown, editable */}
            <div className="relative">
              <button
                onClick={() => setShowDiscTypeMenu(!showDiscTypeMenu)}
                className="w-full rounded-lg bg-card border p-3 text-center hover:border-primary transition-colors group"
              >
                <Disc className="h-4 w-4 mx-auto mb-1 text-muted-foreground group-hover:text-primary" />
                <p className="text-sm font-medium flex items-center justify-center gap-1">
                  {movie.disc_type ? discTypeLabel(movie.disc_type) : t('movies.selectFormat')}
                  <Edit2 className="h-3 w-3 text-muted-foreground opacity-0 group-hover:opacity-100 transition-opacity" />
                </p>
                <p className="text-xs text-muted-foreground">{t('movies.format')}</p>
              </button>

              {/* Disc type dropdown */}
              {showDiscTypeMenu && (
                <>
                  {/* Desktop dropdown */}
                  <div className="hidden sm:block absolute top-full left-0 right-0 mt-1 bg-card border rounded-md shadow-lg z-10">
                    {DISC_TYPES.map((type) => (
                      <button
                        key={type.value}
                        onClick={() => updateDiscTypeMutation.mutate(type.value)}
                        disabled={updateDiscTypeMutation.isPending}
                        className={`w-full text-left px-4 py-3 text-sm hover:bg-muted transition-colors min-h-touch ${
                          movie.disc_type === type.value ? 'bg-muted font-medium' : ''
                        }`}
                      >
                        {type.label}
                      </button>
                    ))}
                    <button
                      onClick={() => setShowDiscTypeMenu(false)}
                      className="w-full text-center px-4 py-2 text-xs text-muted-foreground hover:bg-muted transition-colors border-t"
                    >
                      {t('common.cancel')}
                    </button>
                  </div>

                  {/* Mobile dialog */}
                  <div 
                    className="sm:hidden fixed inset-0 bg-black/50 z-50"
                    onClick={() => setShowDiscTypeMenu(false)}
                  >
                    <div 
                      className="fixed left-4 right-4 top-1/2 -translate-y-1/2 bg-card rounded-xl shadow-2xl overflow-hidden"
                      onClick={(e) => e.stopPropagation()}
                    >
                      <div className="px-4 py-3 border-b bg-muted/50">
                        <h3 className="text-sm font-semibold text-center">
                          {t('movies.selectFormat')}
                        </h3>
                      </div>
                      {DISC_TYPES.map((type) => (
                        <button
                          key={type.value}
                          onClick={() => updateDiscTypeMutation.mutate(type.value)}
                          disabled={updateDiscTypeMutation.isPending}
                          className={`w-full text-left px-4 py-4 text-sm hover:bg-muted transition-colors min-h-touch ${
                            movie.disc_type === type.value ? 'bg-muted font-medium' : ''
                          }`}
                        >
                          {type.label}
                        </button>
                      ))}
                      <button
                        onClick={() => setShowDiscTypeMenu(false)}
                        className="w-full text-center px-4 py-4 text-sm font-medium text-destructive hover:bg-muted transition-colors border-t min-h-touch"
                      >
                        {t('common.cancel')}
                      </button>
                    </div>
                  </div>
                </>
              )}
            </div>

            {/* Collection Type Toggle */}
            <button
              onClick={() => toggleCollectionMutation.mutate(!movie.is_collection)}
              disabled={toggleCollectionMutation.isPending}
              className="w-full rounded-lg bg-card border p-3 text-center hover:border-primary transition-colors group disabled:opacity-50"
            >
              {movie.is_collection ? (
                <Package className="h-4 w-4 mx-auto mb-1 text-primary" />
              ) : (
                <Film className="h-4 w-4 mx-auto mb-1 text-muted-foreground group-hover:text-primary" />
              )}
              <p className="text-sm font-medium flex items-center justify-center gap-1">
                {movie.is_collection ? t('movies.collection') : t('movies.movie')}
                <Edit2 className="h-3 w-3 text-muted-foreground opacity-0 group-hover:opacity-100 transition-opacity" />
              </p>
              <p className="text-xs text-muted-foreground">{t('movies.type')}</p>
            </button>
          </div>
        </div>

        {/* Details */}
        <div className="space-y-4 md:space-y-6">
          {/* Title Section */}
          <div>
            <div className="flex flex-col sm:flex-row sm:items-start sm:justify-between gap-3 sm:gap-4">
              <div className="flex-1 min-w-0">
                <h1 className="text-2xl md:text-3xl font-bold break-words">{movie.title}</h1>
                {movie.original_title && movie.original_title !== movie.title && (
                  <p className="text-base md:text-lg text-muted-foreground mt-1 break-words">{movie.original_title}</p>
                )}
              </div>
              {movie.watched && (
                <div className="flex items-center gap-1 rounded-full bg-green-500 px-3 py-2 text-sm text-white shrink-0">
                  <Check className="h-4 w-4" />
                  {t('movies.watched')}
                </div>
              )}
            </div>

            {movie.tagline && (
              <p className="mt-3 text-base md:text-lg italic text-muted-foreground break-words">„{movie.tagline}"</p>
            )}
          </div>

          {/* Genres */}
          {movie.genres && (
            <div className="flex flex-wrap gap-2">
              {movie.genres.split(',').map(genre => (
                <span
                  key={genre}
                  className="rounded-full bg-primary/10 text-primary px-3 py-1 text-sm"
                >
                  {genre.trim()}
                </span>
              ))}
            </div>
          )}

          {/* Description */}
          {movie.description && (
            <div className="space-y-2">
              <h2 className="text-base md:text-lg font-semibold">{t('movies.plot')}</h2>
              <p className="text-sm md:text-base text-muted-foreground leading-relaxed break-words">{movie.description}</p>
            </div>
          )}

          {/* Cast & Crew */}
          <div className="grid gap-4 md:gap-6 sm:grid-cols-2">
            {movie.director && (
              <div>
                <h2 className="text-xs md:text-sm font-semibold text-muted-foreground uppercase tracking-wide mb-2">{t('movies.director')}</h2>
                <p className="text-sm md:text-base break-words">{movie.director}</p>
              </div>
            )}

            {movie.actors && (
              <div className="sm:col-span-2">
                <h2 className="text-xs md:text-sm font-semibold text-muted-foreground uppercase tracking-wide mb-2">{t('movies.cast')}</h2>
                <p className="text-sm md:text-base leading-relaxed break-words">{movie.actors}</p>
              </div>
            )}
          </div>

          {/* Additional Info */}
          <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-3 pt-4 border-t">
            {movie.barcode && (
              <div>
                <h3 className="text-xs font-semibold text-muted-foreground uppercase">{t('movies.barcode')}</h3>
                <p className="text-sm font-mono break-all">{movie.barcode}</p>
              </div>
            )}
            {movie.imdb_id && (
              <div>
                <h3 className="text-xs font-semibold text-muted-foreground uppercase">IMDb</h3>
                <a
                  href={`https://www.imdb.com/title/${movie.imdb_id}`}
                  target="_blank"
                  rel="noopener noreferrer"
                  className="text-sm text-primary hover:underline active:text-primary/80 break-all"
                >
                  {movie.imdb_id}
                </a>
              </div>
            )}
            {movie.tmdb_id && (
              <div>
                <h3 className="text-xs font-semibold text-muted-foreground uppercase">TMDB</h3>
                <a
                  href={`https://www.themoviedb.org/movie/${movie.tmdb_id}`}
                  target="_blank"
                  rel="noopener noreferrer"
                  className="text-sm text-primary hover:underline active:text-primary/80 break-all"
                >
                  {movie.tmdb_id}
                </a>
              </div>
            )}
            {movie.location && (
              <div>
                <h3 className="text-xs font-semibold text-muted-foreground uppercase">{t('movies.location')}</h3>
                <p className="text-sm flex items-center gap-1 break-words">
                  <MapPin className="h-3 w-3 shrink-0" />
                  {movie.location}
                </p>
              </div>
            )}
            {movie.edition && (
              <div>
                <h3 className="text-xs font-semibold text-muted-foreground uppercase">{t('movies.edition')}</h3>
                <p className="text-sm break-words">{movie.edition}</p>
              </div>
            )}
          </div>

          {/* Notes */}
          {movie.notes && (
            <div className="rounded-lg bg-muted/50 p-4">
              <h2 className="text-sm font-semibold mb-2">{t('movies.notes')}</h2>
              <p className="text-sm text-muted-foreground break-words whitespace-pre-wrap">{movie.notes}</p>
            </div>
          )}

          {/* Actions */}
          <div className="flex flex-col sm:flex-row flex-wrap gap-3 pt-4 border-t">
            <button
              onClick={() => toggleWatchedMutation.mutate()}
              disabled={toggleWatchedMutation.isPending}
              className={`flex items-center justify-center gap-2 rounded-md px-4 py-3 text-sm font-medium transition-colors min-h-touch w-full sm:w-auto ${
                movie.watched
                  ? 'bg-green-500 text-white hover:bg-green-600 active:bg-green-700'
                  : 'bg-secondary hover:bg-secondary/80 active:bg-secondary/60'
              }`}
            >
              <Check className="h-4 w-4" />
              {movie.watched ? t('movies.watched') : t('movies.markAsWatched')}
            </button>
            
            <div className="relative w-full sm:w-auto">
              <button
                onClick={() => setShowRefreshMenu(!showRefreshMenu)}
                disabled={refreshTmdbMutation.isPending}
                className="flex items-center justify-center gap-2 rounded-md bg-secondary px-4 py-3 text-sm font-medium hover:bg-secondary/80 active:bg-secondary/60 min-h-touch w-full sm:w-auto"
              >
                <RefreshCw className={`h-4 w-4 ${refreshTmdbMutation.isPending ? 'animate-spin' : ''}`} />
                {t('movies.refreshTmdb')}
                <ChevronDown className={`h-4 w-4 transition-transform ${showRefreshMenu ? 'rotate-180' : ''}`} />
              </button>
              
              {/* Desktop dropdown */}
              {showRefreshMenu && (
                <div className="hidden sm:block absolute top-full left-0 mt-1 bg-card border rounded-md shadow-lg z-10 min-w-[200px]">
                  <button
                    onClick={() => {
                      refreshTmdbMutation.mutate(false)
                      setShowRefreshMenu(false)
                    }}
                    disabled={refreshTmdbMutation.isPending}
                    className="w-full text-left px-4 py-3 text-sm hover:bg-muted active:bg-muted/80 transition-colors min-h-touch"
                  >
                    {t('movies.refreshTmdbMissing')}
                  </button>
                  <button
                    onClick={() => {
                      refreshTmdbMutation.mutate(true)
                      setShowRefreshMenu(false)
                    }}
                    disabled={refreshTmdbMutation.isPending}
                    className="w-full text-left px-4 py-3 text-sm hover:bg-muted active:bg-muted/80 transition-colors border-t min-h-touch"
                  >
                    {t('movies.refreshTmdbAll')}
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
                      {t('movies.refreshTmdb')}
                    </h3>
                  </div>
                  <button
                    onClick={() => {
                      refreshTmdbMutation.mutate(false)
                      setShowRefreshMenu(false)
                    }}
                    disabled={refreshTmdbMutation.isPending}
                    className="w-full text-left px-4 py-4 text-sm hover:bg-muted active:bg-muted/80 transition-colors min-h-touch"
                  >
                    {t('movies.refreshTmdbMissing')}
                  </button>
                  <button
                    onClick={() => {
                      refreshTmdbMutation.mutate(true)
                      setShowRefreshMenu(false)
                    }}
                    disabled={refreshTmdbMutation.isPending}
                    className="w-full text-left px-4 py-4 text-sm hover:bg-muted active:bg-muted/80 transition-colors border-t min-h-touch"
                  >
                    {t('movies.refreshTmdbAll')}
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

            {/* Collection Analysis Button - Only show if movie might be a collection */}
            {!movie.is_collection && (
              <button
                onClick={handleAnalyzeCollection}
                disabled={analyzeCollectionMutation.isPending}
                className="flex items-center justify-center gap-2 rounded-md bg-secondary px-4 py-3 text-sm font-medium hover:bg-secondary/80 active:bg-secondary/60 min-h-touch w-full sm:w-auto"
              >
                <Package className={`h-4 w-4 ${analyzeCollectionMutation.isPending ? 'animate-pulse' : ''}`} />
                {t('movies.analyzeCollection')}
              </button>
            )}

            {/* Show link to child movies if this is a collection */}
            {movie.is_collection && (
              <div className="flex items-center gap-2 rounded-md bg-primary/10 border border-primary/30 px-4 py-3 text-sm font-medium w-full sm:w-auto">
                <Package className="h-4 w-4 text-primary" />
                <span className="text-primary">{t('movies.isCollection')}</span>
              </div>
            )}

            <button
              onClick={() => {
                if (confirm(t('movies.deleteConfirm'))) {
                  deleteMutation.mutate()
                }
              }}
              disabled={deleteMutation.isPending}
              className="flex items-center justify-center gap-2 rounded-md bg-destructive px-4 py-3 text-sm font-medium text-destructive-foreground hover:bg-destructive/90 active:bg-destructive/80 min-h-touch w-full sm:w-auto sm:ml-auto"
            >
              <Trash2 className="h-4 w-4" />
              {t('movies.delete')}
            </button>
          </div>
        </div>
      </div>
    </div>
  )
}

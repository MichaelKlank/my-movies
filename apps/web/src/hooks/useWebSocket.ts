import { useEffect } from 'react'
import { useQueryClient } from '@tanstack/react-query'
import { wsClient, WsMessage } from '@/lib/ws'

export function useWebSocketSync() {
  const queryClient = useQueryClient()

  useEffect(() => {
    const unsubscribe = wsClient.subscribe((message: WsMessage) => {
      // Handle all cache invalidation via WebSocket events
      switch (message.type) {
        // Movie events
        case 'movie_added':
        case 'movie_updated':
        case 'movie_deleted':
        case 'movies_enriched':
        case 'tmdb_enrich_complete':
          queryClient.invalidateQueries({ queryKey: ['movies'] })
          // Also invalidate individual movie if ID is provided
          if (message.payload && typeof message.payload === 'object' && 'id' in message.payload) {
            queryClient.invalidateQueries({ queryKey: ['movie', (message.payload as { id: string }).id] })
          }
          break

        // Series events
        case 'series_added':
        case 'series_updated':
        case 'series_deleted':
          queryClient.invalidateQueries({ queryKey: ['series'] })
          if (message.payload && typeof message.payload === 'object' && 'id' in message.payload) {
            queryClient.invalidateQueries({ queryKey: ['series', (message.payload as { id: string }).id] })
          }
          break

        // Collection events
        case 'collection_imported':
          queryClient.invalidateQueries({ queryKey: ['movies'] })
          queryClient.invalidateQueries({ queryKey: ['series'] })
          queryClient.invalidateQueries({ queryKey: ['collections'] })
          break

        // TMDB enrichment progress events - don't invalidate, just for UI updates
        case 'tmdb_enrich_started':
        case 'tmdb_enrich_progress':
          // These are handled by the import page directly
          break

        default:
          // Log unknown events for debugging
          console.debug('Unhandled WS message:', message.type)
      }
    })

    return unsubscribe
  }, [queryClient])
}

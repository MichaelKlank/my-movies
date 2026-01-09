import { createFileRoute, Outlet } from '@tanstack/react-router'

// Define search params schema for the movies route and all children
export interface MoviesSearchParams {
  search?: string
  watched?: 'true' | 'false'
  disc_type?: string
  is_collection?: 'true' | 'false'
}

export const Route = createFileRoute('/movies')({
  validateSearch: (search: Record<string, unknown>): MoviesSearchParams => {
    return {
      search: typeof search.search === 'string' ? search.search : undefined,
      watched: search.watched === 'true' || search.watched === 'false' ? search.watched : undefined,
      disc_type: typeof search.disc_type === 'string' ? search.disc_type : undefined,
      is_collection: search.is_collection === 'true' || search.is_collection === 'false' ? search.is_collection : undefined,
    }
  },
  component: MoviesLayout,
})

function MoviesLayout() {
  return <Outlet />
}

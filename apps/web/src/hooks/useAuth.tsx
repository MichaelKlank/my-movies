import { createContext, useContext, useState, useEffect, useCallback, ReactNode } from 'react'
import { useQueryClient } from '@tanstack/react-query'
import { api, User } from '@/lib/api'
import { wsClient } from '@/lib/ws'

interface AuthContextType {
  user: User | null
  isLoading: boolean
  isAuthenticated: boolean
  login: (username: string, password: string) => Promise<void>
  register: (username: string, email: string, password: string) => Promise<void>
  logout: () => void
  updateUser: (user: User) => void
}

const AuthContext = createContext<AuthContextType | null>(null)

export function AuthProvider({ children }: { children: ReactNode }) {
  const [user, setUser] = useState<User | null>(null)
  const [isLoading, setIsLoading] = useState(true)
  const queryClient = useQueryClient()

  // Check for existing session on mount
  useEffect(() => {
    const token = api.getToken()
    if (token) {
      api.me()
        .then(user => {
          setUser(user)
          wsClient.connect()
        })
        .catch(() => {
          api.logout()
        })
        .finally(() => setIsLoading(false))
      return
    }
    setIsLoading(false)
  }, [])

  const login = useCallback(async (username: string, password: string) => {
    const result = await api.login(username, password)
    setUser(result.user)
    wsClient.connect()
  }, [])

  const register = useCallback(async (username: string, email: string, password: string) => {
    const result = await api.register(username, email, password)
    setUser(result.user)
    wsClient.connect()
  }, [])

  const logout = useCallback(() => {
    api.logout()
    wsClient.disconnect()
    queryClient.clear() // Clear all cached data when switching users
    setUser(null)
  }, [queryClient])

  const updateUser = useCallback((updatedUser: User) => {
    setUser(updatedUser)
  }, [])

  return (
    <AuthContext.Provider
      value={{
        user,
        isLoading,
        isAuthenticated: !!user,
        login,
        register,
        logout,
        updateUser,
      }}
    >
      {children}
    </AuthContext.Provider>
  )
}

export function useAuth() {
  const context = useContext(AuthContext)
  if (!context) {
    throw new Error('useAuth must be used within an AuthProvider')
  }
  return context
}

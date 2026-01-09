import { createFileRoute, redirect } from '@tanstack/react-router'
import { useState } from 'react'
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query'
import {
  Users,
  Shield,
  ShieldOff,
  Trash2,
  Key,
  AlertCircle,
  Loader2,
  UserCircle,
  Calendar,
  Mail,
  UserPlus,
  Copy,
  Check,
  LayoutGrid,
  List,
} from 'lucide-react'
import { api, UserWithDate } from '@/lib/api'
import { useAuth } from '@/hooks/useAuth'
import { useI18n } from '@/hooks/useI18n'
import { Avatar } from '@/components/Avatar'

export const Route = createFileRoute('/users')({
  beforeLoad: ({ context }) => {
    if (!context.auth.isAuthenticated) {
      throw redirect({ to: '/login' })
    }
  },
  component: UsersPage,
})

function UsersPage() {
  const { user } = useAuth()
  const { t } = useI18n()
  const [showCreateModal, setShowCreateModal] = useState(false)

  // Only admins can access users
  if (user?.role !== 'admin') {
    return (
      <div className="container mx-auto px-4 py-8">
        <div className="rounded-lg border border-destructive/50 bg-destructive/10 p-6 text-center">
          <AlertCircle className="mx-auto h-12 w-12 text-destructive" />
          <h2 className="mt-4 text-xl font-semibold">{t('users.accessDenied')}</h2>
          <p className="mt-2 text-muted-foreground">
            {t('users.onlyAdmins')}
          </p>
        </div>
      </div>
    )
  }

  const { data: users, isLoading } = useQuery({
    queryKey: ['users'],
    queryFn: () => api.getUsers(),
    staleTime: 0, // Always refetch when navigating to this page
  })

  if (isLoading) {
    return (
      <div className="container mx-auto px-4 py-8">
        <div className="flex items-center justify-center">
          <Loader2 className="h-8 w-8 animate-spin" />
        </div>
      </div>
    )
  }

  return (
    <div className="container mx-auto px-4 py-8">
      <div className="mb-8 flex flex-col sm:flex-row sm:items-center sm:justify-between gap-4">
        <div>
          <h1 className="flex items-center gap-3 text-3xl font-bold">
            <Users className="h-8 w-8" />
            {t('users.title')}
          </h1>
          <p className="mt-2 text-muted-foreground">
            {t('users.subtitle')}
          </p>
        </div>
        <button
          onClick={() => setShowCreateModal(true)}
          className="flex items-center justify-center gap-2 rounded-md bg-primary px-4 py-3 text-sm font-medium text-primary-foreground hover:bg-primary/90 active:bg-primary/80 min-h-touch"
        >
          <UserPlus className="h-4 w-4" />
          {t('users.createUser')}
        </button>
      </div>

      {showCreateModal && (
        <CreateUserModal onClose={() => setShowCreateModal(false)} />
      )}

      <div className="rounded-lg border">
        <table className="w-full">
          <thead>
            <tr className="border-b bg-muted/50">
              <th className="px-4 py-3 text-left text-sm font-medium">{t('users.user')}</th>
              <th className="px-4 py-3 text-left text-sm font-medium">{t('users.email')}</th>
              <th className="px-4 py-3 text-left text-sm font-medium">{t('users.role')}</th>
              <th className="px-4 py-3 text-left text-sm font-medium">{t('users.registered')}</th>
              <th className="px-4 py-3 text-right text-sm font-medium">{t('users.actions')}</th>
            </tr>
          </thead>
          <tbody>
            {users?.map((u) => (
              <UserRow key={u.id} userData={u} currentUserId={user.id} />
            ))}
          </tbody>
        </table>
      </div>

      {users?.length === 0 && (
        <div className="py-12 text-center text-muted-foreground">
          {t('users.noUsersFound')}
        </div>
      )}
    </div>
  )
}

function UserRow({ userData, currentUserId }: { userData: UserWithDate; currentUserId: string }) {
  const { t } = useI18n()
  const [showPasswordModal, setShowPasswordModal] = useState(false)
  const [showDeleteModal, setShowDeleteModal] = useState(false)

  const isCurrentUser = userData.id === currentUserId

  const updateRoleMutation = useMutation({
    mutationFn: ({ userId, role }: { userId: string; role: 'admin' | 'user' }) =>
      api.updateUserRole(userId, role),
    // WebSocket event will handle cache invalidation
  })

  const deleteMutation = useMutation({
    mutationFn: (userId: string) => api.deleteUser(userId),
    onSuccess: () => {
      // WebSocket event will handle cache invalidation
      setShowDeleteModal(false)
    },
  })

  const toggleRole = () => {
    const newRole = userData.role === 'admin' ? 'user' : 'admin'
    updateRoleMutation.mutate({ userId: userData.id, role: newRole })
  }

  return (
    <>
      <tr className="border-b last:border-0 hover:bg-muted/30">
        <td className="px-4 py-3">
          <div className="flex items-center gap-3">
            <div className="flex h-10 w-10 items-center justify-center rounded-full bg-primary/10">
              <UserCircle className="h-5 w-5 text-primary" />
            </div>
            <div>
              <div className="font-medium">{userData.username}</div>
              {isCurrentUser && (
                <span className="text-xs text-muted-foreground">{t('users.you')}</span>
              )}
            </div>
          </div>
        </td>
        <td className="px-4 py-3">
          <div className="flex items-center gap-2 text-sm text-muted-foreground">
            <Mail className="h-4 w-4" />
            {userData.email}
          </div>
        </td>
        <td className="px-4 py-3">
          <RoleBadge role={userData.role} />
        </td>
        <td className="px-4 py-3">
          <div className="flex items-center gap-2 text-sm text-muted-foreground">
            <Calendar className="h-4 w-4" />
            {new Date(userData.created_at).toLocaleDateString('de-DE')}
          </div>
        </td>
        <td className="px-4 py-3">
          <div className="flex items-center justify-end gap-2">
            <button
              onClick={toggleRole}
              disabled={isCurrentUser || updateRoleMutation.isPending}
              className="rounded-md p-2 hover:bg-muted disabled:cursor-not-allowed disabled:opacity-50"
              title={userData.role === 'admin' ? 'Zum User machen' : 'Zum Admin machen'}
            >
              {updateRoleMutation.isPending ? (
                <Loader2 className="h-4 w-4 animate-spin" />
              ) : userData.role === 'admin' ? (
                <ShieldOff className="h-4 w-4" />
              ) : (
                <Shield className="h-4 w-4" />
              )}
            </button>
            <button
              onClick={() => setShowPasswordModal(true)}
              className="rounded-md p-2 hover:bg-muted"
              title="Passwort ändern"
            >
              <Key className="h-4 w-4" />
            </button>
            <button
              onClick={() => setShowDeleteModal(true)}
              disabled={isCurrentUser}
              className="rounded-md p-2 text-destructive hover:bg-destructive/10 disabled:cursor-not-allowed disabled:opacity-50"
              title="Benutzer löschen"
            >
              <Trash2 className="h-4 w-4" />
            </button>
          </div>
        </td>
      </tr>

      {showPasswordModal && (
        <PasswordModal
          user={userData}
          onClose={() => setShowPasswordModal(false)}
        />
      )}

      {showDeleteModal && (
        <DeleteModal
          user={userData}
          onClose={() => setShowDeleteModal(false)}
          onConfirm={() => deleteMutation.mutate(userData.id)}
          isDeleting={deleteMutation.isPending}
        />
      )}
    </>
  )
}

function RoleBadge({ role }: { role: 'admin' | 'user' }) {
  if (role === 'admin') {
    return (
      <span className="inline-flex items-center gap-1 rounded-full bg-primary/10 px-2 py-1 text-xs font-medium text-primary">
        <Shield className="h-3 w-3" />
        Admin
      </span>
    )
  }
  return (
    <span className="inline-flex items-center gap-1 rounded-full bg-muted px-2 py-1 text-xs font-medium text-muted-foreground">
      <UserCircle className="h-3 w-3" />
      User
    </span>
  )
}

function PasswordModal({ user, onClose }: { user: UserWithDate; onClose: () => void }) {
  const { t } = useI18n()
  const [password, setPassword] = useState('')
  const [confirmPassword, setConfirmPassword] = useState('')
  const [error, setError] = useState('')
  const queryClient = useQueryClient()

  const mutation = useMutation({
    mutationFn: () => api.adminSetPassword(user.id, password),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['users'] })
      onClose()
    },
    onError: (err) => {
      setError(err instanceof Error ? err.message : t('settings.unknownError'))
    },
  })

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault()
    setError('')

    if (password.length < 4) {
      setError(t('resetPassword.passwordTooShort'))
      return
    }

    if (password !== confirmPassword) {
      setError(t('resetPassword.passwordsDontMatch'))
      return
    }

    mutation.mutate()
  }

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50 p-4" onClick={onClose}>
      <div className="w-full max-w-md rounded-lg bg-card p-4 md:p-6 shadow-lg" onClick={(e) => e.stopPropagation()}>
        <h3 className="text-base md:text-lg font-semibold">{t('users.setPassword')}</h3>
        <p className="mt-1 text-xs md:text-sm text-muted-foreground">
          {t('resetPassword.newPassword')} for <strong>{user.username}</strong>
        </p>

        <form onSubmit={handleSubmit} className="mt-4 space-y-4">
          {error && (
            <div className="rounded-md bg-destructive/10 p-3 text-sm text-destructive">
              {error}
            </div>
          )}

          <div>
            <label className="block text-xs md:text-sm font-medium">{t('resetPassword.newPassword')}</label>
            <input
              type="password"
              value={password}
              onChange={(e) => setPassword(e.target.value)}
              className="mt-1 w-full rounded-md border bg-background px-4 py-3 text-base md:text-sm focus:border-primary focus:outline-none focus:ring-1 focus:ring-primary min-h-touch"
              autoFocus
            />
          </div>

          <div>
            <label className="block text-xs md:text-sm font-medium">{t('resetPassword.confirmPassword')}</label>
            <input
              type="password"
              value={confirmPassword}
              onChange={(e) => setConfirmPassword(e.target.value)}
              className="mt-1 w-full rounded-md border bg-background px-4 py-3 text-base md:text-sm focus:border-primary focus:outline-none focus:ring-1 focus:ring-primary min-h-touch"
            />
          </div>

          <div className="flex flex-col sm:flex-row justify-end gap-2">
            <button
              type="button"
              onClick={onClose}
              className="flex items-center justify-center rounded-md border px-4 py-3 text-sm font-medium hover:bg-muted active:bg-muted/80 min-h-touch w-full sm:w-auto"
            >
              {t('common.cancel')}
            </button>
            <button
              type="submit"
              disabled={mutation.isPending}
              className="flex items-center justify-center rounded-md bg-primary px-4 py-3 text-sm font-medium text-primary-foreground hover:bg-primary/90 active:bg-primary/80 disabled:opacity-50 min-h-touch w-full sm:w-auto"
            >
              {mutation.isPending ? t('settings.saving') : t('users.setPassword')}
            </button>
          </div>
        </form>
      </div>
    </div>
  )
}

function DeleteModal({
  user,
  onClose,
  onConfirm,
  isDeleting,
}: {
  user: UserWithDate
  onClose: () => void
  onConfirm: () => void
  isDeleting: boolean
}) {
  const { t } = useI18n()
  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50 p-4" onClick={onClose}>
      <div className="w-full max-w-md rounded-lg bg-card p-4 md:p-6 shadow-lg" onClick={(e) => e.stopPropagation()}>
        <h3 className="text-base md:text-lg font-semibold text-destructive">{t('deleteUser.title')}</h3>
        <p className="mt-2 text-xs md:text-sm text-muted-foreground break-words">
          {t('deleteUser.confirm')} <strong>{user.username}</strong>?
        </p>
        <p className="mt-2 text-xs md:text-sm text-destructive break-words">
          {t('deleteUser.warning')}
        </p>

        <div className="mt-6 flex flex-col sm:flex-row justify-end gap-2">
          <button
            onClick={onClose}
            className="flex items-center justify-center rounded-md border px-4 py-3 text-sm font-medium hover:bg-muted active:bg-muted/80 min-h-touch w-full sm:w-auto"
          >
            {t('common.cancel')}
          </button>
          <button
            onClick={onConfirm}
            disabled={isDeleting}
            className="flex items-center justify-center rounded-md bg-destructive px-4 py-3 text-sm font-medium text-destructive-foreground hover:bg-destructive/90 active:bg-destructive/80 disabled:opacity-50 min-h-touch w-full sm:w-auto"
          >
            {isDeleting ? t('common.loading') : t('common.delete')}
          </button>
        </div>
      </div>
    </div>
  )
}

function CreateUserModal({ onClose }: { onClose: () => void }) {
  const { t } = useI18n()
  const [username, setUsername] = useState('')
  const [email, setEmail] = useState('')
  const [password, setPassword] = useState('')
  const [confirmPassword, setConfirmPassword] = useState('')
  const [useTemporaryPassword, setUseTemporaryPassword] = useState(true)
  const [error, setError] = useState('')
  const [resetLink, setResetLink] = useState<string | null>(null)
  const [copied, setCopied] = useState(false)
  const queryClient = useQueryClient()

  const mutation = useMutation({
    mutationFn: () => api.adminCreateUser(
      username,
      email,
      useTemporaryPassword ? undefined : password
    ),
    onSuccess: (result) => {
      queryClient.invalidateQueries({ queryKey: ['users'] })
      if (result.reset_token) {
        // Show reset link to admin
        const link = `${window.location.origin}/reset-password?token=${result.reset_token}`
        setResetLink(link)
      } else {
        onClose()
      }
    },
    onError: (err) => {
      setError(err instanceof Error ? err.message : t('settings.unknownError'))
    },
  })

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault()
    setError('')

    if (username.length < 2) {
      setError(t('users.usernameTooShort'))
      return
    }

    if (!email.includes('@')) {
      setError(t('users.invalidEmail'))
      return
    }

    if (!useTemporaryPassword) {
      if (password.length < 4) {
        setError(t('resetPassword.passwordTooShort'))
        return
      }
      if (password !== confirmPassword) {
        setError(t('resetPassword.passwordsDontMatch'))
        return
      }
    }

    mutation.mutate()
  }

  const copyLink = async () => {
    if (resetLink) {
      await navigator.clipboard.writeText(resetLink)
      setCopied(true)
      setTimeout(() => setCopied(false), 2000)
    }
  }

  // Show reset link after creation
  if (resetLink) {
    return (
      <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50 p-4" onClick={onClose}>
        <div className="w-full max-w-md rounded-lg bg-card p-4 md:p-6 shadow-lg" onClick={(e) => e.stopPropagation()}>
          <h3 className="text-base md:text-lg font-semibold flex items-center gap-2">
            <Check className="h-5 w-5 text-green-500" />
            {t('users.userCreated')}
          </h3>
          <p className="mt-2 text-xs md:text-sm text-muted-foreground">
            {t('users.shareResetLink')}
          </p>

          <div className="mt-4 flex gap-2">
            <input
              type="text"
              value={resetLink}
              readOnly
              className="flex-1 rounded-md border bg-muted px-3 py-2 text-xs font-mono"
            />
            <button
              onClick={copyLink}
              className="rounded-md border px-3 py-2 hover:bg-muted"
              title={t('users.copyLink')}
            >
              {copied ? <Check className="h-4 w-4 text-green-500" /> : <Copy className="h-4 w-4" />}
            </button>
          </div>

          <div className="mt-6 flex justify-end">
            <button
              onClick={onClose}
              className="flex items-center justify-center rounded-md bg-primary px-4 py-3 text-sm font-medium text-primary-foreground hover:bg-primary/90 min-h-touch"
            >
              {t('common.close')}
            </button>
          </div>
        </div>
      </div>
    )
  }

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50 p-4" onClick={onClose}>
      <div className="w-full max-w-md rounded-lg bg-card p-4 md:p-6 shadow-lg" onClick={(e) => e.stopPropagation()}>
        <h3 className="text-base md:text-lg font-semibold">{t('users.createUser')}</h3>
        <p className="mt-1 text-xs md:text-sm text-muted-foreground">
          {t('users.createUserDesc')}
        </p>

        <form onSubmit={handleSubmit} className="mt-4 space-y-4">
          {error && (
            <div className="rounded-md bg-destructive/10 p-3 text-sm text-destructive">
              {error}
            </div>
          )}

          <div>
            <label className="block text-xs md:text-sm font-medium">{t('auth.username')}</label>
            <input
              type="text"
              value={username}
              onChange={(e) => setUsername(e.target.value)}
              className="mt-1 w-full rounded-md border bg-background px-4 py-3 text-base md:text-sm focus:border-primary focus:outline-none focus:ring-1 focus:ring-primary min-h-touch"
              autoFocus
              required
            />
          </div>

          <div>
            <label className="block text-xs md:text-sm font-medium">{t('auth.email')}</label>
            <input
              type="email"
              value={email}
              onChange={(e) => setEmail(e.target.value)}
              className="mt-1 w-full rounded-md border bg-background px-4 py-3 text-base md:text-sm focus:border-primary focus:outline-none focus:ring-1 focus:ring-primary min-h-touch"
              required
            />
          </div>

          <div className="flex items-center gap-3">
            <input
              type="checkbox"
              id="useTemporaryPassword"
              checked={useTemporaryPassword}
              onChange={(e) => setUseTemporaryPassword(e.target.checked)}
              className="h-4 w-4 rounded border-gray-300"
            />
            <label htmlFor="useTemporaryPassword" className="text-xs md:text-sm">
              {t('users.generateResetLink')}
            </label>
          </div>

          {!useTemporaryPassword && (
            <>
              <div>
                <label className="block text-xs md:text-sm font-medium">{t('auth.password')}</label>
                <input
                  type="password"
                  value={password}
                  onChange={(e) => setPassword(e.target.value)}
                  className="mt-1 w-full rounded-md border bg-background px-4 py-3 text-base md:text-sm focus:border-primary focus:outline-none focus:ring-1 focus:ring-primary min-h-touch"
                  required
                />
              </div>

              <div>
                <label className="block text-xs md:text-sm font-medium">{t('resetPassword.confirmPassword')}</label>
                <input
                  type="password"
                  value={confirmPassword}
                  onChange={(e) => setConfirmPassword(e.target.value)}
                  className="mt-1 w-full rounded-md border bg-background px-4 py-3 text-base md:text-sm focus:border-primary focus:outline-none focus:ring-1 focus:ring-primary min-h-touch"
                  required
                />
              </div>
            </>
          )}

          <div className="flex flex-col sm:flex-row justify-end gap-2 pt-2">
            <button
              type="button"
              onClick={onClose}
              className="flex items-center justify-center rounded-md border px-4 py-3 text-sm font-medium hover:bg-muted active:bg-muted/80 min-h-touch w-full sm:w-auto"
            >
              {t('common.cancel')}
            </button>
            <button
              type="submit"
              disabled={mutation.isPending}
              className="flex items-center justify-center rounded-md bg-primary px-4 py-3 text-sm font-medium text-primary-foreground hover:bg-primary/90 active:bg-primary/80 disabled:opacity-50 min-h-touch w-full sm:w-auto"
            >
              {mutation.isPending ? t('common.loading') : t('users.createUser')}
            </button>
          </div>
        </form>
      </div>
    </div>
  )
}

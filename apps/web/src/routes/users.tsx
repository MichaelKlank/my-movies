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
import { Button } from '@/components/ui/Button'
import { Input } from '@/components/ui/Input'
import { Modal } from '@/components/ui/Modal'

export const Route = createFileRoute('/users')({
  beforeLoad: ({ context }) => {
    if (!context.auth.isAuthenticated) {
      throw redirect({ to: '/login' })
    }
  },
  component: UsersPage,
})

type ViewMode = 'table' | 'cards'

function UsersPage() {
  const { user } = useAuth()
  const { t } = useI18n()
  const [showCreateModal, setShowCreateModal] = useState(false)
  const [viewMode, setViewMode] = useState<ViewMode>(() => {
    const saved = localStorage.getItem('users-view-mode')
    return (saved === 'table' || saved === 'cards') ? saved : 'cards'
  })

  const toggleViewMode = (mode: ViewMode) => {
    setViewMode(mode)
    localStorage.setItem('users-view-mode', mode)
  }

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
      <div className="mb-6 flex flex-col sm:flex-row sm:items-center sm:justify-between gap-4">
        <div>
          <h1 className="flex items-center gap-3 text-2xl sm:text-3xl font-bold">
            <Users className="h-7 w-7 sm:h-8 sm:w-8" />
            {t('users.title')}
          </h1>
          <p className="mt-1 text-sm text-muted-foreground">
            {t('users.subtitle')}
          </p>
        </div>
        <div className="flex items-center gap-2">
          {/* View mode toggle */}
          <div className="flex rounded-md border bg-muted/50 p-1">
            <button
              onClick={() => toggleViewMode('cards')}
              className={`flex items-center justify-center h-8 w-8 rounded transition-colors ${
                viewMode === 'cards' ? 'bg-background shadow-sm' : 'hover:bg-background/50'
              }`}
              title={t('users.cardView')}
            >
              <LayoutGrid className="h-4 w-4" />
            </button>
            <button
              onClick={() => toggleViewMode('table')}
              className={`flex items-center justify-center h-8 w-8 rounded transition-colors ${
                viewMode === 'table' ? 'bg-background shadow-sm' : 'hover:bg-background/50'
              }`}
              title={t('users.tableView')}
            >
              <List className="h-4 w-4" />
            </button>
          </div>
          <button
            onClick={() => setShowCreateModal(true)}
            className="flex items-center justify-center gap-2 rounded-md bg-primary px-4 py-2.5 text-sm font-medium text-primary-foreground hover:bg-primary/90 active:bg-primary/80 min-h-touch"
          >
            <UserPlus className="h-4 w-4" />
            <span className="hidden sm:inline">{t('users.createUser')}</span>
          </button>
        </div>
      </div>

      {showCreateModal && (
        <CreateUserModal onClose={() => setShowCreateModal(false)} />
      )}

      {users?.length === 0 ? (
        <div className="py-12 text-center text-muted-foreground">
          {t('users.noUsersFound')}
        </div>
      ) : viewMode === 'cards' ? (
        /* Card View */
        <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
          {users?.map((u) => (
            <UserCard key={u.id} userData={u} currentUserId={user.id} />
          ))}
        </div>
      ) : (
        /* Table View */
        <div className="rounded-lg border overflow-x-auto">
          <table className="w-full min-w-[600px]">
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

function UserCard({ userData, currentUserId }: { userData: UserWithDate; currentUserId: string }) {
  const { t } = useI18n()
  const [showPasswordModal, setShowPasswordModal] = useState(false)
  const [showDeleteModal, setShowDeleteModal] = useState(false)

  const isCurrentUser = userData.id === currentUserId

  const updateRoleMutation = useMutation({
    mutationFn: ({ userId, role }: { userId: string; role: 'admin' | 'user' }) =>
      api.updateUserRole(userId, role),
  })

  const deleteMutation = useMutation({
    mutationFn: (userId: string) => api.deleteUser(userId),
    onSuccess: () => {
      setShowDeleteModal(false)
    },
  })

  const toggleRole = () => {
    const newRole = userData.role === 'admin' ? 'user' : 'admin'
    updateRoleMutation.mutate({ userId: userData.id, role: newRole })
  }

  return (
    <>
      <div className="rounded-lg border bg-card p-4 space-y-4">
        {/* User header */}
        <div className="flex items-start gap-3">
          <Avatar user={userData} size="lg" />
          <div className="flex-1 min-w-0">
            <div className="flex items-center gap-2">
              <h3 className="font-semibold truncate">{userData.username}</h3>
              {isCurrentUser && (
                <span className="text-xs text-muted-foreground">({t('users.you')})</span>
              )}
            </div>
            <p className="text-sm text-muted-foreground truncate">{userData.email}</p>
          </div>
          <RoleBadge role={userData.role} />
        </div>

        {/* User info */}
        <div className="flex items-center gap-2 text-sm text-muted-foreground">
          <Calendar className="h-4 w-4 shrink-0" />
          <span>{t('users.registered')}: {new Date(userData.created_at).toLocaleDateString('de-DE')}</span>
        </div>

        {/* Actions */}
        <div className="flex items-center gap-2 pt-2 border-t">
          <button
            onClick={toggleRole}
            disabled={isCurrentUser || updateRoleMutation.isPending}
            className="flex items-center gap-2 rounded-md px-3 py-2 text-sm hover:bg-muted disabled:cursor-not-allowed disabled:opacity-50 transition-colors"
            title={userData.role === 'admin' ? t('users.makeUser') : t('users.makeAdmin')}
          >
            {updateRoleMutation.isPending ? (
              <Loader2 className="h-4 w-4 animate-spin" />
            ) : userData.role === 'admin' ? (
              <ShieldOff className="h-4 w-4" />
            ) : (
              <Shield className="h-4 w-4" />
            )}
            <span className="hidden sm:inline">
              {userData.role === 'admin' ? t('users.makeUser') : t('users.makeAdmin')}
            </span>
          </button>
          <button
            onClick={() => setShowPasswordModal(true)}
            className="flex items-center gap-2 rounded-md px-3 py-2 text-sm hover:bg-muted transition-colors"
            title={t('users.setPassword')}
          >
            <Key className="h-4 w-4" />
            <span className="hidden sm:inline">{t('users.setPassword')}</span>
          </button>
          <div className="flex-1" />
          <button
            onClick={() => setShowDeleteModal(true)}
            disabled={isCurrentUser}
            className="flex items-center gap-2 rounded-md px-3 py-2 text-sm text-destructive hover:bg-destructive/10 disabled:cursor-not-allowed disabled:opacity-50 transition-colors"
            title={t('common.delete')}
          >
            <Trash2 className="h-4 w-4" />
          </button>
        </div>
      </div>

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
    <Modal
      open={true}
      onClose={onClose}
      title={t('users.setPassword')}
      description={`${t('resetPassword.newPassword')} for ${user.username}`}
    >
      <form onSubmit={handleSubmit} className="space-y-4">
        {error && (
          <div className="rounded-md bg-destructive/10 p-3 text-sm text-destructive">
            {error}
          </div>
        )}

        <div>
          <label className="block text-xs md:text-sm font-medium">{t('resetPassword.newPassword')}</label>
          <Input
            type="password"
            value={password}
            onChange={(e) => setPassword(e.target.value)}
            className="mt-1"
            autoFocus
          />
        </div>

        <div>
          <label className="block text-xs md:text-sm font-medium">{t('resetPassword.confirmPassword')}</label>
          <Input
            type="password"
            value={confirmPassword}
            onChange={(e) => setConfirmPassword(e.target.value)}
            className="mt-1"
          />
        </div>

        <Modal.Footer>
          <Button type="button" variant="outline" onClick={onClose}>
            {t('common.cancel')}
          </Button>
          <Button type="submit" isLoading={mutation.isPending}>
            {t('users.setPassword')}
          </Button>
        </Modal.Footer>
      </form>
    </Modal>
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
    <Modal open={true} onClose={onClose} title={t('deleteUser.title')}>
      <p className="text-sm text-muted-foreground">
        {t('deleteUser.confirm')} <strong>{user.username}</strong>?
      </p>
      <p className="mt-2 text-sm text-destructive">
        {t('deleteUser.warning')}
      </p>

      <Modal.Footer>
        <Button variant="outline" onClick={onClose}>
          {t('common.cancel')}
        </Button>
        <Button variant="destructive" onClick={onConfirm} isLoading={isDeleting}>
          {t('common.delete')}
        </Button>
      </Modal.Footer>
    </Modal>
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
      <Modal open={true} onClose={onClose} title={t('users.userCreated')} showCloseButton={false}>
        <div className="flex items-center gap-2 mb-4">
          <Check className="h-5 w-5 text-green-500" />
          <span className="text-sm text-muted-foreground">{t('users.shareResetLink')}</span>
        </div>

        <div className="flex gap-2">
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

        <Modal.Footer>
          <Button onClick={onClose}>{t('common.close')}</Button>
        </Modal.Footer>
      </Modal>
    )
  }

  return (
    <Modal
      open={true}
      onClose={onClose}
      title={t('users.createUser')}
      description={t('users.createUserDesc')}
    >
      <form onSubmit={handleSubmit} className="space-y-4">
        {error && (
          <div className="rounded-md bg-destructive/10 p-3 text-sm text-destructive">
            {error}
          </div>
        )}

        <div>
          <label className="block text-xs md:text-sm font-medium">{t('auth.username')}</label>
          <Input
            type="text"
            value={username}
            onChange={(e) => setUsername(e.target.value)}
            className="mt-1"
            autoFocus
            required
          />
        </div>

        <div>
          <label className="block text-xs md:text-sm font-medium">{t('auth.email')}</label>
          <Input
            type="email"
            value={email}
            onChange={(e) => setEmail(e.target.value)}
            className="mt-1"
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
              <Input
                type="password"
                value={password}
                onChange={(e) => setPassword(e.target.value)}
                className="mt-1"
                required
              />
            </div>

            <div>
              <label className="block text-xs md:text-sm font-medium">{t('resetPassword.confirmPassword')}</label>
              <Input
                type="password"
                value={confirmPassword}
                onChange={(e) => setConfirmPassword(e.target.value)}
                className="mt-1"
                required
              />
            </div>
          </>
        )}

        <Modal.Footer>
          <Button type="button" variant="outline" onClick={onClose}>
            {t('common.cancel')}
          </Button>
          <Button type="submit" isLoading={mutation.isPending}>
            {t('users.createUser')}
          </Button>
        </Modal.Footer>
      </form>
    </Modal>
  )
}

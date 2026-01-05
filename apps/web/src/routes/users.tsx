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
} from 'lucide-react'
import { api, UserWithDate } from '@/lib/api'
import { useAuth } from '@/hooks/useAuth'
import { useI18n } from '@/hooks/useI18n'

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
      <div className="mb-8">
        <h1 className="flex items-center gap-3 text-3xl font-bold">
          <Users className="h-8 w-8" />
          {t('users.title')}
        </h1>
        <p className="mt-2 text-muted-foreground">
          {t('users.subtitle')}
        </p>
      </div>

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
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50" onClick={onClose}>
      <div className="w-full max-w-md rounded-lg bg-card p-6 shadow-lg" onClick={(e) => e.stopPropagation()}>
        <h3 className="text-lg font-semibold">{t('users.setPassword')}</h3>
        <p className="mt-1 text-sm text-muted-foreground">
          {t('resetPassword.newPassword')} for <strong>{user.username}</strong>
        </p>

        <form onSubmit={handleSubmit} className="mt-4 space-y-4">
          {error && (
            <div className="rounded-md bg-destructive/10 p-3 text-sm text-destructive">
              {error}
            </div>
          )}

          <div>
            <label className="block text-sm font-medium">{t('resetPassword.newPassword')}</label>
            <input
              type="password"
              value={password}
              onChange={(e) => setPassword(e.target.value)}
              className="mt-1 w-full rounded-md border bg-background px-3 py-2 text-sm focus:border-primary focus:outline-none focus:ring-1 focus:ring-primary"
              autoFocus
            />
          </div>

          <div>
            <label className="block text-sm font-medium">{t('resetPassword.confirmPassword')}</label>
            <input
              type="password"
              value={confirmPassword}
              onChange={(e) => setConfirmPassword(e.target.value)}
              className="mt-1 w-full rounded-md border bg-background px-3 py-2 text-sm focus:border-primary focus:outline-none focus:ring-1 focus:ring-primary"
            />
          </div>

          <div className="flex justify-end gap-2">
            <button
              type="button"
              onClick={onClose}
              className="rounded-md border px-4 py-2 text-sm font-medium hover:bg-muted"
            >
              {t('common.cancel')}
            </button>
            <button
              type="submit"
              disabled={mutation.isPending}
              className="rounded-md bg-primary px-4 py-2 text-sm font-medium text-primary-foreground hover:bg-primary/90 disabled:opacity-50"
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
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50" onClick={onClose}>
      <div className="w-full max-w-md rounded-lg bg-card p-6 shadow-lg" onClick={(e) => e.stopPropagation()}>
        <h3 className="text-lg font-semibold text-destructive">{t('deleteUser.title')}</h3>
        <p className="mt-2 text-sm text-muted-foreground">
          {t('deleteUser.confirm')} <strong>{user.username}</strong>?
        </p>
        <p className="mt-2 text-sm text-destructive">
          {t('deleteUser.warning')}
        </p>

        <div className="mt-6 flex justify-end gap-2">
          <button
            onClick={onClose}
            className="rounded-md border px-4 py-2 text-sm font-medium hover:bg-muted"
          >
            Abbrechen
          </button>
          <button
            onClick={onConfirm}
            disabled={isDeleting}
            className="rounded-md bg-destructive px-4 py-2 text-sm font-medium text-destructive-foreground hover:bg-destructive/90 disabled:opacity-50"
          >
            {isDeleting ? t('common.loading') : t('common.delete')}
          </button>
        </div>
      </div>
    </div>
  )
}


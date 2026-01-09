import { ReactNode, ElementType } from 'react'
import { cn } from '@/lib/utils'

export interface CardProps {
  children: ReactNode
  className?: string
}

export function Card({ children, className }: CardProps) {
  return (
    <div className={cn('rounded-lg border bg-card p-4 md:p-6', className)}>
      {children}
    </div>
  )
}

interface CardHeaderProps {
  icon?: ElementType
  title: string
  description?: string
  action?: ReactNode
  className?: string
}

Card.Header = function CardHeader({
  icon: Icon,
  title,
  description,
  action,
  className,
}: CardHeaderProps) {
  return (
    <div className={cn('flex items-start gap-4', className)}>
      {Icon && <Icon className="h-8 w-8 text-muted-foreground shrink-0 mt-1" />}
      <div className="flex-1 min-w-0">
        <div className="flex items-start justify-between gap-4">
          <div>
            <h2 className="font-semibold">{title}</h2>
            {description && (
              <p className="mt-1 text-sm text-muted-foreground">{description}</p>
            )}
          </div>
          {action}
        </div>
      </div>
    </div>
  )
}

interface CardContentProps {
  children: ReactNode
  className?: string
}

Card.Content = function CardContent({ children, className }: CardContentProps) {
  return <div className={cn('mt-4', className)}>{children}</div>
}

interface CardFooterProps {
  children: ReactNode
  className?: string
}

Card.Footer = function CardFooter({ children, className }: CardFooterProps) {
  return (
    <div className={cn('mt-4 flex flex-wrap items-center gap-2', className)}>
      {children}
    </div>
  )
}

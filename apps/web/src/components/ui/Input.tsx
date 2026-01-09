import { forwardRef, InputHTMLAttributes } from 'react'
import { cn } from '@/lib/utils'

export interface InputProps extends InputHTMLAttributes<HTMLInputElement> {
  error?: boolean
}

const Input = forwardRef<HTMLInputElement, InputProps>(
  ({ className, type = 'text', error, ...props }, ref) => {
    return (
      <input
        type={type}
        ref={ref}
        className={cn(
          'w-full rounded-md border bg-background px-4 py-3 text-base md:text-sm',
          'focus:border-primary focus:outline-none focus:ring-1 focus:ring-primary',
          'disabled:cursor-not-allowed disabled:opacity-50',
          'min-h-touch',
          error && 'border-destructive focus:border-destructive focus:ring-destructive',
          className
        )}
        {...props}
      />
    )
  }
)

Input.displayName = 'Input'

export { Input }

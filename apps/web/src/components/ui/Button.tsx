import { forwardRef, ButtonHTMLAttributes } from 'react'
import { cn } from '@/lib/utils'

export interface ButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: 'primary' | 'secondary' | 'destructive' | 'ghost' | 'outline'
  size?: 'sm' | 'md' | 'lg'
  isLoading?: boolean
}

const Button = forwardRef<HTMLButtonElement, ButtonProps>(
  ({ className, variant = 'primary', size = 'md', isLoading, disabled, children, ...props }, ref) => {
    const baseStyles = 'inline-flex items-center justify-center gap-2 rounded-md font-medium transition-colors focus:outline-none focus:ring-2 focus:ring-offset-2 disabled:opacity-50 disabled:cursor-not-allowed'
    
    const variants = {
      primary: 'bg-primary text-primary-foreground hover:bg-primary/90 active:bg-primary/80 focus:ring-primary',
      secondary: 'bg-secondary text-secondary-foreground hover:bg-secondary/80 active:bg-secondary/70 focus:ring-secondary',
      destructive: 'bg-destructive text-destructive-foreground hover:bg-destructive/90 active:bg-destructive/80 focus:ring-destructive',
      ghost: 'hover:bg-accent hover:text-accent-foreground focus:ring-accent',
      outline: 'border border-input bg-background hover:bg-accent hover:text-accent-foreground focus:ring-accent',
    }

    const sizes = {
      sm: 'h-8 px-3 text-xs',
      md: 'h-10 px-4 py-2 text-sm min-h-touch',
      lg: 'h-12 px-6 py-3 text-base min-h-touch',
    }

    return (
      <button
        ref={ref}
        className={cn(baseStyles, variants[variant], sizes[size], className)}
        disabled={disabled || isLoading}
        {...props}
      >
        {isLoading && (
          <svg
            className="h-4 w-4 animate-spin"
            xmlns="http://www.w3.org/2000/svg"
            fill="none"
            viewBox="0 0 24 24"
          >
            <circle
              className="opacity-25"
              cx="12"
              cy="12"
              r="10"
              stroke="currentColor"
              strokeWidth="4"
            />
            <path
              className="opacity-75"
              fill="currentColor"
              d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"
            />
          </svg>
        )}
        {children}
      </button>
    )
  }
)

Button.displayName = 'Button'

export { Button }

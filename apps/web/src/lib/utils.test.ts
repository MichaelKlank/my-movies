import { describe, it, expect } from 'vitest'
import { cn } from './utils'

describe('cn utility', () => {
  it('merges class names', () => {
    const result = cn('foo', 'bar')
    expect(result).toBe('foo bar')
  })

  it('handles undefined values', () => {
    const result = cn('foo', undefined, 'bar')
    expect(result).toBe('foo bar')
  })

  it('handles conditional classes', () => {
    const isActive = true
    const result = cn('base', isActive && 'active')
    expect(result).toBe('base active')
  })

  it('handles false conditional classes', () => {
    const isActive = false
    const result = cn('base', isActive && 'active')
    expect(result).toBe('base')
  })

  it('handles object syntax', () => {
    const result = cn('base', { active: true, disabled: false })
    expect(result).toBe('base active')
  })

  it('handles array syntax', () => {
    const result = cn(['foo', 'bar'])
    expect(result).toBe('foo bar')
  })

  it('merges tailwind classes correctly', () => {
    // twMerge should handle conflicting classes
    const result = cn('px-2 py-1', 'px-4')
    expect(result).toBe('py-1 px-4')
  })

  it('handles empty inputs', () => {
    const result = cn()
    expect(result).toBe('')
  })

  it('handles null values', () => {
    const result = cn('foo', null, 'bar')
    expect(result).toBe('foo bar')
  })

  it('handles duplicate non-tailwind classes', () => {
    // clsx doesn't deduplicate, it concatenates
    const result = cn('foo', 'foo', 'bar')
    expect(result).toBe('foo foo bar')
  })

  it('merges responsive variants correctly', () => {
    const result = cn('text-sm', 'md:text-base', 'text-lg')
    expect(result).toBe('md:text-base text-lg')
  })
})

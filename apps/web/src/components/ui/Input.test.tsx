import { describe, it, expect, vi } from 'vitest'
import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { Input } from './Input'

describe('Input', () => {
  it('renders with default props', () => {
    render(<Input />)
    expect(screen.getByRole('textbox')).toBeInTheDocument()
  })

  it('renders with placeholder', () => {
    render(<Input placeholder="Enter text" />)
    expect(screen.getByPlaceholderText('Enter text')).toBeInTheDocument()
  })

  it('renders with value', () => {
    render(<Input value="test value" onChange={() => {}} />)
    expect(screen.getByDisplayValue('test value')).toBeInTheDocument()
  })

  it('handles text input', async () => {
    const handleChange = vi.fn()
    const user = userEvent.setup()
    
    render(<Input onChange={handleChange} />)
    const input = screen.getByRole('textbox')
    
    await user.type(input, 'hello')
    
    expect(handleChange).toHaveBeenCalled()
  })

  it('renders with type password', () => {
    render(<Input type="password" data-testid="password-input" />)
    const input = screen.getByTestId('password-input')
    expect(input).toHaveAttribute('type', 'password')
  })

  it('renders with type email', () => {
    render(<Input type="email" data-testid="email-input" />)
    const input = screen.getByTestId('email-input')
    expect(input).toHaveAttribute('type', 'email')
  })

  it('is disabled when disabled prop is true', () => {
    render(<Input disabled />)
    expect(screen.getByRole('textbox')).toBeDisabled()
  })

  it('does not accept input when disabled', async () => {
    const handleChange = vi.fn()
    const user = userEvent.setup()
    
    render(<Input disabled onChange={handleChange} />)
    const input = screen.getByRole('textbox')
    
    await user.type(input, 'hello')
    
    expect(handleChange).not.toHaveBeenCalled()
  })

  it('applies error styles when error prop is true', () => {
    render(<Input error />)
    const input = screen.getByRole('textbox')
    expect(input).toHaveClass('border-destructive')
  })

  it('applies custom className', () => {
    render(<Input className="custom-class" />)
    const input = screen.getByRole('textbox')
    expect(input).toHaveClass('custom-class')
  })

  it('renders with name attribute', () => {
    render(<Input name="username" />)
    const input = screen.getByRole('textbox')
    expect(input).toHaveAttribute('name', 'username')
  })

  it('renders with id attribute', () => {
    render(<Input id="my-input" />)
    const input = screen.getByRole('textbox')
    expect(input).toHaveAttribute('id', 'my-input')
  })

  it('supports autoComplete', () => {
    render(<Input autoComplete="email" />)
    const input = screen.getByRole('textbox')
    expect(input).toHaveAttribute('autoComplete', 'email')
  })
})

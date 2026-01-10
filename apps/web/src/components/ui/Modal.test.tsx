import { describe, it, expect, vi } from 'vitest'
import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { Modal } from './Modal'

describe('Modal', () => {
  it('renders nothing when open is false', () => {
    render(
      <Modal open={false} onClose={() => {}} title="Test Modal">
        <p>Modal content</p>
      </Modal>
    )
    expect(screen.queryByText('Test Modal')).not.toBeInTheDocument()
  })

  it('renders when open is true', () => {
    render(
      <Modal open={true} onClose={() => {}} title="Test Modal">
        <p>Modal content</p>
      </Modal>
    )
    expect(screen.getByText('Test Modal')).toBeInTheDocument()
    expect(screen.getByText('Modal content')).toBeInTheDocument()
  })

  it('calls onClose when close button is clicked', async () => {
    const handleClose = vi.fn()
    const user = userEvent.setup()
    
    render(
      <Modal open={true} onClose={handleClose} title="Test Modal">
        <p>Modal content</p>
      </Modal>
    )
    
    const closeButton = screen.getByRole('button', { name: /close/i })
    await user.click(closeButton)
    
    expect(handleClose).toHaveBeenCalledTimes(1)
  })

  it('does not close when modal content is clicked', async () => {
    const handleClose = vi.fn()
    const user = userEvent.setup()
    
    render(
      <Modal open={true} onClose={handleClose} title="Test Modal">
        <p>Modal content</p>
      </Modal>
    )
    
    const content = screen.getByText('Modal content')
    await user.click(content)
    
    expect(handleClose).not.toHaveBeenCalled()
  })

  it('renders with title', () => {
    render(
      <Modal open={true} onClose={() => {}} title="My Title">
        <p>Content</p>
      </Modal>
    )
    
    expect(screen.getByText('My Title')).toBeInTheDocument()
  })

  it('renders with description', () => {
    render(
      <Modal open={true} onClose={() => {}} title="Title" description="My description">
        <p>Content</p>
      </Modal>
    )
    
    expect(screen.getByText('My description')).toBeInTheDocument()
  })

  it('hides close button when showCloseButton is false', () => {
    render(
      <Modal open={true} onClose={() => {}} title="Test" showCloseButton={false}>
        <p>Content</p>
      </Modal>
    )
    
    expect(screen.queryByRole('button', { name: /close/i })).not.toBeInTheDocument()
  })

  it('renders Modal.Footer correctly', () => {
    render(
      <Modal open={true} onClose={() => {}} title="Test Modal">
        <p>Modal content</p>
        <Modal.Footer>
          <button>Cancel</button>
          <button>Confirm</button>
        </Modal.Footer>
      </Modal>
    )
    
    expect(screen.getByText('Cancel')).toBeInTheDocument()
    expect(screen.getByText('Confirm')).toBeInTheDocument()
  })

  it('applies size classes', () => {
    const { container } = render(
      <Modal open={true} onClose={() => {}} title="Test" size="lg">
        <p>Content</p>
      </Modal>
    )
    
    // Find the modal content div (has max-w-lg class)
    const modalContent = container.querySelector('.max-w-lg')
    expect(modalContent).toBeInTheDocument()
  })
})

import { useState, useEffect, useRef } from 'react'
import { Plus, ScanLine, Keyboard, Search, ArrowUp, X, Upload } from 'lucide-react'
import { useNavigate } from '@tanstack/react-router'
import { useI18n } from '@/hooks/useI18n'

interface FABProps {
  showScrollTop?: boolean
}

export function FAB({ showScrollTop = false }: FABProps) {
  const [isOpen, setIsOpen] = useState(false)
  const [showScrollButton, setShowScrollButton] = useState(false)
  const menuRef = useRef<HTMLDivElement>(null)
  const navigate = useNavigate()
  const { t } = useI18n()

  // Track scroll position for scroll-to-top button
  useEffect(() => {
    if (!showScrollTop) return

    const handleScroll = () => {
      // Check both window scroll and main container scroll
      const mainElement = document.querySelector('main')
      const scrollTop = mainElement?.scrollTop || window.scrollY || document.documentElement.scrollTop
      setShowScrollButton(scrollTop > 300)
    }

    // Listen to both window and main element scroll
    const mainElement = document.querySelector('main')
    window.addEventListener('scroll', handleScroll, { passive: true })
    mainElement?.addEventListener('scroll', handleScroll, { passive: true })
    
    // Initial check
    handleScroll()
    
    return () => {
      window.removeEventListener('scroll', handleScroll)
      mainElement?.removeEventListener('scroll', handleScroll)
    }
  }, [showScrollTop])

  // Close menu when clicking outside
  useEffect(() => {
    const handleClickOutside = (e: MouseEvent) => {
      if (menuRef.current && !menuRef.current.contains(e.target as Node)) {
        setIsOpen(false)
      }
    }

    if (isOpen) {
      document.addEventListener('mousedown', handleClickOutside)
    }
    return () => document.removeEventListener('mousedown', handleClickOutside)
  }, [isOpen])

  const scrollToTop = () => {
    // Scroll both window and main container
    const mainElement = document.querySelector('main')
    if (mainElement) {
      mainElement.scrollTo({ top: 0, behavior: 'smooth' })
    }
    window.scrollTo({ top: 0, behavior: 'smooth' })
  }

  const menuItems = [
    {
      icon: Upload,
      label: t('fab.import'),
      action: () => navigate({ to: '/import' }),
    },
    {
      icon: ScanLine,
      label: t('fab.scanBarcode'),
      action: () => navigate({ to: '/scan', search: { mode: 'camera' } }),
    },
    {
      icon: Keyboard,
      label: t('fab.enterBarcode'),
      action: () => navigate({ to: '/scan', search: { mode: 'manual' } }),
    },
    {
      icon: Search,
      label: t('fab.searchTmdb'),
      action: () => navigate({ to: '/scan', search: { mode: 'search' } }),
    },
  ]

  return (
    <div 
      ref={menuRef}
      className="fixed z-50 flex flex-col items-end gap-3 right-16 md:right-16"
      style={{
        bottom: 'calc(5rem + env(safe-area-inset-bottom, 0px))',
      }}
    >
      {/* Scroll to top button */}
      {showScrollTop && showScrollButton && (
        <button
          onClick={scrollToTop}
          className="flex h-12 w-12 items-center justify-center rounded-full bg-secondary text-secondary-foreground shadow-lg hover:bg-secondary/80 active:bg-secondary/60 transition-all"
          title={t('fab.scrollToTop')}
        >
          <ArrowUp className="h-5 w-5" />
        </button>
      )}

      {/* Menu items - shown when FAB is open */}
      {isOpen && (
        <div className="flex flex-col gap-2 items-end">
          {menuItems.map((item, index) => (
            <button
              key={index}
              onClick={() => {
                item.action()
                setIsOpen(false)
              }}
              className="flex items-center gap-3 rounded-full bg-card border shadow-lg pl-4 pr-3 py-2 hover:bg-accent active:bg-accent/80 transition-all animate-in fade-in slide-in-from-bottom-2 duration-200"
              style={{ animationDelay: `${index * 50}ms` }}
            >
              <span className="text-sm font-medium whitespace-nowrap">{item.label}</span>
              <div className="flex h-10 w-10 items-center justify-center rounded-full bg-secondary">
                <item.icon className="h-5 w-5" />
              </div>
            </button>
          ))}
        </div>
      )}

      {/* Main FAB button */}
      <button
        onClick={() => setIsOpen(!isOpen)}
        className={`flex h-14 w-14 items-center justify-center rounded-full shadow-lg transition-all duration-200 ${
          isOpen 
            ? 'bg-destructive text-destructive-foreground rotate-45' 
            : 'bg-primary text-primary-foreground hover:bg-primary/90 active:bg-primary/80'
        }`}
        title={isOpen ? t('common.close') : t('fab.addMovie')}
      >
        {isOpen ? <X className="h-6 w-6" /> : <Plus className="h-6 w-6" />}
      </button>
    </div>
  )
}

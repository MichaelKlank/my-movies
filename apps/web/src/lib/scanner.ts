import { Html5Qrcode, Html5QrcodeScannerState } from 'html5-qrcode'

// Check if running in Tauri
declare global {
  interface Window {
    __TAURI__?: {
      invoke: (cmd: string, args?: unknown) => Promise<unknown>
    }
  }
}

export function isTauri(): boolean {
  return typeof window !== 'undefined' && !!window.__TAURI__
}

export type ScanResult = {
  barcode: string
  format: string
}

export type ScanError = {
  message: string
  code?: string
}

// Browser scanner using html5-qrcode
class BrowserScanner {
  private scanner: Html5Qrcode | null = null
  private elementId: string

  constructor(elementId: string = 'scanner-container') {
    this.elementId = elementId
  }

  async start(onSuccess: (result: ScanResult) => void, onError?: (error: ScanError) => void): Promise<void> {
    try {
      this.scanner = new Html5Qrcode(this.elementId)

      const cameras = await Html5Qrcode.getCameras()
      if (cameras.length === 0) {
        throw new Error('No cameras found')
      }

      // Prefer back camera on mobile
      const backCamera = cameras.find(c => 
        c.label.toLowerCase().includes('back') || 
        c.label.toLowerCase().includes('rear')
      )
      const cameraId = backCamera?.id || cameras[0].id

      await this.scanner.start(
        cameraId,
        {
          fps: 10,
          qrbox: { width: 250, height: 150 },
          aspectRatio: 1.777,
        },
        (decodedText, decodedResult) => {
          onSuccess({
            barcode: decodedText,
            format: decodedResult.result.format?.formatName || 'unknown',
          })
        },
        (errorMessage) => {
          // Ignore "no code found" errors
          if (!errorMessage.includes('No MultiFormat Readers')) {
            onError?.({ message: errorMessage })
          }
        }
      )
    } catch (error) {
      onError?.({ message: error instanceof Error ? error.message : 'Scanner error' })
      throw error
    }
  }

  async stop(): Promise<void> {
    if (this.scanner && this.scanner.getState() === Html5QrcodeScannerState.SCANNING) {
      await this.scanner.stop()
    }
    this.scanner = null
  }

  isRunning(): boolean {
    return this.scanner?.getState() === Html5QrcodeScannerState.SCANNING
  }
}

// Tauri native scanner
class TauriScanner {
  async scan(): Promise<ScanResult> {
    if (!window.__TAURI__) {
      throw new Error('Tauri not available')
    }

    const result = await window.__TAURI__.invoke('plugin:barcode-scanner|scan', {
      windowed: false,
      formats: ['EAN_13', 'EAN_8', 'UPC_A', 'UPC_E', 'QR_CODE'],
    }) as { content: string; format: string }

    return {
      barcode: result.content,
      format: result.format,
    }
  }

  async checkPermission(): Promise<boolean> {
    if (!window.__TAURI__) return false

    const result = await window.__TAURI__.invoke('plugin:barcode-scanner|check_permission') as string
    return result === 'granted'
  }

  async requestPermission(): Promise<boolean> {
    if (!window.__TAURI__) return false

    const result = await window.__TAURI__.invoke('plugin:barcode-scanner|request_permission') as string
    return result === 'granted'
  }
}

// Export unified scanner interface
export const browserScanner = new BrowserScanner()
export const tauriScanner = new TauriScanner()

// Helper to scan with the best available method
export async function scanBarcode(): Promise<ScanResult> {
  if (isTauri()) {
    // Use native scanner on Tauri
    const hasPermission = await tauriScanner.checkPermission()
    if (!hasPermission) {
      const granted = await tauriScanner.requestPermission()
      if (!granted) {
        throw new Error('Camera permission denied')
      }
    }
    return tauriScanner.scan()
  }

  // For browser, we need to use the component-based approach
  throw new Error('Use BrowserScanner component for browser scanning')
}

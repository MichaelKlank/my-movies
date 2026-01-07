import { Html5Qrcode, Html5QrcodeScannerState } from 'html5-qrcode'

// Dynamically import Tauri plugin to avoid errors in browser
// Using type assertion to avoid TypeScript errors when package isn't installed
type BarcodeScannerModule = {
  scan: (options: {
    windowed?: boolean
    formats?: string[]
    scanArea?: {
      x: number
      y: number
      width: number
      height: number
    }
  }) => Promise<{ content: string; format: string }>
  checkPermission: () => Promise<string>
  requestPermission: () => Promise<string>
}

let barcodeScanner: BarcodeScannerModule | null = null

// Check if running in Tauri
export async function isTauri(): Promise<boolean> {
  if (typeof window === 'undefined') return false
  try {
    // Try to import the plugin - if it fails, we're not in Tauri
    // Use @vite-ignore to prevent Rollup from trying to resolve this at build time
    if (!barcodeScanner) {
      const pluginPath = '@tauri-apps/plugin-barcode-scanner'
      const module = await import(/* @vite-ignore */ pluginPath)
      barcodeScanner = module as unknown as BarcodeScannerModule
    }
    return true
  } catch {
    return false
  }
}

// Synchronous check for immediate use (may be less accurate)
export function isTauriSync(): boolean {
  return typeof window !== 'undefined' && '__TAURI__' in window
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
  private async getPlugin(): Promise<BarcodeScannerModule> {
    if (!barcodeScanner) {
      // Construct import path at runtime to prevent Rollup from resolving it statically
      // This allows the module to be externalized and loaded only in Tauri runtime
      const pluginPath = '@tauri-apps/plugin-barcode-scanner'
      const module = await import(/* @vite-ignore */ pluginPath)
      barcodeScanner = module as unknown as BarcodeScannerModule
    }
    return barcodeScanner
  }

  async scan(): Promise<ScanResult> {
    const plugin = await this.getPlugin()
    // Use windowed: true to integrate scanner into UI with transparent webview
    // This allows the scan area to be controlled by the UI layout
    const result = await plugin.scan({
      windowed: true,
      formats: ['EAN_13', 'EAN_8', 'UPC_A', 'UPC_E', 'QR_CODE'],
    })

    return {
      barcode: result.content,
      format: result.format,
    }
  }

  async checkPermission(): Promise<boolean> {
    try {
      const plugin = await this.getPlugin()
      const result = await plugin.checkPermission()
      return result === 'granted'
    } catch {
      return false
    }
  }

  async requestPermission(): Promise<boolean> {
    try {
      const plugin = await this.getPlugin()
      const result = await plugin.requestPermission()
      return result === 'granted'
    } catch {
      return false
    }
  }
}

// Export unified scanner interface
export const browserScanner = new BrowserScanner()
export const tauriScanner = new TauriScanner()

// Helper to scan with the best available method
export async function scanBarcode(): Promise<ScanResult> {
  if (await isTauri()) {
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

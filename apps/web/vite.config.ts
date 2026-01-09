import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'
import { TanStackRouterVite } from '@tanstack/router-vite-plugin'
import tailwindcss from '@tailwindcss/vite'
import path from 'path'

export default defineConfig({
  plugins: [
    react(),
    TanStackRouterVite(),
    tailwindcss(),
  ],
  resolve: {
    alias: {
      '@': path.resolve(__dirname, './src'),
    },
  },
  optimizeDeps: {
    // Exclude Tauri plugins from pre-bundling - they're only available at runtime
    exclude: ['@tauri-apps/plugin-barcode-scanner'],
  },
  build: {
    rollupOptions: {
      // Externalize Tauri plugins - they're only available at runtime in Tauri
      external: (id) => {
        // Externalize all @tauri-apps/plugin-* packages
        return id.startsWith('@tauri-apps/plugin-')
      },
    },
  },
  server: {
    host: '0.0.0.0',
    port: 5173,
    proxy: {
      '/api': {
        target: 'http://localhost:3000',
        changeOrigin: true,
      },
      '/ws': {
        target: 'ws://localhost:3000',
        ws: true,
      },
      '/uploads': {
        target: 'http://localhost:3000',
        changeOrigin: true,
      },
    },
  },
})

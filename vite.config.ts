import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'

// Tauri expects a fixed dev port. 1420 avoids clashing with the web app (5173).
export default defineConfig({
  plugins: [react()],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
  },
  build: {
    target: 'es2021',
    // Two entry HTML files: the overlay and the settings window.
    rollupOptions: {
      input: {
        overlay: 'index.html',
        settings: 'settings.html',
        toast: 'toast.html',
      },
    },
  },
})

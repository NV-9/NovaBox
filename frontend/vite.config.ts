import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'
import path from 'path'

export default defineConfig({
  plugins: [react()],
  resolve: {
    alias: {
      '@': path.resolve(__dirname, './src'),
    },
  },
  server: {
    port: 3000,
    allowedHosts: true,
    proxy: {
      '/api': {
        target: process.env.BACKEND_URL ?? 'http://localhost:8080',
        changeOrigin: true,
      },
      '/ws': {
        target: (process.env.BACKEND_URL ?? 'http://localhost:8080').replace(/^http/, 'ws'),
        ws: true,
        changeOrigin: true,
      },
    },
  },
})

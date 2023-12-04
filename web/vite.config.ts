import react from '@vitejs/plugin-react-swc';
import autoprefixer from 'autoprefixer';
import * as path from 'path';
import { defineConfig } from 'vite';

export default defineConfig({
  clearScreen: false,
  plugins: [react()],
  server: {
    strictPort: false,
    port: 3000,
    proxy: {
      '/api': {
        target: 'http://127.0.0.1:8000/',
        changeOrigin: true,
        headers: {
          'X-Forwarded-For': '1.1.1.4',
          'X-Real-Ip': '1.1.1.3',
        },
      },
      '/.well-known': {
        target: 'http://127.0.0.1:8000/',
        changeOrigin: true,
      },
      '/svg': {
        target: 'http://127.0.0.1:8000/',
        changeOrigin: true,
      },
    },
    fs: {
      allow: ['.'],
    },
  },
  envPrefix: ['VITE_'],
  assetsInclude: ['./src/shared/assets/**/*'],
  resolve: {
    alias: {
      '@scss': path.resolve(__dirname, '/src/shared/scss'),
      '@scssutils': path.resolve(__dirname, '/src/shared/scss/helpers'),
    },
  },
  build: {
    chunkSizeWarningLimit: 10000,
    rollupOptions: {
      logLevel: 'silent',
      onwarn: (warning, warn) => {
        return;
      },
    },
  },
  css: {
    postcss: {
      plugins: [autoprefixer],
    },
  },
});

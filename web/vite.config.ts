import react from '@vitejs/plugin-react';
import autoprefixer from 'autoprefixer';
import * as path from 'path';
import { defineConfig } from 'vite';
import { nodePolyfills } from 'vite-plugin-node-polyfills';

export default defineConfig({
  plugins: [
    react(),
    nodePolyfills({
      protocolImports: true,
    }),
  ],
  server: {
    port: 3000,
    proxy: {
      '/api': {
        target: 'http://127.0.0.1:8000/',
        changeOrigin: true,
      },
    },
    fs: {
      allow: ['.'],
    },
  },
  assetsInclude: ['./src/shared/fonts/**/*', './src/shared/assets/**/*'],
  resolve: {
    alias: {
      '@fonts': path.resolve(__dirname, '/src/shared/fonts'),
      '@shared': path.resolve(__dirname, '/src/shared'),
      '@scss': path.resolve(__dirname, '/src/shared/scss'),
    },
  },
  build: {
    chunkSizeWarningLimit: 10000,
  },
  css: {
    postcss: {
      plugins: [autoprefixer],
    },
  },
});

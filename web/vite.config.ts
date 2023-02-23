import react from '@vitejs/plugin-react';
import autoprefixer from 'autoprefixer';
import * as path from 'path';
import { defineConfig } from 'vite';
import { nodePolyfills } from 'vite-plugin-node-polyfills';

let buildTarget = 'modules';

if (process.env.TAURI_PLATFORM) {
  buildTarget =
    process.env.TAURI_PLATFORM == 'windows' ? 'chrome105' : 'safari13';
}

export default defineConfig({
  clearScreen: false,
  plugins: [
    react(),
    nodePolyfills({
      protocolImports: true,
    }),
  ],
  server: {
    strictPort: true,
    port: 3000,
    proxy: {
      '/api': {
        target: 'http://127.0.0.1:8000/',
        changeOrigin: true,
      },
      '/svg': {
        target: 'http://127.0.0.1:8000/',
        changeOrigin: true,
      },
      '/.well-known': {
        target: 'http://127.0.0.1:8000/',
        changeOrigin: true,
      },
    },
    fs: {
      allow: ['.'],
    },
  },
  envPrefix: ['VITE_', 'TAURI_'],
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
    // Tauri uses Chromium on Windows and WebKit on macOS and Linux
    target: buildTarget,
    // don't minify for debug builds
    minify: !process.env.TAURI_DEBUG ? 'esbuild' : false,
    // produce sourcemaps for debug builds
    sourcemap: !!process.env.TAURI_DEBUG,
  },
  css: {
    postcss: {
      plugins: [autoprefixer],
    },
  },
});

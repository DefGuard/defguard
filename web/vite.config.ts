import GlobalsPolyfills from '@esbuild-plugins/node-globals-polyfill';
import NodeModulesPolyfills from '@esbuild-plugins/node-modules-polyfill';
import react from '@vitejs/plugin-react';
import autoprefixer from 'autoprefixer';
import * as path from 'path';
import nodePolyfills from 'rollup-plugin-polyfill-node';
import { defineConfig } from 'vite';
import LoadVersion from 'vite-plugin-package-version';

export default defineConfig({
  envDir: './env',
  plugins: [react(), LoadVersion()],
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
    rollupOptions: {
      plugins: [nodePolyfills()],
    },
  },
  optimizeDeps: {
    esbuildOptions: {
      plugins: [
        NodeModulesPolyfills(),
        GlobalsPolyfills({
          process: true,
          buffer: true,
        }),
      ],
      define: {
        global: 'globalThis',
      },
    },
  },
  css: {
    postcss: {
      plugins: [autoprefixer],
    },
  },
});

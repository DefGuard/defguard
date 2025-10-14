import { paraglideVitePlugin } from '@inlang/paraglide-js';
import { defineConfig } from 'vite';
import { ViteImageOptimizer } from 'vite-plugin-image-optimizer';
import { tanstackRouter } from '@tanstack/router-plugin/vite';
import autoprefixer from 'autoprefixer';
import react from '@vitejs/plugin-react-swc';
import * as path from 'path';

const proxyTarget = 'http://127.0.0.1:8000';

// https://vite.dev/config/
export default defineConfig({
  server: {
    strictPort: true,
    port: 3000,
    cors: true,
    proxy: {
      '/api': {
        target: proxyTarget,
        changeOrigin: true,
        secure: false,
      },
      '/.well-known': {
        target: proxyTarget,
        changeOrigin: true,
        secure: false,
      },
    },
  },
  plugins: [
    paraglideVitePlugin({
      project: './project.inlang',
      outdir: './src/paraglide',
      strategy: ['localStorage', 'preferredLanguage', 'baseLocale'],
    }),
    tanstackRouter({
      target: 'react',
      autoCodeSplitting: true,
    }),
    ViteImageOptimizer({
      test: /\.(jpe?g|png|gif|tiff|webp|avif)$/i,
    }),
    react(),
  ],
  resolve: {
    alias: {
      '@scssutils': path.resolve(__dirname, './src/shared/defguard-ui/scss/global'),
    },
  },
  css: {
    preprocessorOptions: {
      scss: {
        additionalData: `@use "@scssutils" as *;\n`,
      },
    },
    postcss: {
      plugins: [
        autoprefixer({ })
      ]
    }
  },
  build: {
    chunkSizeWarningLimit: 2500,
  },
});

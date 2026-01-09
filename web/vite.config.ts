import { devtools } from '@tanstack/devtools-vite';
import { paraglideVitePlugin } from '@inlang/paraglide-js';
import { defineConfig, loadEnv, type ProxyOptions } from 'vite';
import { ViteImageOptimizer } from 'vite-plugin-image-optimizer';
import { tanstackRouter } from '@tanstack/router-plugin/vite';
import autoprefixer from 'autoprefixer';
import react from '@vitejs/plugin-react-swc';
import * as path from 'path';

const isEnvTrue = (val: string | null | undefined) => {
  if (typeof val === 'string' && val.length > 0) {
    return ['true', '1', 'yes'].includes(val.toLowerCase().trim());
  }
  return false;
};

// https://vite.dev/config/
export default defineConfig(({ mode }) => {
  const env = loadEnv(mode, process.cwd(), '');

  const proxyOptions: Record<string, string | ProxyOptions> = {};
  const proxyUpdateServiceTarget = env.UPDATE_TARGET_URL;
  const proxyApiTargetEnv = env.PROXY_API_TARGET;
  const proxyApiSecureEnv = env.PROXY_API_SECURE;

  if (mode === 'development') {
    // update service
    if (proxyUpdateServiceTarget && proxyUpdateServiceTarget.length) {
      proxyOptions['/update'] = {
        target: proxyUpdateServiceTarget,
        changeOrigin: true,
        secure: true,
        rewrite: (path) => path.replace(/^\/update/, ''),
      };
    }
    // api

    let apiTarget: string;
    const apiSecure = isEnvTrue(proxyApiSecureEnv);
    if (proxyApiTargetEnv && proxyApiTargetEnv.length) {
      apiTarget = proxyApiTargetEnv;
    } else {
      apiTarget = 'http://127.0.0.1:8000';
    }
    proxyOptions['/api'] = {
      target: apiTarget,
      changeOrigin: true,
      secure: apiSecure,
    };
    proxyOptions['/.well-known'] = {
      target: apiTarget,
      changeOrigin: true,
      secure: apiSecure,
    };
  }

  return {
    server: {
      strictPort: true,
      port: 3000,
      cors: true,
      proxy: {
        ...proxyOptions,
      },
    },
    plugins: [
      devtools(),
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
        plugins: [autoprefixer({})],
      },
    },
    build: {
      chunkSizeWarningLimit: 2500,
    },
  };
});

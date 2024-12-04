import react from '@vitejs/plugin-react-swc';
import autoprefixer from 'autoprefixer';
import * as path from 'path';
import { defineConfig, loadEnv } from 'vite';

export default ({ mode }) => {
  process.env = { ...process.env, ...loadEnv(mode, process.cwd()) };

  let proxyTarget = 'http://127.0.0.1:8000/';
  const envProxyTarget = process.env.PROXY_TARGET;

  if (envProxyTarget && envProxyTarget.length > 0) {
    proxyTarget = envProxyTarget;
  }

  return defineConfig({
    clearScreen: false,
    plugins: [react()],
    server: {
      strictPort: false,
      port: 3000,
      proxy: {
        '/api': {
          target: proxyTarget,
          changeOrigin: true,
        },
        '/.well-known': {
          target: proxyTarget,
          changeOrigin: true,
        },
        '/svg': {
          target: proxyTarget,
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
        '@scss': path.resolve(__dirname, './src/shared/scss'),
        '@scssutils': path.resolve(__dirname, './src/shared/scss/global'),
      },
    },
    build: {
      chunkSizeWarningLimit: 10000,
      rollupOptions: {
        logLevel: 'silent',
        // eslint-disable-next-line @typescript-eslint/no-unused-vars
        onwarn: (_warning, _warn) => {
          return;
        },
      },
    },
    css: {
      preprocessorOptions: {
        scss: {
          additionalData: `@use "@scssutils" as *;\n`,
        },
      },
      postcss: {
        plugins: [autoprefixer],
      },
    },
  });
};

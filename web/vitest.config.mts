import { defineConfig } from 'vitest/config';
import * as path from 'path';

export default defineConfig({
  test: {
    globals: true,
    environment: 'node',
    include: ['tests/**/*.test.ts'],
  },
  resolve: {
    alias: {
      '@scss': path.resolve(__dirname, './src/shared/scss'),
      '@scssutils': path.resolve(__dirname, './src/shared/scss/global'),
    },
  },
});

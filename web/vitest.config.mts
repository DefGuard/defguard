import * as path from 'node:path';
import { paraglideVitePlugin } from '@inlang/paraglide-js';
import { defineConfig, type Plugin } from 'vitest/config';

const appParaglideDir = path.resolve(import.meta.dirname, './src/paraglide');
const testParaglideDir = path.resolve(
  import.meta.dirname,
  './node_modules/.paraglide-test',
);

// The app build emits one module per message (~2.6k files), which every test worker
// would have to transform and evaluate on its own. Tests use a single-file-per-locale
// compile instead.
const redirectParaglide = (): Plugin => ({
  name: 'redirect-paraglide-to-test-build',
  enforce: 'pre',
  async resolveId(source, importer, options) {
    if (!source.includes('paraglide')) return null;
    const resolved = await this.resolve(source, importer, { ...options, skipSelf: true });
    if (!resolved || !resolved.id.startsWith(appParaglideDir)) return null;
    return path.join(testParaglideDir, path.relative(appParaglideDir, resolved.id));
  },
});

export default defineConfig({
  plugins: [
    paraglideVitePlugin({
      project: './project.inlang',
      outdir: testParaglideDir,
      outputStructure: 'locale-modules',
      emitGitIgnore: false,
      emitPrettierIgnore: false,
      emitReadme: false,
    }),
    redirectParaglide(),
  ],
  test: {
    globals: true,
    environment: 'jsdom',
    include: ['tests/**/*.test.{ts,tsx}'],
    setupFiles: ['tests/setup.ts'],
  },
  resolve: {
    alias: {
      '@scss': path.resolve(import.meta.dirname, './src/shared/scss'),
      '@scssutils': path.resolve(import.meta.dirname, './src/shared/scss/global'),
    },
  },
});

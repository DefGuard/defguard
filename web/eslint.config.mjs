import eslint from '@eslint/js';
import tseslint from 'typescript-eslint';
import prettierConfig from 'eslint-config-prettier';
import reactPlugin from 'eslint-plugin-react';
import reactHooks from 'eslint-plugin-react-hooks';
import reactRefresh from 'eslint-plugin-react-refresh';
import simpleImportSort from 'eslint-plugin-simple-import-sort';
import globals from 'globals';

export default tseslint.config(
  {
    ignores: [
      'dist',
      'src/i18n/formatters.ts',
      'src/i18n/i18n-*',
      'build',
      'node_modules',
      '**/svg',
    ],
  },
  eslint.configs.recommended,
  tseslint.configs.recommendedTypeChecked,
  // @ts-ignore
  reactPlugin.configs.flat.recommended,
  // @ts-ignore
  reactPlugin.configs.flat['jsx-runtime'],
  reactRefresh.configs.recommended,
  {
    languageOptions: {
      parserOptions: {
        projectService: true,
        tsconfigRootDir: import.meta.dirname,
        project: ['./tsconfig.json', './tsconfig.app.json', './tsconfig.node.json'],
        ecmaFeatures: {
          jsx: true,
        },
      },
      ecmaVersion: 'latest',
    },
  },
  {
    files: ['**/*.{js,cjs,mjs,jsx,ts,tsx,mtsx}'],
    plugins: {
      'react-hooks': reactHooks,
      'simple-import-sort': simpleImportSort,
    },
    languageOptions: {
      globals: {
        ...globals.serviceworker,
        ...globals.browser,
      },
    },
    settings: {
      react: {
        version: 'detect',
        defaultVersion: '18.2',
      },
    },
    // @ts-ignore
    rules: {
      ...reactHooks.configs.recommended.rules,
      '@typescript-eslint/unbound-method': 'off',
      'react/display-name': 'off',
      'react-hooks/rules-of-hooks': 'error',
      'react-hooks/exhaustive-deps': 'error',
      'simple-import-sort/imports': 'error',
      'simple-import-sort/exports': 'error',
    },
  },
  {
    files: ['**/*.js'],
    extends: [tseslint.configs.disableTypeChecked],
  },
  prettierConfig,
);

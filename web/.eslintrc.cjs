module.exports = {
  root: true,
  env: { browser: true, es2020: true },
  extends: [
    'eslint:recommended',
    'plugin:@typescript-eslint/recommended',
    'plugin:react-hooks/recommended',
    'plugin:react/recommended',
    'plugin:react/jsx-runtime',
    'plugin:prettier/recommended',
    'plugin:import/recommended',
    'plugin:import/typescript',
  ],
  ignorePatterns: ['dist', '.eslintrc.cjs'],
  parser: '@typescript-eslint/parser',
  parserOptions: {
    ecmaVersion: 'latest',
    sourceType: 'module',
    project: ['./tsconfig.json', './tsconfig.node.json'],
    tsconfigRootDir: __dirname,
  },
  plugins: ['react-refresh', 'react-hooks', 'simple-import-sort'],
  rules: {
    'react-refresh/only-export-components': [
      'error',
      {
        allowConstantExport: true,
      },
    ],
    'max-len': [
      'error',
      {
        code: 90,
        comments: 140,
        tabWidth: 2,
        ignorePattern: '^import .* |.*LL\\..*|.*d=.*',
        ignoreComments: true,
        ignoreRegExpLiterals: true,
        ignoreTemplateLiterals: true,
      },
    ],
    'react-hooks/rules-of-hooks': 'error',
    'react-hooks/exhaustive-deps': 'error',
    'react/prop-types': 'off',
    'react/display-name': 'off',
    semi: [
      'error',
      'always',
      {
        omitLastInOneLineBlock: false,
      },
    ],
    'prettier/prettier': [
      'error',
      {
        semi: true,
      },
    ],
    'simple-import-sort/imports': 'error',
    'react/react-in-jsx-scope': 'off',
    '@typescript-eslint/no-unused-vars': 'error',
    '@typescript-eslint/no-explicit-any': 'error',
    '@typescript-eslint/no-non-null-assertion': 'error',
    'import/no-unresolved': [
      'error',
      {
        ignore: ['@ladle/react', '.md', 'typesafe-i18n/detectors', '@hookform/devtools'],
      },
    ],
  },
  overrides: [
    {
      extends: ['plugin:@typescript-eslint/disable-type-checked'],
      files: ['./**/*.js'],
    },
  ],
  settings: {
    react: {
      version: '18.2',
    },
  },
};

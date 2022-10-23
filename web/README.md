# Defguard frontend

React.js based, web user interface for Defguard project.

## Run development server

Install dependencies

```bash
  pnpm install
```

Start the dev server

```bash
  pnpm run dev
```

You can configure API proxy by changing `target` key value under `/api` path within `vite.config.ts` file.

## Build docker image

```bash
docker build .
```

## Linting

For linting this project uses [prettier](https://www.npmjs.com/package/prettier), [eslint](https://www.npmjs.com/package/eslint) and [stylelint](https://stylelint.io/)

#### Available commands

Check linting in all supported files ( both prettier and eslint )

```bash
pnpm run lint
```

Fix all autofixable problems with eslint

```bash
pnpm run eslint-fix
```

Fix all autofixable probles with prettier

```bash
pnpm run prettier-fix
```

Fix all autofixable problems with pretter and eslint

```bash
pnpm fix-lint
```

#### Adding new SVG components

Move .svg files into `src/shared/images/svg` then use command:

```bash
pnpm parse-svgs
```

This will generate new components within `src/shared/components/svg`, they can be used as a regular components. Also this command doesn't replace or modify already existing files.

## Conventional Commits

Using [commitlint](https://commitlint.js.org/#/) with this [config](https://github.com/conventional-changelog/commitlint/tree/master/%40commitlint/config-conventional).

## Versioning

Using [standard-version](https://github.com/conventional-changelog/standard-version)
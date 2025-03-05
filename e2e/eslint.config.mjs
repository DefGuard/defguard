import { fixupConfigRules, fixupPluginRules } from "@eslint/compat";
import typescriptEslint from "@typescript-eslint/eslint-plugin";
import simpleImportSort from "eslint-plugin-simple-import-sort";
import tsParser from "@typescript-eslint/parser";
import path from "node:path";
import { fileURLToPath } from "node:url";
import js from "@eslint/js";
import { FlatCompat } from "@eslint/eslintrc";

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const compat = new FlatCompat({
    baseDirectory: __dirname,
    recommendedConfig: js.configs.recommended,
    allConfig: js.configs.all
});

export default [{
    ignores: [
        "dist",
        "**/*.scss",
        "**/*.css",
        ".next",
        "node_modules",
        "build",
        "**/*.js",
        "src/i18n/formatters.ts",
        "src/i18n/i18n-react.tsx",
    ],
}, ...fixupConfigRules(compat.extends(
    "plugin:@typescript-eslint/recommended",
    "prettier",
    "plugin:prettier/recommended",
    "plugin:import/recommended",
    "plugin:import/typescript",
)), {
    plugins: {
        "@typescript-eslint": fixupPluginRules(typescriptEslint),
        "simple-import-sort": simpleImportSort,
    },

    languageOptions: {
        parser: tsParser,
        ecmaVersion: 2022,
        sourceType: "module",
    },

    rules: {
        "max-len": ["error", {
            code: 90,
            comments: 140,
            tabWidth: 2,
            ignorePattern: "^import .*",
            ignoreComments: true,
            ignoreRegExpLiterals: true,
            ignoreTemplateLiterals: true,
        }],

        semi: ["error", "always", {
            omitLastInOneLineBlock: false,
        }],

        "prettier/prettier": ["error", {
            semi: true,
        }],

        "simple-import-sort/imports": "error",
        "@typescript-eslint/no-unused-vars": "error",
        "@typescript-eslint/no-explicit-any": "error",
        "@typescript-eslint/no-non-null-assertion": "error",
    },
}];
import js from '@eslint/js'
import globals from 'globals'
import reactHooks from 'eslint-plugin-react-hooks'
import reactRefresh from 'eslint-plugin-react-refresh'
import tseslint from 'typescript-eslint'
import { defineConfig, globalIgnores } from 'eslint/config'
import iconButtonNeedsTooltip from './eslint-rules/icon-button-needs-tooltip.js'

export default defineConfig([
  globalIgnores(['dist', 'target']),
  {
    files: ['**/*.{ts,tsx}'],
    extends: [
      js.configs.recommended,
      tseslint.configs.recommended,
      reactHooks.configs.flat.recommended,
      reactRefresh.configs.vite,
    ],
    languageOptions: {
      ecmaVersion: 2020,
      globals: globals.browser,
    },
    plugins: {
      local: { rules: { 'icon-button-needs-tooltip': iconButtonNeedsTooltip } },
    },
    rules: {
      'local/icon-button-needs-tooltip': 'error',
    },
  },
])

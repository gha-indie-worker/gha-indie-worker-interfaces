// ESLint flat config for the Node tooling in scripts/.
//
// No framework plugins: this repository has no application JavaScript, only the
// contract tooling. Run with `npm run lint:js`.
export default [
  {
    files: ['scripts/**/*.mjs'],
    languageOptions: {
      ecmaVersion: 2023,
      sourceType: 'module',
      globals: { process: 'readonly', console: 'readonly', URL: 'readonly' },
    },
    linterOptions: { reportUnusedDisableDirectives: true },
    rules: {
      'no-console': 'off',
      'no-unused-vars': ['error', { argsIgnorePattern: '^_' }],
      'no-var': 'error',
      'prefer-const': 'error',
      eqeqeq: ['error', 'always', { null: 'ignore' }],
      'no-implicit-coercion': 'error',
      'no-throw-literal': 'error',
      curly: ['error', 'multi-line'],
    },
  },
  { ignores: ['node_modules/**', 'generated/**', 'target/**'] },
];

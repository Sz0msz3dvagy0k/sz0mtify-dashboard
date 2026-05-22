import js from '@eslint/js';
import svelte from 'eslint-plugin-svelte';
import globals from 'globals';
import tseslint from 'typescript-eslint';

export default [
	{
		ignores: [
			'.svelte-kit/**',
			'build/**',
			'ios/**',
			'node_modules/**',
			'static/**'
		]
	},
	js.configs.recommended,
	...tseslint.configs.recommended,
	...svelte.configs['flat/recommended'],
	{
		files: ['**/*.{js,mjs,cjs,ts,svelte}'],
		languageOptions: {
			ecmaVersion: 'latest',
			sourceType: 'module',
			globals: {
				...globals.browser,
				...globals.node
			}
		}
	},
	{
		files: ['**/*.{ts,svelte}'],
		rules: {
			'no-undef': 'off'
		}
	},
	{
		files: ['**/*.svelte'],
		languageOptions: {
			parserOptions: {
				parser: tseslint.parser,
				extraFileExtensions: ['.svelte'],
				svelteConfig: './svelte.config.js'
			}
		},
		rules: {
			'@typescript-eslint/no-unused-vars': 'off',
			'no-unused-vars': 'off',
			'svelte/infinite-reactive-loop': 'off',
			'svelte/no-immutable-reactive-statements': 'off',
			'svelte/no-navigation-without-resolve': 'off',
			'svelte/prefer-svelte-reactivity': 'off',
			'svelte/require-each-key': 'off'
		}
	}
];

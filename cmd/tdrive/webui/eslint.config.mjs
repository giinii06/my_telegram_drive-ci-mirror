import { plugin as shadcn } from '@shadcn/lint';
import parser from 'svelte-eslint-parser';
export default [{files: ['**/*.html'], languageOptions: {parser}, plugins: {shadcn}, rules: {'shadcn/no-raw-colors': 'error', 'shadcn/no-arbitrary-values': 'error'}}];

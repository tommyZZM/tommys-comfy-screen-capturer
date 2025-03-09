import js from '@eslint/js';
import reactPlugin from 'eslint-plugin-react';
import typescriptPlugin from '@typescript-eslint/eslint-plugin';
import typescriptParser from '@typescript-eslint/parser';

export default [
  // 应用 ESLint 的推荐配置
  js.configs.recommended,
  {
    files: ['**/*.{js,mjs,jsx,ts,tsx}'], // 指定要检查的文件类型，并忽略 eslint.config.js
    languageOptions: {
      parser: typescriptParser,
      parserOptions: {
        ecmaVersion: 2020,
        sourceType: 'module',
        ecmaFeatures: {
          jsx: true,
        },
        moduleResolution: "node16"
      },
      globals: {
        browser: true, // 添加浏览器环境
        node: true,
        es6: true,
      },
    },
    plugins: {
      react: reactPlugin,
      '@typescript-eslint': typescriptPlugin,
    },
    rules: {
      'semi': 'warn', // 禁止不必要的分号
      'no-undef': 'warn', // 禁止使用未声明的变量
      'no-unused-vars': 'warn', // 禁止未使用过的变量
      'object-curly-newline': ['warn', { // 对象换行规则
        'ObjectExpression': { 'multiline': true, 'minProperties': 4, 'consistent': true },
        'ObjectPattern': { 'multiline': true, 'minProperties': 1, 'consistent': true },
        'ImportDeclaration': { 'multiline': true, 'minProperties': 4, 'consistent': true },
        'ExportDeclaration': { 'multiline': true, 'minProperties': 4, 'consistent': true },
      }],
      '@typescript-eslint/no-unused-vars': 'warn', // 禁止未使用过的变量 (TypeScript)
    },
  },
  reactPlugin.configs.flat.recommended, // 这是一个共享配置对象
  reactPlugin.configs.flat['jsx-runtime'], // 如果使用 React 17+，添加此配置
  {
    files: ['**/*.jsx', '**/*.tsx'], // 指定要检查的文件类型
    rules: {
      'react/jsx-uses-react': 'off', // 如果使用 React 17 或更高版本，并且已配置 JSX 转换，可以禁用此规则
      'react/react-in-jsx-scope': 'off', // 同上
      'react/prop-types': 'warn',
      'react/no-unknown-property': 'warn',
    },
    settings: {
      react: {
        version: 'detect', // 自动检测 React 版本
      },
    },
  },
];

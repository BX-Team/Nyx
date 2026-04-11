import path from 'node:path';
import tailwindcss from '@tailwindcss/vite';
import react from '@vitejs/plugin-react';
import { defineConfig } from 'vite';
import monacoEditorPluginModule from 'vite-plugin-monaco-editor';

const host = process.env.TAURI_DEV_HOST;

const isObjectWithDefaultFunction = (module: unknown): module is { default: typeof monacoEditorPluginModule } =>
  module != null && typeof module === 'object' && 'default' in module && typeof module.default === 'function';
const monacoEditorPlugin = isObjectWithDefaultFunction(monacoEditorPluginModule)
  ? monacoEditorPluginModule.default
  : monacoEditorPluginModule;

export default defineConfig({
  optimizeDeps: {
    include: ['monaco-yaml', 'monaco-yaml/yaml.worker', 'path-browserify'],
  },
  plugins: [
    react(),
    tailwindcss(),
    monacoEditorPlugin({
      languageWorkers: ['editorWorkerService', 'typescript', 'css'],
      customDistPath: (_, out) => `${out}/monacoeditorwork`,
      customWorkers: [
        {
          label: 'yaml',
          entry: 'monaco-yaml/yaml.worker',
        },
      ],
    }),
  ],
  resolve: {
    alias: { '@': path.resolve(__dirname, 'src') },
  },
  build: {
    outDir: 'dist',
    rollupOptions: {
      input: {
        index: 'index.html',
      },
    },
  },
  clearScreen: false,
  server: {
    port: 5173,
    strictPort: true,
    host: host || false,
    hmr: host
      ? {
          protocol: 'ws',
          host,
          port: 5174,
        }
      : undefined,
    watch: {
      ignored: ['**/src-tauri/**'],
    },
  },
});

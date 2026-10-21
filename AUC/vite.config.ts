import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';

export default defineConfig({
  plugins: [react()],
  base: './',
  build: {
    assetsDir: '',
    rollupOptions: { output: { entryFileNames: 'auc.js', assetFileNames: 'auc.[ext]' } },
  },
  server: {
    port: 3000,
    proxy: {
      '/api': 'http://127.0.0.1:3001',
      '/ws': { target: 'ws://127.0.0.1:3001', ws: true },
    },
  },
});

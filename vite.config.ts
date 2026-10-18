import { defineConfig } from 'vite';
import vue from '@vitejs/plugin-vue';

export default defineConfig({
  root: 'ui',
  plugins: [vue()],
  clearScreen: false,
  server: { port: 1420, strictPort: true, host: '127.0.0.1' },
  build: {
    outDir: '../dist',
    emptyOutDir: true,
    target: ['es2021', 'safari16'],
    chunkSizeWarningLimit: 650,
  },
});

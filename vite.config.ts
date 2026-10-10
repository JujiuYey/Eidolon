import { fileURLToPath, URL } from 'node:url';

import tailwindcss from '@tailwindcss/vite';
import vue from '@vitejs/plugin-vue';
import AutoImport from 'unplugin-auto-import/vite';
import Component from 'unplugin-vue-components/vite';
import { defineConfig } from 'vite';
import oxlintPlugin from 'vite-plugin-oxlint';

export default defineConfig({
  plugins: [
    vue(),
    tailwindcss(),
    // oxlint 接管 lint（编辑器内走 Oxc 扩展，CI/手动走 pnpm lint），dev 启动 + HMR 自动检查
    oxlintPlugin({
      lintOnStart: true,
      lintOnHotUpdate: true,
    }),
    AutoImport({
      include: [/\.[tj]sx?$/, /\.vue$/, /\.md$/],
      imports: ['vue'],
      dirs: ['src/composables/**/*.ts', 'src/enum/**/*.ts', 'src/store/**/*.ts'],
      defaultExportByFilename: true,
      dts: 'src/types/auto-import.d.ts',
    }),
    Component({
      dirs: ['src/components/ui'],
      collapseSamePrefixes: true,
      directoryAsNamespace: false,
      dts: 'src/types/auto-import-components.d.ts',
    }),
  ],
  resolve: {
    alias: {
      '@': fileURLToPath(new URL('./src', import.meta.url)),
    },
  },
  server: {
    host: '127.0.0.1',
    port: 59415,
    strictPort: true,
  },
});

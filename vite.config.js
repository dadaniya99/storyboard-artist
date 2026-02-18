import { defineConfig } from 'vite'

export default defineConfig({
  // Vite 在开发模式下会为 Tauri 正常工作
  server: {
    port: 5173,
    strictPort: true,
    watch: {
      // 2. 告诉 Vite 监听 Tauri 后端文件的更改
      ignored: ['**/src-tauri/**'],
    },
  },
  // 前端入口点
  clearScreen: false,
})

import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'

// https://vite.dev/config/
export default defineConfig({
  plugins: [react()],
  // SPA 应用类型，Vite 会自动处理路由刷新
  appType: 'spa',
  preview: {
    // 配置预览服务器端口
    port: 4173,
    // 严格模式禁用，允许访问所有 host
    strictPort: false,
  },
})

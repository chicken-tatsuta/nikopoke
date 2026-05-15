import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'
import tailwindcss from '@tailwindcss/vite'
import { execSync } from 'node:child_process'

function getAppVersion() {
  const vercelCommit = process.env.VERCEL_GIT_COMMIT_SHA
  if (vercelCommit) return vercelCommit.slice(0, 12)

  try {
    return execSync('git rev-parse --short=12 HEAD', { encoding: 'utf8' }).trim()
  } catch {
    return String(Date.now())
  }
}

// https://vite.dev/config/
export default defineConfig({
  plugins: [react(), tailwindcss()],
  define: {
    __APP_VERSION__: JSON.stringify(getAppVersion()),
  },
  optimizeDeps: {
    exclude: ['engine-rust']
  }
})

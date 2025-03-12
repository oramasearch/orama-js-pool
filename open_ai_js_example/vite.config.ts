import { resolve } from 'path'
import { defineConfig } from 'vite'
import cleanup from 'rollup-plugin-cleanup'

export default defineConfig({
    build: {
        target: 'esnext',
        emptyOutDir: true,
        minify: true,
        lib: {
            entry: resolve(__dirname, 'src/index.ts'),
            formats: ['es'],
            name: 'openai',
            fileName: 'openai',
        },
        rollupOptions: {
            external: [],
            output: {
                compact: true,
            },
            plugins: [cleanup()],
        },
    },
})
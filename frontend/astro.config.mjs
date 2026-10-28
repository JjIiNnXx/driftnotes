import { defineConfig } from 'astro/config';
import cloudflare from '@astrojs/cloudflare';
import { requestLogger } from './plugins/request-logger.mjs';

export default defineConfig({
  output: 'server',
  adapter: cloudflare({ imageService: 'passthrough' }),
  vite: {
    plugins: [requestLogger()],
    server: {
      proxy: {
        '/api': 'http://localhost:8787',
        '/images': 'http://localhost:8787',
      },
    },
  },
});

import { appendFile, mkdir, readdir, unlink } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const pluginDir = path.dirname(fileURLToPath(import.meta.url));
const LOG_DIR = process.env.FRONTEND_LOG_DIR
  ? path.resolve(process.env.FRONTEND_LOG_DIR)
  : path.resolve(pluginDir, '../../log/frontend');
const RETENTION_DAYS = 3;
const CLEANUP_INTERVAL_MS = 72 * 60 * 60 * 1000;
let cleanupTimer;

const pad = (value, width = 2) => String(value).padStart(width, '0');

function localDate(date) {
  return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())}`;
}

function localTimestamp(date) {
  const offset = -date.getTimezoneOffset();
  const sign = offset >= 0 ? '+' : '-';
  return `${localDate(date)}T${pad(date.getHours())}:${pad(date.getMinutes())}:${pad(date.getSeconds())}.${pad(date.getMilliseconds(), 3)}${sign}${pad(Math.floor(Math.abs(offset) / 60))}:${pad(Math.abs(offset) % 60)}`;
}

function writeLine(level, target, message) {
  const now = new Date();
  const file = path.join(LOG_DIR, `${localDate(now)}.log`);
  const line = `${localTimestamp(now)} [${level}] ${target}: ${message}\n`;
  appendFile(file, line, 'utf8').catch(() => {
    // Logging must never take down the dev server.
  });
}

async function cleanup() {
  try {
    const entries = await readdir(LOG_DIR);
    const cutoff = Date.now() - RETENTION_DAYS * 24 * 60 * 60 * 1000;
    await Promise.all(entries.map(async (name) => {
      const match = /^(\d{4}-\d{2}-\d{2})\.log$/.exec(name);
      if (!match) return;
      const fileTime = new Date(`${match[1]}T00:00:00`).getTime();
      if (fileTime < cutoff) {
        await unlink(path.join(LOG_DIR, name)).catch(() => {});
      }
    }));
  } catch {
    // The directory may not exist yet on the very first boot.
  }
}

/**
 * Local-only Vite plugin: writes SSR/API request logs to
 * log/frontend/YYYY-MM-DD.log, keeps three days, and prunes every 72 hours.
 * Production Cloudflare logging will be configured separately.
 */
export function requestLogger() {
  return {
    name: 'qichi-request-logger',
    apply: 'serve',
    async configureServer(server) {
      await mkdir(LOG_DIR, { recursive: true });
      await cleanup();
      writeLine('INFO', 'app', `frontend dev server log dir: ${LOG_DIR}`);
      cleanupTimer = setInterval(cleanup, CLEANUP_INTERVAL_MS);
      cleanupTimer.unref?.();
      server.httpServer?.once('close', () => clearInterval(cleanupTimer));

      server.middlewares.use((request, response, next) => {
        const url = request.url || '/';
        const accept = String(request.headers.accept || '');
        const isViteInternal = url.startsWith('/@')
          || url.startsWith('/src/')
          || url.startsWith('/node_modules/')
          || url.startsWith('/favicon');
        const looksLikeAsset = /\.[a-z0-9]+(\?|$)/i.test(url);
        const isDocument = accept.includes('text/html')
          || (!isViteInternal && !looksLikeAsset && (request.method === 'GET' || request.method === 'HEAD'));
        const interesting = isDocument
          || url.startsWith('/api')
          || url.startsWith('/images')
          || url.endsWith('.xml');
        if (!interesting) {
          next();
          return;
        }

        const started = process.hrtime.bigint();
        response.once('finish', () => {
          const elapsed = Number(process.hrtime.bigint() - started) / 1e6;
          const status = response.statusCode;
          const level = status >= 500 ? 'ERROR' : status >= 400 ? 'WARN' : 'INFO';
          const userAgent = String(request.headers['user-agent'] || '-')
            .slice(0, 300)
            .replaceAll('"', '\\"');
          const remote = request.socket?.remoteAddress || '-';
          writeLine(
            level,
            'http',
            `${request.method} ${url} -> ${status} ${elapsed.toFixed(1)}ms ip=${remote} ua="${userAgent}"`,
          );
        });
        response.once('close', () => {
          if (!response.writableEnded) {
            writeLine('WARN', 'http', `${request.method} ${url} -> aborted`);
          }
        });
        next();
      });
    },
  };
}

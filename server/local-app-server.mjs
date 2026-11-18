/**
 * DEV / BROWSER-PREVIEW ONLY SERVER
 * ---------------------------------
 * This Node server exists strictly for browser development and sandbox preview.
 * The Windows Desktop builds (Installed NSIS and Portable ZIP via Tauri v2)
 * embed `dist/` natively into `factory-atlas.exe`, require NO Node.js runtime,
 * NO terminal, and NO open local HTTP port.
 */
import http from 'node:http';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const rootDir = path.resolve(__dirname, '..');
const distDir = path.join(rootDir, 'dist');
const overridesDir = path.join(rootDir, 'assets-overrides');
const originalsDir = path.join(rootDir, 'assets-local', 'upstream-originals');
const indexFile = path.join(rootDir, 'assets-local', 'local-originals-index.json');
const visualIndexFile = path.join(rootDir, 'assets-local', 'visual-index.json');

// Default to loopback 127.0.0.1. Only bind 0.0.0.0 when HOST=0.0.0.0 is explicitly set for cloud preview.
const HOST = process.env.HOST || '127.0.0.1';
const PORT = Number(process.env.PORT || 4173);

const MIME_TYPES = {
  '.html': 'text/html; charset=utf-8',
  '.js': 'application/javascript; charset=utf-8',
  '.css': 'text/css; charset=utf-8',
  '.svg': 'image/svg+xml',
  '.png': 'image/png',
  '.ico': 'image/x-icon',
  '.json': 'application/json; charset=utf-8',
  '.txt': 'text/plain; charset=utf-8',
};

function safeResolve(baseDir, relUrl) {
  const clean = decodeURIComponent(relUrl).replace(/^\/+/, '');
  if (!clean || clean.includes('..') || clean.includes('\0') || clean.includes('\\')) {
    return null;
  }
  const target = path.resolve(baseDir, clean);
  if (target !== baseDir && !target.startsWith(baseDir + path.sep)) {
    return null;
  }
  return target;
}

function sendJson(res, status, data) {
  res.writeHead(status, {
    'Content-Type': 'application/json; charset=utf-8',
    'Cache-Control': 'no-cache',
  });
  res.end(JSON.stringify(data));
}

function serveFile(res, filePath, cacheControl = 'public, max-age=3600') {
  if (!fs.existsSync(filePath) || fs.statSync(filePath).isDirectory()) {
    res.writeHead(404);
    res.end('Not found');
    return;
  }
  const ext = path.extname(filePath).toLowerCase();
  res.writeHead(200, {
    'Content-Type': MIME_TYPES[ext] || 'application/octet-stream',
    'Cache-Control': cacheControl,
  });
  fs.createReadStream(filePath).pipe(res);
}

const server = http.createServer(async (req, res) => {
  const urlPath = (req.url || '/').split('?')[0];

  if (req.method === 'GET' && urlPath === '/api/health') {
    sendJson(res, 200, {
      ok: true,
      app: 'factory-atlas',
      mode: 'dev-browser-preview-server',
      host: HOST,
      distReady: fs.existsSync(path.join(distDir, 'index.html')),
    });
    return;
  }

  if (req.method === 'GET' && urlPath === '/api/local-originals-index') {
    if (fs.existsSync(indexFile)) {
      serveFile(res, indexFile, 'no-cache');
    } else {
      sendJson(res, 200, { copied: [], missing: [] });
    }
    return;
  }

  if (req.method === 'GET' && urlPath === '/api/visual-index') {
    if (fs.existsSync(visualIndexFile)) {
      serveFile(res, visualIndexFile, 'no-cache');
    } else {
      sendJson(res, 200, {
        generatedAt: '',
        sources: [],
        statsByNamespace: {},
        entries: [],
      });
    }
    return;
  }

  if (req.method === 'GET' && urlPath === '/api/overrides-list') {
    const found = [];
    if (fs.existsSync(overridesDir)) {
      const walk = (dir, rel = '') => {
        for (const ent of fs.readdirSync(dir, { withFileTypes: true })) {
          const nextRel = rel ? `${rel}/${ent.name}` : ent.name;
          if (ent.isDirectory()) walk(path.join(dir, ent.name), nextRel);
          else if (ent.name.endsWith('.png')) found.push(nextRel);
        }
      };
      walk(overridesDir);
    }
    sendJson(res, 200, { files: found });
    return;
  }

  if (req.method === 'POST' && urlPath === '/api/assets/override') {
    const chunks = [];
    let total = 0;
    req.on('data', (c) => {
      total += c.length;
      if (total <= 4 * 1024 * 1024) chunks.push(c);
    });
    req.on('end', () => {
      if (total > 4 * 1024 * 1024) {
        sendJson(res, 413, { error: 'Override file exceeds 4 MB limit' });
        return;
      }
      try {
        const body = JSON.parse(Buffer.concat(chunks).toString('utf8'));
        const safePath = safeResolve(overridesDir, String(body.fileName || ''));
        if (!safePath || !safePath.endsWith('.png')) {
          sendJson(res, 400, { error: 'Invalid PNG override filename' });
          return;
        }
        const match = String(body.dataUrl || '').match(/^data:image\/png;base64,([A-Za-z0-9+/=]+)$/);
        if (!match) {
          sendJson(res, 400, { error: 'Expected data:image/png;base64,...' });
          return;
        }
        const pngBuf = Buffer.from(match[1], 'base64');
        if (pngBuf.length < 8 || pngBuf[0] !== 0x89 || pngBuf[1] !== 0x50) {
          sendJson(res, 400, { error: 'Invalid PNG signature' });
          return;
        }
        fs.mkdirSync(path.dirname(safePath), { recursive: true });
        fs.writeFileSync(safePath, pngBuf);
        sendJson(res, 200, { ok: true, saved: path.relative(overridesDir, safePath) });
      } catch (err) {
        sendJson(res, 400, { error: err instanceof Error ? err.message : 'Bad request' });
      }
    });
    return;
  }

  if (req.method === 'GET' && urlPath.startsWith('/local-originals/')) {
    const target = safeResolve(originalsDir, urlPath.slice('/local-originals/'.length));
    if (!target) {
      res.writeHead(400);
      res.end('Invalid path');
      return;
    }
    serveFile(res, target);
    return;
  }

  if (req.method === 'GET' && urlPath.startsWith('/assets-overrides/')) {
    const target = safeResolve(overridesDir, urlPath.slice('/assets-overrides/'.length));
    if (!target) {
      res.writeHead(400);
      res.end('Invalid path');
      return;
    }
    serveFile(res, target, 'no-cache');
    return;
  }

  if (req.method === 'GET') {
    const reqPath = urlPath === '/' ? 'index.html' : urlPath.slice(1);
    const target = safeResolve(distDir, reqPath);
    if (target && fs.existsSync(target) && !fs.statSync(target).isDirectory()) {
      serveFile(res, target);
      return;
    }
    const indexFallback = path.join(distDir, 'index.html');
    if (fs.existsSync(indexFallback)) {
      serveFile(res, indexFallback, 'no-cache');
      return;
    }
  }

  res.writeHead(404);
  res.end('Build not found. Run "npm run build" first.');
});

server.listen(PORT, HOST, () => {
  console.log(`Factory Atlas dev/preview server listening on http://${HOST}:${PORT}`);
});

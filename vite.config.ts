import { defineConfig, type Plugin } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';
import fs from 'node:fs';
import path from 'node:path';

function localAssetsPlugin(): Plugin {
  const rootDir = process.cwd();
  const overridesDir = path.resolve(rootDir, 'assets-overrides');
  const originalsDir = path.resolve(rootDir, 'assets-local', 'upstream-originals');
  const indexFile = path.resolve(rootDir, 'assets-local', 'local-originals-index.json');
  const visualIndexFile = path.resolve(rootDir, 'assets-local', 'visual-index.json');

  return {
    name: 'factory-atlas-local-assets',
    configureServer(server) {
      server.middlewares.use((req, res, next) => {
        const url = req.url?.split('?')[0] || '';
        if (url === '/api/local-originals-index') {
          if (fs.existsSync(indexFile)) {
            res.setHeader('Content-Type', 'application/json; charset=utf-8');
            res.end(fs.readFileSync(indexFile));
            return;
          }
          res.setHeader('Content-Type', 'application/json; charset=utf-8');
          res.end(JSON.stringify({ copied: [], missing: [] }));
          return;
        }
        if (url === '/api/visual-index') {
          res.setHeader('Content-Type', 'application/json; charset=utf-8');
          if (fs.existsSync(visualIndexFile)) {
            res.end(fs.readFileSync(visualIndexFile));
          } else {
            res.end(JSON.stringify({ generatedAt: '', sources: [], statsByNamespace: {}, entries: [] }));
          }
          return;
        }
        if (url === '/api/overrides-list') {
          const found: string[] = [];
          if (fs.existsSync(overridesDir)) {
            const walk = (dir: string, rel = '') => {
              for (const ent of fs.readdirSync(dir, { withFileTypes: true })) {
                const nextRel = rel ? `${rel}/${ent.name}` : ent.name;
                if (ent.isDirectory()) walk(path.join(dir, ent.name), nextRel);
                else if (ent.name.endsWith('.png') || ent.name.endsWith('.json')) found.push(nextRel);
              }
            };
            walk(overridesDir);
          }
          res.setHeader('Content-Type', 'application/json; charset=utf-8');
          res.end(JSON.stringify({ files: found }));
          return;
        }
        if (url.startsWith('/local-originals/')) {
          const rel = decodeURIComponent(url.slice('/local-originals/'.length));
          if (rel.includes('..') || rel.includes('\0')) {
            res.statusCode = 400;
            res.end('Invalid path');
            return;
          }
          const target = path.resolve(originalsDir, rel);
          if (!target.startsWith(originalsDir + path.sep) || !fs.existsSync(target) || fs.statSync(target).isDirectory()) {
            res.statusCode = 404;
            res.end('Not found');
            return;
          }
          const ct = target.endsWith('.json') || target.endsWith('.mcmeta')
            ? 'application/json; charset=utf-8'
            : 'image/png';
          res.setHeader('Content-Type', ct);
          res.setHeader('Cache-Control', 'public, max-age=3600');
          fs.createReadStream(target).pipe(res);
          return;
        }
        if (url.startsWith('/assets-overrides/')) {
          const rel = decodeURIComponent(url.slice('/assets-overrides/'.length));
          if (rel.includes('..') || rel.includes('\0')) {
            res.statusCode = 400;
            res.end('Invalid path');
            return;
          }
          const target = path.resolve(overridesDir, rel);
          if (!target.startsWith(overridesDir + path.sep) || !fs.existsSync(target) || fs.statSync(target).isDirectory()) {
            res.statusCode = 404;
            res.end('Not found');
            return;
          }
          const ct = target.endsWith('.json') ? 'application/json; charset=utf-8' : 'image/png';
          res.setHeader('Content-Type', ct);
          res.setHeader('Cache-Control', 'no-cache');
          fs.createReadStream(target).pipe(res);
          return;
        }
        next();
      });
    },
  };
}

export default defineConfig({
  plugins: [svelte(), localAssetsPlugin()],
  server: {
    host: process.env.HOST || '127.0.0.1',
    port: 5173,
    allowedHosts: process.env.HOST === '0.0.0.0' ? true : ['localhost', '127.0.0.1'],
    cors: true,
  },
  preview: {
    host: process.env.HOST || '127.0.0.1',
    port: 4173,
    allowedHosts: process.env.HOST === '0.0.0.0' ? true : ['localhost', '127.0.0.1'],
  },
  build: {
    outDir: 'dist',
    emptyOutDir: true,
    target: 'es2022',
  },
});

import { BoundedLruCache } from './mcModelResolver';

const imageCache = new BoundedLruCache<string, HTMLImageElement>(160);
const pendingCache = new Map<string, Promise<HTMLImageElement | null>>();

export function resolveTexturePathToUrl(texturePath: string): string {
  if (texturePath.startsWith('data:') || texturePath.startsWith('/')) {
    return texturePath;
  }
  return `/local-originals/${texturePath}`;
}

export async function loadTextureImageCached(texturePath: string): Promise<HTMLImageElement | null> {
  if (typeof Image === 'undefined') return null;

  const hit = imageCache.get(texturePath);
  if (hit) return hit;

  const inflight = pendingCache.get(texturePath);
  if (inflight) return inflight;

  const url = resolveTexturePathToUrl(texturePath);
  const promise = new Promise<HTMLImageElement | null>((resolve) => {
    const img = new Image();
    img.onload = () => {
      imageCache.set(texturePath, img);
      pendingCache.delete(texturePath);
      resolve(img);
    };
    img.onerror = () => {
      pendingCache.delete(texturePath);
      resolve(null);
    };
    img.src = url;
  });

  pendingCache.set(texturePath, promise);
  return promise;
}

export async function preloadTexturesForPaths(
  paths: string[]
): Promise<Map<string, HTMLImageElement>> {
  const unique = Array.from(new Set(paths.filter(Boolean)));
  const results = await Promise.all(unique.map((p) => loadTextureImageCached(p)));
  const out = new Map<string, HTMLImageElement>();
  unique.forEach((p, idx) => {
    const img = results[idx];
    if (img) out.set(p, img);
  });
  return out;
}

export function clearTextureImageCache(): void {
  imageCache.clear();
  pendingCache.clear();
}

export function getTextureImageCacheSize(): number {
  return imageCache.size;
}

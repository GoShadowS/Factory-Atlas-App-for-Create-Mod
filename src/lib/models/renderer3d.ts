import type { FaceDir, ResolvedCuboid } from './mcModelResolver';

export interface Vec3 {
  x: number;
  y: number;
  z: number;
}

export interface Camera3D {
  yawDeg: number;
  pitchDeg: number;
  zoom: number;
  panX: number;
  panY: number;
}

export interface SceneBlockInstance {
  bx: number;
  by: number;
  bz: number;
  cuboids: ResolvedCuboid[];
  placeholderColor?: string;
}

interface ProjectedQuad {
  p0: [number, number];
  p1: [number, number];
  p2: [number, number];
  p3: [number, number];
  depth: number;
  shade: number;
  texturePath: string | null;
  uv: [number, number, number, number];
  faceRotation: number;
  fallbackColor: string;
  isWireframePlaceholder?: boolean;
}

function rotateAxisDeg(v: Vec3, origin: Vec3, axis: 'x' | 'y' | 'z', angleDeg: number): Vec3 {
  if (!angleDeg) return v;
  const rad = (angleDeg * Math.PI) / 180;
  const c = Math.cos(rad);
  const s = Math.sin(rad);
  const dx = v.x - origin.x;
  const dy = v.y - origin.y;
  const dz = v.z - origin.z;

  if (axis === 'x') {
    return { x: origin.x + dx, y: origin.y + dy * c - dz * s, z: origin.z + dy * s + dz * c };
  }
  if (axis === 'y') {
    return { x: origin.x + dx * c + dz * s, y: origin.y + dy, z: origin.z - dx * s + dz * c };
  }
  return { x: origin.x + dx * c - dy * s, y: origin.y + dx * s + dy * c, z: origin.z + dz };
}

function transformBlockVertex(
  local0to16: Vec3,
  c: ResolvedCuboid,
  worldOffset: Vec3
): Vec3 {
  // 1. Convert 0..16 Minecraft model units into 0..1 block units
  let p: Vec3 = {
    x: local0to16.x / 16,
    y: local0to16.y / 16,
    z: local0to16.z / 16,
  };

  // 2. Apply element-level rotation (e.g. cogwheel 22.5° / 45° teeth)
  if (c.elementRotation) {
    const o: Vec3 = {
      x: c.elementRotation.origin[0] / 16,
      y: c.elementRotation.origin[1] / 16,
      z: c.elementRotation.origin[2] / 16,
    };
    p = rotateAxisDeg(p, o, c.elementRotation.axis, c.elementRotation.angle);
  }

  // 3. Apply blockstate X then Y rotation (Minecraft convention: clockwise looking down -Y / +X -> negative yaw)
  const center: Vec3 = { x: 0.5, y: 0.5, z: 0.5 };
  if (c.blockRotationX) {
    p = rotateAxisDeg(p, center, 'x', -c.blockRotationX);
  }
  if (c.blockRotationY) {
    p = rotateAxisDeg(p, center, 'y', -c.blockRotationY);
  }

  return {
    x: p.x + worldOffset.x,
    y: p.y + worldOffset.y,
    z: p.z + worldOffset.z,
  };
}

function projectPoint(v: Vec3, yawRad: number, pitchRad: number): { sx: number; sy: number; depth: number } {
  const cy = Math.cos(yawRad);
  const sy = Math.sin(yawRad);
  const cp = Math.cos(pitchRad);
  const sp = Math.sin(pitchRad);

  const rx = v.x * cy - v.z * sy;
  const rz = v.x * sy + v.z * cy;
  const ry = v.y * cp - rz * sp;
  const depth = v.y * sp + rz * cp;
  return { sx: rx, sy: -ry, depth };
}

// Corner order (p0 = top-left UV, p1 = top-right UV, p2 = bottom-right UV, p3 = bottom-left UV)
// chosen so outward normal points outside the cuboid in Minecraft coordinates (+Y up, +Z south, +X east)
function getFaceCorners0to16(
  from: [number, number, number],
  to: [number, number, number],
  dir: FaceDir
): [Vec3, Vec3, Vec3, Vec3] {
  const [x1, y1, z1] = from;
  const [x2, y2, z2] = to;
  switch (dir) {
    case 'north': // -Z face, looking from -Z towards +Z: left=+X? wait: facing north (+Z->-Z), outside looking at -Z sees X2 on left, X1 on right
      return [
        { x: x2, y: y2, z: z1 },
        { x: x1, y: y2, z: z1 },
        { x: x1, y: y1, z: z1 },
        { x: x2, y: y1, z: z1 },
      ];
    case 'south': // +Z face
      return [
        { x: x1, y: y2, z: z2 },
        { x: x2, y: y2, z: z2 },
        { x: x2, y: y1, z: z2 },
        { x: x1, y: y1, z: z2 },
      ];
    case 'west': // -X face
      return [
        { x: x1, y: y2, z: z1 },
        { x: x1, y: y2, z: z2 },
        { x: x1, y: y1, z: z2 },
        { x: x1, y: y1, z: z1 },
      ];
    case 'east': // +X face
      return [
        { x: x2, y: y2, z: z2 },
        { x: x2, y: y2, z: z1 },
        { x: x2, y: y1, z: z1 },
        { x: x2, y: y1, z: z2 },
      ];
    case 'up': // +Y face
      return [
        { x: x1, y: y2, z: z1 },
        { x: x2, y: y2, z: z1 },
        { x: x2, y: y2, z: z2 },
        { x: x1, y: y2, z: z2 },
      ];
    case 'down': // -Y face
      return [
        { x: x1, y: y1, z: z2 },
        { x: x2, y: y1, z: z2 },
        { x: x2, y: y1, z: z1 },
        { x: x1, y: y1, z: z1 },
      ];
  }
}

const FACE_SHADE: Record<FaceDir, number> = {
  up: 1.0,
  north: 0.86,
  south: 0.9,
  east: 0.78,
  west: 0.72,
  down: 0.56,
};

/**
 * Draws an affine texture-mapped parallelogram `(p0, p1, p3)` onto Canvas2D.
 * Under orthographic 3D projection, every cuboid face is an exact parallelogram,
 * so `ctx.setTransform` maps pixel textures without distortion and respects `imageSmoothingEnabled = false`.
 */
function drawTexturedParallelogram(
  ctx: CanvasRenderingContext2D,
  img: HTMLImageElement,
  q: ProjectedQuad
): void {
  const iw = img.naturalWidth || 16;
  const ih = img.naturalHeight || 16;
  // Support animated vertical strips (e.g., 16x512 fluids): clamp source height to frame 0
  const frameH = ih > iw * 2 && ih % iw === 0 ? iw : ih;

  let [u1, v1, u2, v2] = q.uv;
  if (u2 < u1) [u1, u2] = [u2, u1];
  if (v2 < v1) [v1, v2] = [v2, v1];

  const sx = (u1 / 16) * iw;
  const sy = (v1 / 16) * frameH;
  const sw = Math.max(0.25, ((u2 - u1) / 16) * iw);
  const sh = Math.max(0.25, ((v2 - v1) / 16) * frameH);

  // Rotate corners if Minecraft face.rotation is 90/180/270
  const pts = [q.p0, q.p1, q.p2, q.p3];
  const steps = (((q.faceRotation || 0) / 90) % 4 + 4) % 4;
  const a0 = pts[(0 + steps) % 4];
  const a1 = pts[(1 + steps) % 4];
  const a3 = pts[(3 + steps) % 4];

  const dxU = a1[0] - a0[0];
  const dyU = a1[1] - a0[1];
  const dxV = a3[0] - a0[0];
  const dyV = a3[1] - a0[1];

  ctx.save();
  ctx.imageSmoothingEnabled = false;
  ctx.transform(dxU / sw, dyU / sw, dxV / sh, dyV / sh, a0[0], a0[1]);
  ctx.drawImage(img, sx, sy, sw, sh, 0, 0, sw, sh);

  // Directional face shading overlay (Minecraft ambient lighting approximation)
  if (q.shade < 0.99) {
    ctx.fillStyle = `rgba(0, 0, 0, ${(1 - q.shade).toFixed(3)})`;
    ctx.fillRect(0, 0, sw, sh);
  }
  ctx.restore();
}

export interface Render3DStats {
  renderedFaces: number;
  renderedCuboids: number;
  missingTextures: number;
}

/**
 * Renders an array of world-positioned block instances (1 block for Inspector,
 * or hundreds of blocks for `.nbt` schematics) with real cuboid faces and textures.
 */
export function renderBlocksScene3D(params: {
  ctx: CanvasRenderingContext2D;
  width: number;
  height: number;
  blocks: SceneBlockInstance[];
  centerWorld: Vec3;
  camera: Camera3D;
  texturesByPath: Map<string, HTMLImageElement>;
  customDataUrlImg?: HTMLImageElement | null;
}): Render3DStats {
  const { ctx, width, height, blocks, centerWorld, camera, texturesByPath, customDataUrlImg } = params;

  ctx.clearRect(0, 0, width, height);
  ctx.imageSmoothingEnabled = false;

  const yawRad = (camera.yawDeg * Math.PI) / 180;
  const pitchRad = (camera.pitchDeg * Math.PI) / 180;
  const cx = width / 2 + camera.panX;
  const cy = height / 2 + camera.panY;
  const scale = camera.zoom;

  const quads: ProjectedQuad[] = [];
  let renderedCuboids = 0;

  for (const blk of blocks) {
    const offset: Vec3 = {
      x: blk.bx - centerWorld.x,
      y: blk.by - centerWorld.y,
      z: blk.bz - centerWorld.z,
    };

    if (blk.cuboids.length === 0) {
      // Honest wireframe/placeholder box for missing/unsupported/procedural blocks
      const placeholderCuboid: ResolvedCuboid = {
        from: [1, 1, 1],
        to: [15, 15, 15],
        blockRotationX: 0,
        blockRotationY: 0,
        faces: {
          up: { dir: 'up', texturePath: null, textureRef: null, uv: [0, 0, 16, 16], rotation: 0 },
          north: { dir: 'north', texturePath: null, textureRef: null, uv: [0, 0, 16, 16], rotation: 0 },
          east: { dir: 'east', texturePath: null, textureRef: null, uv: [0, 0, 16, 16], rotation: 0 },
          south: { dir: 'south', texturePath: null, textureRef: null, uv: [0, 0, 16, 16], rotation: 0 },
          west: { dir: 'west', texturePath: null, textureRef: null, uv: [0, 0, 16, 16], rotation: 0 },
        },
      };
      pushCuboidQuads(placeholderCuboid, blk.placeholderColor || '#475569', true);
      continue;
    }

    for (const c of blk.cuboids) {
      renderedCuboids++;
      pushCuboidQuads(c, blk.placeholderColor || '#64748b', false);
    }

    function pushCuboidQuads(c: ResolvedCuboid, fallbackColor: string, isWireframePlaceholder: boolean) {
      for (const dir of ['north', 'south', 'east', 'west', 'up', 'down'] as FaceDir[]) {
        const f = c.faces[dir];
        if (!f) continue;
        const corners = getFaceCorners0to16(c.from, c.to, dir).map((v) =>
          projectPoint(transformBlockVertex(v, c, offset), yawRad, pitchRad)
        );
        const p0: [number, number] = [cx + corners[0].sx * scale, cy + corners[0].sy * scale];
        const p1: [number, number] = [cx + corners[1].sx * scale, cy + corners[1].sy * scale];
        const p2: [number, number] = [cx + corners[2].sx * scale, cy + corners[2].sy * scale];
        const p3: [number, number] = [cx + corners[3].sx * scale, cy + corners[3].sy * scale];

        // Screen-space 2D cross product for backface culling
        const cross = (p1[0] - p0[0]) * (p2[1] - p0[1]) - (p1[1] - p0[1]) * (p2[0] - p0[0]);
        if (cross <= 0.01) continue;

        const depth = (corners[0].depth + corners[1].depth + corners[2].depth + corners[3].depth) / 4;
        quads.push({
          p0,
          p1,
          p2,
          p3,
          depth,
          shade: FACE_SHADE[dir],
          texturePath: f.texturePath,
          uv: f.uv,
          faceRotation: f.rotation,
          fallbackColor,
          isWireframePlaceholder,
        });
      }
    }
  }

  // Painter's algorithm: far -> near
  quads.sort((a, b) => a.depth - b.depth);

  let missingTextures = 0;
  for (const q of quads) {
    const img =
      customDataUrlImg ||
      (q.texturePath ? texturesByPath.get(q.texturePath) : undefined);

    if (img && img.complete && img.naturalWidth > 0 && !q.isWireframePlaceholder) {
      drawTexturedParallelogram(ctx, img, q);
    } else {
      if (!q.isWireframePlaceholder && q.texturePath) missingTextures++;
      ctx.beginPath();
      ctx.moveTo(q.p0[0], q.p0[1]);
      ctx.lineTo(q.p1[0], q.p1[1]);
      ctx.lineTo(q.p2[0], q.p2[1]);
      ctx.lineTo(q.p3[0], q.p3[1]);
      ctx.closePath();
      if (q.isWireframePlaceholder) {
        ctx.fillStyle = q.fallbackColor + '55';
        ctx.fill();
        ctx.strokeStyle = q.fallbackColor;
        ctx.lineWidth = 0.9;
        ctx.stroke();
      } else {
        ctx.fillStyle = q.fallbackColor;
        ctx.fill();
        ctx.fillStyle = `rgba(0,0,0,${(1 - q.shade).toFixed(2)})`;
        ctx.fill();
        ctx.strokeStyle = 'rgba(15,23,42,0.45)';
        ctx.lineWidth = 0.7;
        ctx.stroke();
      }
    }
  }

  return {
    renderedFaces: quads.length,
    renderedCuboids,
    missingTextures,
  };
}

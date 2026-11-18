import { Gunzip, gzipSync } from 'fflate';
import type { NbtBlockEntry, NbtPaletteEntry, ParsedSchematic } from '../types/atlas';

export const NBT_LIMITS = {
  maxCompressedBytes: 8 * 1024 * 1024, // 8 MiB
  maxUncompressedBytes: 32 * 1024 * 1024, // 32 MiB
  maxDepth: 64,
  maxTotalTags: 2_500_000,
  maxPaletteSize: 65536,
  maxBlocksCount: 500_000,
} as const;

type NbtTagValue =
  | number
  | bigint
  | string
  | number[]
  | bigint[]
  | NbtTagValue[]
  | { [key: string]: NbtTagValue };

const TAG_END = 0;
const TAG_BYTE = 1;
const TAG_SHORT = 2;
const TAG_INT = 3;
const TAG_LONG = 4;
const TAG_FLOAT = 5;
const TAG_DOUBLE = 6;
const TAG_BYTE_ARRAY = 7;
const TAG_STRING = 8;
const TAG_LIST = 9;
const TAG_COMPOUND = 10;
const TAG_INT_ARRAY = 11;
const TAG_LONG_ARRAY = 12;

/**
 * Safely inflates GZIP bytes with hard streaming output byte cap to prevent gzip bombs.
 */
export function safeGunzipIfNeeded(input: Uint8Array): Uint8Array {
  if (input.byteLength === 0) {
    throw new Error('Файл .nbt пуст.');
  }
  if (input.byteLength > NBT_LIMITS.maxCompressedBytes) {
    throw new Error(
      `Размер файла (${(input.byteLength / (1024 * 1024)).toFixed(2)} МБ) превышает лимит 8 МБ.`
    );
  }

  const isGzip = input.byteLength >= 2 && input[0] === 0x1f && input[1] === 0x8b;
  if (!isGzip) {
    if (input.byteLength > NBT_LIMITS.maxUncompressedBytes) {
      throw new Error('Несжатый NBT превышает лимит 32 МБ.');
    }
    return input;
  }

  const chunks: Uint8Array[] = [];
  let totalBytes = 0;
  let streamError: Error | null = null;

  const gunzip = new Gunzip((chunk) => {
    totalBytes += chunk.byteLength;
    if (totalBytes > NBT_LIMITS.maxUncompressedBytes) {
      streamError = new Error(
        `Обнаружена возможная GZIP-бомба: распакованный размер превысил ${
          NBT_LIMITS.maxUncompressedBytes / (1024 * 1024)
        } МБ.`
      );
      return;
    }
    chunks.push(chunk);
  });

  // Push in 64 KiB slices so bomb detection triggers before inflating huge payloads
  const sliceSize = 65536;
  for (let off = 0; off < input.byteLength; off += sliceSize) {
    if (streamError) break;
    const end = Math.min(input.byteLength, off + sliceSize);
    gunzip.push(input.subarray(off, end), end === input.byteLength);
  }

  if (streamError) throw streamError;

  const out = new Uint8Array(totalBytes);
  let pos = 0;
  for (const c of chunks) {
    out.set(c, pos);
    pos += c.byteLength;
  }
  return out;
}

class NbtCursor {
  private view: DataView;
  private bytes: Uint8Array;
  private offset = 0;
  private tagsRead = 0;
  private decoder = new TextDecoder('utf-8', { fatal: false });

  constructor(bytes: Uint8Array) {
    this.bytes = bytes;
    this.view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
  }

  private ensure(n: number) {
    if (this.offset + n > this.bytes.byteLength) {
      throw new Error(`Неожиданный конец бинарного NBT-потока на смещении ${this.offset}.`);
    }
  }

  private countTag() {
    this.tagsRead++;
    if (this.tagsRead > NBT_LIMITS.maxTotalTags) {
      throw new Error(`Превышен безопасный лимит числа NBT-тегов (${NBT_LIMITS.maxTotalTags}).`);
    }
  }

  readByte(): number {
    this.ensure(1);
    return this.view.getInt8(this.offset++);
  }

  readUByte(): number {
    this.ensure(1);
    return this.view.getUint8(this.offset++);
  }

  readShort(): number {
    this.ensure(2);
    const v = this.view.getInt16(this.offset, false);
    this.offset += 2;
    return v;
  }

  readUShort(): number {
    this.ensure(2);
    const v = this.view.getUint16(this.offset, false);
    this.offset += 2;
    return v;
  }

  readInt(): number {
    this.ensure(4);
    const v = this.view.getInt32(this.offset, false);
    this.offset += 4;
    return v;
  }

  readLong(): bigint {
    this.ensure(8);
    const v = this.view.getBigInt64(this.offset, false);
    this.offset += 8;
    return v;
  }

  readFloat(): number {
    this.ensure(4);
    const v = this.view.getFloat32(this.offset, false);
    this.offset += 4;
    return v;
  }

  readDouble(): number {
    this.ensure(8);
    const v = this.view.getFloat64(this.offset, false);
    this.offset += 8;
    return v;
  }

  readString(): string {
    const len = this.readUShort();
    this.ensure(len);
    const slice = this.bytes.subarray(this.offset, this.offset + len);
    this.offset += len;
    return this.decoder.decode(slice);
  }

  readRoot(): { name: string; value: Record<string, NbtTagValue> } {
    const rootType = this.readUByte();
    if (rootType !== TAG_COMPOUND) {
      throw new Error(`Неверный корневой тег NBT (ожидался TAG_Compound=10, получено ${rootType}).`);
    }
    const name = this.readString();
    const val = this.readPayload(TAG_COMPOUND, 0) as Record<string, NbtTagValue>;
    return { name, value: val };
  }

  readPayload(tagType: number, depth: number): NbtTagValue {
    if (depth > NBT_LIMITS.maxDepth) {
      throw new Error(`Превышена максимальная глубина вложенности NBT (${NBT_LIMITS.maxDepth}).`);
    }
    this.countTag();

    switch (tagType) {
      case TAG_BYTE:
        return this.readByte();
      case TAG_SHORT:
        return this.readShort();
      case TAG_INT:
        return this.readInt();
      case TAG_LONG:
        return this.readLong();
      case TAG_FLOAT:
        return this.readFloat();
      case TAG_DOUBLE:
        return this.readDouble();
      case TAG_BYTE_ARRAY: {
        const len = this.readInt();
        if (len < 0 || len > NBT_LIMITS.maxUncompressedBytes) {
          throw new Error(`Некорректный размер TAG_Byte_Array: ${len}`);
        }
        this.ensure(len);
        const arr = Array.from(this.bytes.subarray(this.offset, this.offset + len));
        this.offset += len;
        return arr;
      }
      case TAG_STRING:
        return this.readString();
      case TAG_LIST: {
        const itemType = this.readUByte();
        const len = this.readInt();
        if (len < 0 || len > 1_000_000) {
          throw new Error(`Некорректная длина TAG_List: ${len}`);
        }
        const list: NbtTagValue[] = [];
        for (let i = 0; i < len; i++) {
          list.push(this.readPayload(itemType, depth + 1));
        }
        return list;
      }
      case TAG_COMPOUND: {
        const map: Record<string, NbtTagValue> = {};
        while (true) {
          const childType = this.readUByte();
          if (childType === TAG_END) break;
          const childName = this.readString();
          map[childName] = this.readPayload(childType, depth + 1);
        }
        return map;
      }
      case TAG_INT_ARRAY: {
        const len = this.readInt();
        if (len < 0 || len > 1_000_000) {
          throw new Error(`Некорректная длина TAG_Int_Array: ${len}`);
        }
        const arr: number[] = [];
        for (let i = 0; i < len; i++) arr.push(this.readInt());
        return arr;
      }
      case TAG_LONG_ARRAY: {
        const len = this.readInt();
        if (len < 0 || len > 1_000_000) {
          throw new Error(`Некорректная длина TAG_Long_Array: ${len}`);
        }
        const arr: bigint[] = [];
        for (let i = 0; i < len; i++) arr.push(this.readLong());
        return arr;
      }
      default:
        throw new Error(`Неизвестный тип тега NBT: ${tagType}`);
    }
  }
}

/**
 * Parses a Create / Minecraft StructureTemplate `.nbt` schematic securely.
 * Explicitly checks format boundaries (rejects .litematic / Sponge .schem cleanly).
 */
export function parseCreateSchematicNbt(
  rawBytes: Uint8Array,
  fileName: string,
  options?: { isSyntheticDemo?: boolean }
): ParsedSchematic {
  const lower = fileName.toLowerCase();
  if (lower.endsWith('.litematic')) {
    throw new Error(
      'Формат .litematic (Litematica) пока не поддерживается в первой версии. Используйте схемы Create в формате StructureTemplate (.nbt).'
    );
  }
  if (lower.endsWith('.schem') || lower.endsWith('.schematic')) {
    throw new Error(
      'Формат WorldEdit/Sponge (.schem / .schematic) не поддерживается. Откройте файл .nbt, сохранённый в Create (Schematic & Quill).'
    );
  }

  const uncompressed = safeGunzipIfNeeded(rawBytes);
  const cursor = new NbtCursor(uncompressed);
  const { name: rootName, value: root } = cursor.readRoot();

  // Distinguish Litematica / Sponge Schem even if renamed to .nbt
  if ('Regions' in root && 'MinecraftDataVersion' in root && !('blocks' in root)) {
    throw new Error(
      'Файл имеет структуру Litematica (Regions), а не Create StructureTemplate (.nbt).'
    );
  }
  if (rootName === 'Schematic' || ('Palette' in root && 'BlockData' in root && !('blocks' in root))) {
    throw new Error(
      'Файл имеет структуру Sponge Schematic (.schem), а не Create StructureTemplate (.nbt).'
    );
  }

  const sizeRaw = root.size;
  if (!Array.isArray(sizeRaw) || sizeRaw.length !== 3) {
    throw new Error(
      'Неподдерживаемая структура NBT: отсутствует тег size [X, Y, Z] формата Minecraft StructureTemplate / Create .nbt.'
    );
  }
  const size: [number, number, number] = [
    Number(sizeRaw[0]) || 0,
    Number(sizeRaw[1]) || 0,
    Number(sizeRaw[2]) || 0,
  ];
  if (size[0] <= 0 || size[1] <= 0 || size[2] <= 0) {
    throw new Error(`Некорректные габариты схемы: [${size.join(', ')}].`);
  }

  const dataVersion = typeof root.DataVersion === 'number' ? root.DataVersion : null;

  let rawPalette: Array<Record<string, NbtTagValue>> = [];
  if (Array.isArray(root.palette)) {
    rawPalette = root.palette as Array<Record<string, NbtTagValue>>;
  } else if (Array.isArray(root.palettes) && Array.isArray(root.palettes[0])) {
    rawPalette = root.palettes[0] as Array<Record<string, NbtTagValue>>;
  } else {
    throw new Error('В файле .nbt отсутствует палитра блоков (palette / palettes).');
  }

  if (rawPalette.length > NBT_LIMITS.maxPaletteSize) {
    throw new Error(`Палитра слишком велика (${rawPalette.length} записей).`);
  }

  const palette: NbtPaletteEntry[] = rawPalette.map((entry, idx) => {
    const propsObj: Record<string, string> = {};
    if (entry.Properties && typeof entry.Properties === 'object' && !Array.isArray(entry.Properties)) {
      for (const [k, v] of Object.entries(entry.Properties)) {
        propsObj[k] = String(v);
      }
    }
    return {
      index: idx,
      name: typeof entry.Name === 'string' ? entry.Name : 'minecraft:air',
      properties: propsObj,
      count: 0,
    };
  });

  const rawBlocks = Array.isArray(root.blocks) ? (root.blocks as Array<Record<string, NbtTagValue>>) : [];
  if (rawBlocks.length > NBT_LIMITS.maxBlocksCount) {
    throw new Error(
      `Схема содержит ${rawBlocks.length} блоков — это выше безопасного лимита просмотра (${NBT_LIMITS.maxBlocksCount}).`
    );
  }

  const blocks: NbtBlockEntry[] = [];
  let nonAirBlocks = 0;

  for (const b of rawBlocks) {
    if (!b || !Array.isArray(b.pos) || b.pos.length !== 3) continue;
    const stateIdx = typeof b.state === 'number' ? b.state : -1;
    const pal = palette[stateIdx];
    const blockId = pal ? pal.name : 'minecraft:air';
    if (pal) pal.count++;
    if (blockId === 'minecraft:air' || blockId === 'minecraft:cave_air' || blockId === 'minecraft:void_air') {
      continue;
    }
    nonAirBlocks++;
    blocks.push({
      pos: [Number(b.pos[0]), Number(b.pos[1]), Number(b.pos[2])],
      stateIndex: stateIdx,
      blockId,
      properties: pal ? pal.properties : {},
      hasBlockEntityNbt: Boolean(b.nbt),
    });
  }

  const rawEntities = Array.isArray(root.entities)
    ? (root.entities as Array<Record<string, NbtTagValue>>)
    : [];
  const hasCreateEntities = rawEntities.some((e) => {
    const nbt = e?.nbt as Record<string, NbtTagValue> | undefined;
    return typeof nbt?.id === 'string' && nbt.id.startsWith('create:');
  });

  const warnings: string[] = [];
  if (options?.isSyntheticDemo) {
    warnings.push(
      'Показана синтетическая тестовая схема, сгенерированная локально (она имитирует Create StructureTemplate .nbt и не является файлом из игры).'
    );
  }
  if (dataVersion === null) {
    warnings.push('В файле отсутствует тег DataVersion — версия Minecraft в схеме не указана.');
  }

  return {
    fileName,
    formatLabel: 'Minecraft StructureTemplate / Create .nbt',
    dataVersion,
    size,
    totalPlacedBlocks: rawBlocks.length,
    nonAirBlocks,
    entityCount: rawEntities.length,
    hasCreateEntities,
    palette: palette.filter((p) => p.count > 0 && p.name !== 'minecraft:air').sort((a, b) => b.count - a.count),
    blocks,
    warnings,
    isSyntheticDemo: options?.isSyntheticDemo,
  };
}

/**
 * Minimal binary NBT writer + synthetic Create-style `.nbt` generator for unit tests and local demo.
 */
class NbtWriter {
  private buf: number[] = [];
  private encoder = new TextEncoder();

  writeByte(v: number) {
    this.buf.push(v & 0xff);
  }
  writeShort(v: number) {
    this.buf.push((v >> 8) & 0xff, v & 0xff);
  }
  writeInt(v: number) {
    this.buf.push((v >>> 24) & 0xff, (v >>> 16) & 0xff, (v >>> 8) & 0xff, v & 0xff);
  }
  writeString(s: string) {
    const bytes = this.encoder.encode(s);
    this.writeShort(bytes.length);
    for (const b of bytes) this.buf.push(b);
  }
  writeNamedTagHeader(type: number, name: string) {
    this.writeByte(type);
    this.writeString(name);
  }
  toUint8Array(): Uint8Array {
    return new Uint8Array(this.buf);
  }
}

export function buildSyntheticDemoSchematicNbt(compressed = true): Uint8Array {
  const w = new NbtWriter();
  // Root Compound ""
  w.writeNamedTagHeader(TAG_COMPOUND, '');

  // DataVersion: 3955 (1.21.1)
  w.writeNamedTagHeader(TAG_INT, 'DataVersion');
  w.writeInt(3955);

  // size: [5, 3, 3]
  w.writeNamedTagHeader(TAG_LIST, 'size');
  w.writeByte(TAG_INT);
  w.writeInt(3);
  w.writeInt(5);
  w.writeInt(3);
  w.writeInt(3);

  const paletteIds = [
    'create:andesite_casing',
    'create:belt',
    'create:mechanical_press',
    'create:shaft',
    'create:cogwheel',
    'create:basin',
  ];

  // palette
  w.writeNamedTagHeader(TAG_LIST, 'palette');
  w.writeByte(TAG_COMPOUND);
  w.writeInt(paletteIds.length);
  for (const name of paletteIds) {
    w.writeNamedTagHeader(TAG_STRING, 'Name');
    w.writeString(name);
    w.writeByte(TAG_END);
  }

  const placed: Array<[number, number, number, number]> = [
    [0, 0, 1, 0],
    [1, 0, 1, 1],
    [2, 0, 1, 1],
    [3, 0, 1, 1],
    [4, 0, 1, 0],
    [2, 1, 1, 5],
    [2, 2, 1, 2],
    [0, 1, 1, 3],
    [0, 1, 0, 4],
    [1, 1, 0, 4],
  ];

  // blocks
  w.writeNamedTagHeader(TAG_LIST, 'blocks');
  w.writeByte(TAG_COMPOUND);
  w.writeInt(placed.length);
  for (const [x, y, z, st] of placed) {
    w.writeNamedTagHeader(TAG_LIST, 'pos');
    w.writeByte(TAG_INT);
    w.writeInt(3);
    w.writeInt(x);
    w.writeInt(y);
    w.writeInt(z);
    w.writeNamedTagHeader(TAG_INT, 'state');
    w.writeInt(st);
    w.writeByte(TAG_END);
  }

  // entities (empty)
  w.writeNamedTagHeader(TAG_LIST, 'entities');
  w.writeByte(TAG_COMPOUND);
  w.writeInt(0);

  // end root
  w.writeByte(TAG_END);
  const raw = w.toUint8Array();
  return compressed ? gzipSync(raw) : raw;
}

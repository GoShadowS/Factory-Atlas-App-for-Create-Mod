use flate2::read::GzDecoder;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::io::Read;

const MAX_DECOMPRESSED_NBT_BYTES: usize = 64 * 1024 * 1024;
const MAX_SCHEMATIC_BLOCKS: usize = 250_000;
const MAX_NBT_DEPTH: usize = 64;

#[derive(Debug, Clone)]
pub enum NbtTag {
    End,
    Byte(i8),
    Short(i16),
    Int(i32),
    Long(i64),
    Float(f32),
    Double(f64),
    ByteArray(Vec<i8>),
    String(String),
    List(u8, Vec<NbtTag>),
    Compound(BTreeMap<String, NbtTag>),
    IntArray(Vec<i32>),
    LongArray(Vec<i64>),
}

impl NbtTag {
    pub fn as_compound(&self) -> Option<&BTreeMap<String, NbtTag>> {
        match self {
            Self::Compound(m) => Some(m),
            _ => None,
        }
    }

    pub fn as_list(&self) -> Option<&[NbtTag]> {
        match self {
            Self::List(_, v) => Some(v),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::String(s) => Some(s.as_str()),
            _ => None,
        }
    }

    pub fn as_i32(&self) -> Option<i32> {
        match self {
            Self::Byte(v) => Some(i32::from(*v)),
            Self::Short(v) => Some(i32::from(*v)),
            Self::Int(v) => Some(*v),
            Self::Long(v) => i32::try_from(*v).ok(),
            _ => None,
        }
    }

    pub fn as_i32_vec(&self) -> Option<Vec<i32>> {
        match self {
            Self::IntArray(v) => Some(v.clone()),
            Self::List(_, items) => items.iter().map(|i| i.as_i32()).collect(),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchematicPaletteEntry {
    pub block_id: String,
    pub properties: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchematicBlockPlacement {
    pub x: i32,
    pub y: i32,
    pub z: i32,
    pub state_index: usize,
    pub block_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchematicBlockCount {
    pub block_id: String,
    pub count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedNbtSchematic {
    pub format: String,
    pub size: [i32; 3],
    pub palette: Vec<SchematicPaletteEntry>,
    pub blocks: Vec<SchematicBlockPlacement>,
    pub block_counts: Vec<SchematicBlockCount>,
    pub truncated: bool,
}

struct Cursor<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> Cursor<'a> {
    fn new(data: &'a [u8]) -> Self {
        Self { data, pos: 0 }
    }

    fn read_exact(&mut self, len: usize) -> Result<&'a [u8], String> {
        if self.pos.saturating_add(len) > self.data.len() {
            return Err("Unexpected EOF while parsing NBT".to_owned());
        }
        let slice = &self.data[self.pos..self.pos + len];
        self.pos += len;
        Ok(slice)
    }

    fn read_u8(&mut self) -> Result<u8, String> {
        Ok(self.read_exact(1)?[0])
    }

    fn read_i8(&mut self) -> Result<i8, String> {
        Ok(self.read_u8()? as i8)
    }

    fn read_i16(&mut self) -> Result<i16, String> {
        let b = self.read_exact(2)?;
        Ok(i16::from_be_bytes([b[0], b[1]]))
    }

    fn read_u16(&mut self) -> Result<u16, String> {
        let b = self.read_exact(2)?;
        Ok(u16::from_be_bytes([b[0], b[1]]))
    }

    fn read_i32(&mut self) -> Result<i32, String> {
        let b = self.read_exact(4)?;
        Ok(i32::from_be_bytes([b[0], b[1], b[2], b[3]]))
    }

    fn read_i64(&mut self) -> Result<i64, String> {
        let b = self.read_exact(8)?;
        Ok(i64::from_be_bytes([
            b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7],
        ]))
    }

    fn read_f32(&mut self) -> Result<f32, String> {
        let b = self.read_exact(4)?;
        Ok(f32::from_be_bytes([b[0], b[1], b[2], b[3]]))
    }

    fn read_f64(&mut self) -> Result<f64, String> {
        let b = self.read_exact(8)?;
        Ok(f64::from_be_bytes([
            b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7],
        ]))
    }

    fn read_string(&mut self) -> Result<String, String> {
        let len = usize::from(self.read_u16()?);
        let bytes = self.read_exact(len)?;
        Ok(String::from_utf8_lossy(bytes).into_owned())
    }

    fn read_payload(&mut self, tag_id: u8, depth: usize) -> Result<NbtTag, String> {
        if depth > MAX_NBT_DEPTH {
            return Err("NBT nesting exceeds maximum safe depth".to_owned());
        }
        match tag_id {
            0 => Ok(NbtTag::End),
            1 => Ok(NbtTag::Byte(self.read_i8()?)),
            2 => Ok(NbtTag::Short(self.read_i16()?)),
            3 => Ok(NbtTag::Int(self.read_i32()?)),
            4 => Ok(NbtTag::Long(self.read_i64()?)),
            5 => Ok(NbtTag::Float(self.read_f32()?)),
            6 => Ok(NbtTag::Double(self.read_f64()?)),
            7 => {
                let len = self.read_i32()?;
                if len < 0 {
                    return Err("Negative NBT ByteArray length".to_owned());
                }
                let bytes = self.read_exact(len as usize)?;
                Ok(NbtTag::ByteArray(bytes.iter().map(|&b| b as i8).collect()))
            }
            8 => Ok(NbtTag::String(self.read_string()?)),
            9 => {
                let elem_type = self.read_u8()?;
                let len = self.read_i32()?;
                if len < 0 || len as usize > MAX_SCHEMATIC_BLOCKS * 4 {
                    return Err("Invalid NBT List length".to_owned());
                }
                let mut items = Vec::with_capacity((len as usize).min(1024));
                for _ in 0..len {
                    items.push(self.read_payload(elem_type, depth + 1)?);
                }
                Ok(NbtTag::List(elem_type, items))
            }
            10 => {
                let mut map = BTreeMap::new();
                loop {
                    let sub_id = self.read_u8()?;
                    if sub_id == 0 {
                        break;
                    }
                    let name = self.read_string()?;
                    let val = self.read_payload(sub_id, depth + 1)?;
                    map.insert(name, val);
                }
                Ok(NbtTag::Compound(map))
            }
            11 => {
                let len = self.read_i32()?;
                if len < 0 || len as usize > MAX_SCHEMATIC_BLOCKS * 2 {
                    return Err("Invalid NBT IntArray length".to_owned());
                }
                let mut arr = Vec::with_capacity((len as usize).min(1024));
                for _ in 0..len {
                    arr.push(self.read_i32()?);
                }
                Ok(NbtTag::IntArray(arr))
            }
            12 => {
                let len = self.read_i32()?;
                if len < 0 || len as usize > MAX_SCHEMATIC_BLOCKS * 2 {
                    return Err("Invalid NBT LongArray length".to_owned());
                }
                let mut arr = Vec::with_capacity((len as usize).min(1024));
                for _ in 0..len {
                    arr.push(self.read_i64()?);
                }
                Ok(NbtTag::LongArray(arr))
            }
            other => Err(format!("Unknown NBT tag ID {other}")),
        }
    }
}

pub fn maybe_gunzip_nbt(raw: &[u8]) -> Result<Vec<u8>, String> {
    if raw.len() >= 2 && raw[0] == 0x1f && raw[1] == 0x8b {
        let gz = GzDecoder::new(raw);
        let mut limited = gz.take((MAX_DECOMPRESSED_NBT_BYTES + 1) as u64);
        let mut out = Vec::new();
        limited
            .read_to_end(&mut out)
            .map_err(|e| format!("Failed to gunzip NBT: {e}"))?;
        if out.len() > MAX_DECOMPRESSED_NBT_BYTES {
            return Err("Decompressed NBT exceeds 64 MiB safety limit".to_owned());
        }
        Ok(out)
    } else if raw.len() > MAX_DECOMPRESSED_NBT_BYTES {
        Err("Raw NBT exceeds 64 MiB safety limit".to_owned())
    } else {
        Ok(raw.to_vec())
    }
}

pub fn parse_nbt_root(raw: &[u8]) -> Result<(String, NbtTag), String> {
    let bytes = maybe_gunzip_nbt(raw)?;
    let mut cur = Cursor::new(&bytes);
    let root_id = cur.read_u8()?;
    if root_id != 10 {
        return Err(format!("NBT root tag must be Compound (10), got {root_id}"));
    }
    let root_name = cur.read_string()?;
    let payload = cur.read_payload(10, 0)?;
    Ok((root_name, payload))
}

pub fn parse_nbt_schematic_bytes(raw: &[u8]) -> Result<ParsedNbtSchematic, String> {
    let (_name, root_tag) = parse_nbt_root(raw)?;
    let root = root_tag
        .as_compound()
        .ok_or_else(|| "NBT root is not a compound".to_owned())?;

    // Minecraft vanilla / Create structure `.nbt`: `size`, `palette` (or `Palette`), `blocks`
    let size_arr = root
        .get("size")
        .and_then(|s| s.as_i32_vec())
        .unwrap_or_else(|| vec![0, 0, 0]);
    let mut size = [
        *size_arr.first().unwrap_or(&0),
        *size_arr.get(1).unwrap_or(&0),
        *size_arr.get(2).unwrap_or(&0),
    ];

    let raw_palette = root
        .get("palette")
        .or_else(|| root.get("Palette"))
        .and_then(|p| p.as_list())
        .unwrap_or(&[]);

    let mut palette: Vec<SchematicPaletteEntry> = Vec::with_capacity(raw_palette.len());
    for p_item in raw_palette {
        let Some(pmap) = p_item.as_compound() else {
            continue;
        };
        let block_id = pmap
            .get("Name")
            .and_then(|v| v.as_str())
            .unwrap_or("minecraft:air")
            .to_owned();
        let mut properties = BTreeMap::new();
        if let Some(props) = pmap.get("Properties").and_then(|v| v.as_compound()) {
            for (k, v) in props {
                if let Some(vs) = v.as_str() {
                    properties.insert(k.clone(), vs.to_owned());
                } else if let Some(vi) = v.as_i32() {
                    properties.insert(k.clone(), vi.to_string());
                }
            }
        }
        palette.push(SchematicPaletteEntry {
            block_id,
            properties,
        });
    }

    let raw_blocks = root
        .get("blocks")
        .and_then(|b| b.as_list())
        .ok_or_else(|| {
            "NBT file does not contain a 'blocks' list (unsupported schematic flavor)".to_owned()
        })?;

    let mut blocks: Vec<SchematicBlockPlacement> = Vec::new();
    let mut counts: HashMap<String, usize> = HashMap::new();
    let mut max_xyz = [0i32; 3];
    let mut truncated = false;

    for b_item in raw_blocks {
        if blocks.len() >= MAX_SCHEMATIC_BLOCKS {
            truncated = true;
            break;
        }
        let Some(bmap) = b_item.as_compound() else {
            continue;
        };
        let state_idx = bmap
            .get("state")
            .and_then(|v| v.as_i32())
            .unwrap_or(0)
            .max(0) as usize;
        let block_id = palette
            .get(state_idx)
            .map(|p| p.block_id.clone())
            .unwrap_or_else(|| "minecraft:air".to_owned());
        if block_id == "minecraft:air"
            || block_id == "minecraft:void_air"
            || block_id == "minecraft:cave_air"
        {
            continue;
        }
        let pos = bmap
            .get("pos")
            .and_then(|p| p.as_i32_vec())
            .unwrap_or_else(|| vec![0, 0, 0]);
        let x = *pos.first().unwrap_or(&0);
        let y = *pos.get(1).unwrap_or(&0);
        let z = *pos.get(2).unwrap_or(&0);
        max_xyz[0] = max_xyz[0].max(x + 1);
        max_xyz[1] = max_xyz[1].max(y + 1);
        max_xyz[2] = max_xyz[2].max(z + 1);
        *counts.entry(block_id.clone()).or_insert(0) += 1;
        blocks.push(SchematicBlockPlacement {
            x,
            y,
            z,
            state_index: state_idx,
            block_id,
        });
    }

    if size == [0, 0, 0] {
        size = max_xyz;
    }

    let mut block_counts: Vec<SchematicBlockCount> = counts
        .into_iter()
        .map(|(block_id, count)| SchematicBlockCount { block_id, count })
        .collect();
    block_counts.sort_by(|a, b| b.count.cmp(&a.count).then_with(|| a.block_id.cmp(&b.block_id)));

    Ok(ParsedNbtSchematic {
        format: if root.contains_key("DataVersion") {
            "Minecraft / Create Structure NBT".to_owned()
        } else {
            "Schematic NBT".to_owned()
        },
        size,
        palette,
        blocks,
        block_counts,
        truncated,
    })
}

/// Helper for unit tests and compact fixtures: encodes a minimal Compound NBT buffer.
pub fn encode_test_nbt_compound(name: &str, entries: &[(&str, NbtTag)]) -> Vec<u8> {
    fn write_string(buf: &mut Vec<u8>, s: &str) {
        let bytes = s.as_bytes();
        buf.extend_from_slice(&(bytes.len() as u16).to_be_bytes());
        buf.extend_from_slice(bytes);
    }

    fn write_tag_payload(buf: &mut Vec<u8>, tag: &NbtTag) {
        match tag {
            NbtTag::End => {}
            NbtTag::Byte(v) => buf.push(*v as u8),
            NbtTag::Short(v) => buf.extend_from_slice(&v.to_be_bytes()),
            NbtTag::Int(v) => buf.extend_from_slice(&v.to_be_bytes()),
            NbtTag::Long(v) => buf.extend_from_slice(&v.to_be_bytes()),
            NbtTag::Float(v) => buf.extend_from_slice(&v.to_be_bytes()),
            NbtTag::Double(v) => buf.extend_from_slice(&v.to_be_bytes()),
            NbtTag::ByteArray(arr) => {
                buf.extend_from_slice(&(arr.len() as i32).to_be_bytes());
                for b in arr {
                    buf.push(*b as u8);
                }
            }
            NbtTag::String(s) => write_string(buf, s),
            NbtTag::List(elem_id, items) => {
                buf.push(*elem_id);
                buf.extend_from_slice(&(items.len() as i32).to_be_bytes());
                for item in items {
                    write_tag_payload(buf, item);
                }
            }
            NbtTag::Compound(map) => {
                for (k, v) in map {
                    buf.push(tag_id_of(v));
                    write_string(buf, k);
                    write_tag_payload(buf, v);
                }
                buf.push(0);
            }
            NbtTag::IntArray(arr) => {
                buf.extend_from_slice(&(arr.len() as i32).to_be_bytes());
                for v in arr {
                    buf.extend_from_slice(&v.to_be_bytes());
                }
            }
            NbtTag::LongArray(arr) => {
                buf.extend_from_slice(&(arr.len() as i32).to_be_bytes());
                for v in arr {
                    buf.extend_from_slice(&v.to_be_bytes());
                }
            }
        }
    }

    fn tag_id_of(tag: &NbtTag) -> u8 {
        match tag {
            NbtTag::End => 0,
            NbtTag::Byte(_) => 1,
            NbtTag::Short(_) => 2,
            NbtTag::Int(_) => 3,
            NbtTag::Long(_) => 4,
            NbtTag::Float(_) => 5,
            NbtTag::Double(_) => 6,
            NbtTag::ByteArray(_) => 7,
            NbtTag::String(_) => 8,
            NbtTag::List(_, _) => 9,
            NbtTag::Compound(_) => 10,
            NbtTag::IntArray(_) => 11,
            NbtTag::LongArray(_) => 12,
        }
    }

    let mut out = Vec::new();
    out.push(10);
    write_string(&mut out, name);
    for (k, v) in entries {
        out.push(tag_id_of(v));
        write_string(&mut out, k);
        write_tag_payload(&mut out, v);
    }
    out.push(0);
    out
}

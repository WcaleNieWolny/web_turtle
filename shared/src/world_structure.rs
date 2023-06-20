use std::{collections::{HashMap}, hash::{Hasher, Hash, BuildHasher}, error::Error};

use bytes::{Bytes, BytesMut, BufMut, Buf};
use bytestring::ByteString;
use ndcopy::copy3;
use ndshape::{ConstShape3u32, ConstShape};

pub type ChunkShape = ConstShape3u32<18, 18, 18>;
pub type RealChunkShape = ConstShape3u32<16, 16, 16>;

#[derive(Debug, Eq, PartialEq, Clone, Copy)]
#[repr(transparent)]
pub struct TurtleVoxel {
    pub id: u16
}

#[derive(Clone, Eq, PartialEq, Debug, Hash)]
pub struct ChunkLocation {
    pub x: i32,
    pub y: i8,
    pub z: i32,

}

#[derive(Eq, PartialEq, Debug)]
pub struct TurtleWorldData {
    chunks: HashMap<ChunkLocation, TurtleChunk>, 
}

#[derive(Eq, PartialEq, Debug)]
pub struct TurtleWorldPalette {
    palette: Vec<ByteString>,
    palette_hashmap: HashMap<String, usize>, //Used to convert name of block into pallete index,
}

#[derive(Eq, PartialEq, Debug)]
pub struct TurtleWorld {
    pub pallete: TurtleWorldPalette,
    pub data: TurtleWorldData
}

#[derive(Eq, PartialEq, Debug)]
pub struct TurtleChunk {
    location: ChunkLocation,
    data: [TurtleVoxel; ChunkShape::SIZE as usize]
}

impl ChunkLocation {
    pub fn xyz(x: i32, y: i8, z: i32) -> Self {
        ChunkLocation {
            x,
            y,
            z
        }
    }

    ///Padding is left to individual functions
    #[inline(always)]
    fn global_xyz_to_local(&self, x: i32, y: i32, z: i32) -> Result<(u32, u32, u32), Box<dyn Error + Send + Sync>> {
        let (chunk_top_x, chunk_top_z) = (self.x << 4, self.z << 4);

        //These are local chunk XYZ
        let (x, y, z): (u32, u32, u32) = ((x - chunk_top_x).abs().try_into()?, (y - (TryInto::<i32>::try_into(self.y)? << 4)).try_into()?, (z - chunk_top_z).abs().try_into()?);

        Ok((x, y, z))
    }
}

impl TurtleVoxel {
    pub fn air() -> Self {
        Self {
            id: 0
        }
    }

    pub fn id(id: u16) -> Self {
        Self {
            id
        }
    }
}

impl Default for TurtleWorldPalette {
    fn default() -> Self {
        let mut palette_hashmap = HashMap::new();
        palette_hashmap.insert("minecraft:air".to_owned(), 0);
        Self { 
            palette_hashmap,
            palette: vec![ByteString::from_static("minecraft:air")]
        }
    }
}

impl TurtleChunk {
    fn new_by_xyz(loc: ChunkLocation) -> Self {
        Self {
            location: loc,
            data: [TurtleVoxel::air(); ChunkShape::SIZE as usize]
        }
    }

    pub fn get_mut_block_by_local_xyz(&mut self, x: u32, y: u32, z: u32) -> Option<&mut TurtleVoxel> {
        return self.data.get_mut(ChunkShape::linearize([x + 1, y + 1, z + 1]) as usize);
    }

    pub fn remove_by_global_xyz(&mut self, x: i32, y: i32, z: i32) -> Result<(), Box<dyn Error + Send + Sync>> {
        return self.update_voxel_by_global_xyz(x, y, z, |voxel| {
            if voxel.id == 0 {
                return Err("Given block is already air".into());
            };
            voxel.id = 0;
            Ok(())
        });
    }

    pub fn update_voxel_by_global_xyz<F>(&mut self, x: i32, y: i32, z: i32, mut func: F) -> Result<(), Box<dyn Error + Send + Sync>> 
        where F: FnMut(&mut TurtleVoxel) -> Result<(), Box<dyn Error + Send + Sync>>{

        println!("Working on {:?}", self.location);
        let (x, y, z) = self.location.global_xyz_to_local(x, y, z)?;
        let data = self.data.get_mut(ChunkShape::linearize([(x + 1).try_into()?, (y + 1).try_into()?, (z + 1).try_into()?]) as usize).ok_or::<String>("Something went really wrong, linearize is out of bounds".into())?;

        func(data)
    }

    pub fn voxels(&self) -> &[TurtleVoxel; ChunkShape::SIZE as usize] {
        &self.data
    }
}

impl TurtleWorld {
    pub fn new() -> Self {
        Self {
            pallete: TurtleWorldPalette::default(),
            data: TurtleWorldData { 
                chunks: HashMap::new()
            },
        }
    }

    pub fn to_bytes(&self) -> Result<Bytes, Box<dyn Error + Send + Sync>> {
        let mut bytes = BytesMut::new();

        macro_rules! write_usize {
            ($data:expr) => {
                bytes.reserve(8);
                bytes.put_u64_le($data.try_into()?);
            };
        }

        macro_rules! write_slice {
            ($data:expr) => {
                {
                    bytes.reserve(8 + $data.len());
                    bytes.put_u64_le($data.len().try_into()?);
                    bytes.put_slice($data);
                }

            };
        }

        write_usize!(self.pallete.palette.len());
        for block_name in &self.pallete.palette {
            write_slice!(str::as_bytes(&block_name))
        }
        write_usize!(self.data.chunks.len());

        for (_, chunk) in &self.data.chunks {
            bytes.reserve(9);
            bytes.put_i32_le(chunk.location.x);
            bytes.put_i8(chunk.location.y);
            bytes.put_i32_le(chunk.location.z);

            let mut real_data = [TurtleVoxel::air(); RealChunkShape::SIZE as usize];
            copy3([16; 3], &chunk.data, &ChunkShape {}, [1; 3], &mut real_data, &RealChunkShape {}, [0; 3]);
            let real_data: Vec<u8> = real_data
                .iter()
                .flat_map(|x| x.id.to_le_bytes())
                .collect();

            write_slice!(&real_data);
        }

        Ok(bytes.freeze())
    }

    pub fn from_bytes(mut bytes: Bytes) -> Result<Self, Box<dyn Error + Send + Sync>>{

        macro_rules! safe_assert {
            ($cond:expr) => {
                if !$cond {
                    return Err(format!("Safe assert failed ({}:{})) Cond: {}", std::file!(), std::line!(), stringify!($cond)).into())
                }
            };
        }

        macro_rules! assert_len {
            ($len:expr) => {
                safe_assert!(bytes.remaining() >= $len);
            };
        }
       
        assert_len!(8);
        let pallete_len = bytes.get_u64_le().try_into()?;
        let pallete: Result<Vec<ByteString>, String> = (0..pallete_len)
            .into_iter()
            .map(|_| {
                assert_len!(8);
                let len = bytes.get_u64_le() as usize;
                let bytes_read = bytes.len() - bytes.remaining();
                assert_len!(len);
                let byte_string = unsafe {
                    let to_ret = ByteString::from_bytes_unchecked(bytes.slice(bytes_read..(bytes_read + len)));
                    bytes.advance(len);
                    Ok(to_ret)
                };

                byte_string
            })
            .collect();
        let palette = pallete?;

        let palette_hashmap = palette
            .iter()
            .enumerate()
            .map(|(id, string)| {
                (str::to_owned(string), id)
            })
            .collect();

        assert_len!(8);
        let chunks_len = bytes.get_i64_le().try_into()?;
        let chunks: Result<HashMap<ChunkLocation, TurtleChunk>, String> = (0..chunks_len)
            .into_iter()
            .map(|_| {
                assert_len!(9);
                let x = bytes.get_i32_le();
                let y = bytes.get_i8();
                let z = bytes.get_i32_le();

                let data_len = bytes.get_u64_le() as usize;
                let bytes_read = bytes.len() - bytes.remaining();
                assert!(bytes_read + data_len <= bytes.len());
                let data = bytes.slice(bytes_read..(bytes_read + data_len));
                bytes.advance(data_len);

                let location = ChunkLocation {
                    x,
                    y,
                    z
                };

                let data_read: Vec<TurtleVoxel> = data
                    .chunks(2)
                    .map(|data| u16::from_le_bytes([data[0], data[1]]))
                    .map(|data| TurtleVoxel::id(data))
                    .collect();

                let mut final_data = [TurtleVoxel::air(); ChunkShape::SIZE as usize];

                copy3([16; 3], &data_read, &RealChunkShape {}, [0; 3], &mut final_data, &ChunkShape {}, [1; 3]);

                Ok((location.clone(), TurtleChunk {
                    location,
                    data: final_data
                }))
            })
            .collect();

        let chunks = chunks?;

        Ok(Self {
            data: TurtleWorldData {
                chunks
            },
            pallete: TurtleWorldPalette {
                palette,
                palette_hashmap
            }
        })
    }

    pub fn get_chunk_loc_from_global_xyz(x: i32, y: i32, z: i32) -> Result<(ChunkLocation, u32, u32, u32), Box<dyn Error + Send + Sync>> {
        let chunk_y: i8 = match (y >> 4).try_into().ok() {
            Some(val) => val,
            None => return Err("Cannot do chunk loc convertion".into())
        };

        let (chunk_x, chunk_z) = (x >> 4, z >> 4);
        let chunk_loc = ChunkLocation::xyz(chunk_x, chunk_y, chunk_z);
        let (x, y, z) = chunk_loc.global_xyz_to_local(x, y, z)?;
        return Ok((chunk_loc, x, y, z))
    }

    pub fn get_fields_mut(&mut self) -> (&mut TurtleWorldPalette, &mut TurtleWorldData) {
        let TurtleWorld { pallete, data } = self;
        (pallete, data)
    }
}

impl TurtleWorldData {
    pub fn get_mut_chunk_by_loc(&mut self, loc: &ChunkLocation) -> Option<&mut TurtleChunk> {
        self.chunks.get_mut(loc)
    }

    pub fn force_get_mut_chunk_by_loc(&mut self, loc: &ChunkLocation) -> &mut TurtleChunk {
        self.chunks.entry(loc.clone()).or_insert(TurtleChunk::new_by_xyz(loc.clone()))
    }

    pub fn remove_global_block_by_xyz(&mut self, x: i32, y: i32, z: i32) -> Result<(), Box<dyn Error + Send + Sync>> {
        let chunk_y: i8 = (y << 4).try_into()?;

        let (chunk_x, chunk_z) = (x << 4, z << 4);
        let chunk_loc = ChunkLocation::xyz(chunk_x, chunk_y, chunk_z);

        let chunk = self.chunks.get_mut(&chunk_loc).ok_or("Given block does not exist (chunk_err)".to_owned())?;
        return chunk.remove_by_global_xyz(x, y, z);
    }

    pub fn iter(&self) -> std::collections::hash_map::Iter<ChunkLocation, TurtleChunk> {
        self.chunks.iter()
    }
}


impl TurtleWorldPalette {
    pub fn get_pallete_from_id(&self, id: u16) -> Option<ByteString> {
        return self.palette.get(id as usize).cloned()
    }

    #[must_use]
    pub fn get_pallete_index(&mut self, item: &str) -> usize {
        match self.palette_hashmap.get(item) {
            Some(id) => *id,
            None => {
                self.palette.push(into_byte_string(item.into()));
                let id = self.palette.len() - 1;
                self.palette_hashmap.insert(item.into(), id);
                id
            },
        }
    }
}

#[inline(always)]
pub fn into_byte_string(data: String) -> ByteString {
    unsafe {
        ByteString::from_bytes_unchecked(data.into())
    }
}

#[cfg(test)]
mod tests {
    use bytestring::ByteString;

    use crate::world_structure::{TurtleWorld, TurtleChunk};
    use crate::world_structure::TurtleVoxel;
    use crate::world_structure::ChunkShape;
    use crate::world_structure::ChunkLocation;
    use ndshape::ConstShape;

    #[test]
    fn test_encoding_and_decoding() {
        let mut world = TurtleWorld::new();

        let _ = world.get_pallete_index("hello world");
        let loc = ChunkLocation::xyz(0, 0, 0);
        let mut chunk = TurtleChunk {
            location: loc.clone(),
            data: [TurtleVoxel::air(); ChunkShape::SIZE as usize],
        };

        chunk.data[ChunkShape::linearize([15u32; 3]) as usize] = TurtleVoxel::id(12);
        world.chunks.insert(loc, chunk);

        let bytes = world.to_bytes().expect("Cannot serialize!");
        let deserialized = TurtleWorld::from_bytes(bytes).expect("Cannot deserialize");

        assert!(deserialized == world);
    }
}

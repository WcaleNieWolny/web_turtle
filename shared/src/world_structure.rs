use std::{collections::{HashMap, hash_map::ValuesMut}, hash::{Hasher, Hash, BuildHasher}, error::Error, ops::Deref};

use bytes::{Bytes, BytesMut, BufMut, Buf};
use bytestring::ByteString;
use ndcopy::copy3;
use ndshape::{ConstShape3u32, ConstShape};

pub type ChunkShape = ConstShape3u32<18, 18, 18>;
pub type RealChunkShape = ConstShape3u32<16, 16, 16>;

#[derive(Debug, Eq, PartialEq, Clone, Copy)]
#[repr(transparent)]
pub struct TurtleVoxel {
    id: u16
}

#[derive(Clone, Eq, PartialEq, Debug)]
pub struct TurtleLocation {
    pub x: i32,
    pub y: i8,
    pub z: i32,

}

#[derive(Eq, PartialEq, Debug)]
pub struct TurtleWorld {
    pallete: Vec<ByteString>,
    pallete_hashmap: HashMap<String, usize>, //Used to convert name of block into pallete index
    chunks: HashMap<TurtleLocation, TurtleChunk, DumbHasherBuilder>,
}

#[derive(Eq, PartialEq, Debug)]
pub struct TurtleChunk {
    location: TurtleLocation,
    data: [TurtleVoxel; ChunkShape::SIZE as usize]
}

impl TurtleLocation {
    pub fn xyz(x: i32, y: i8, z: i32) -> Self {
        TurtleLocation {
            x,
            y,
            z
        }
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

impl TurtleChunk {
    pub fn get_global_block_xyz(&self, x: i32, y: i32, z: i32) -> TurtleVoxel {
        let (chunk_top_x, chunk_top_z) = (self.location.x << 4, self.location.z << 4);

        //These are local chunk XYZ
        let (x, y, z) = ((x - chunk_top_x).abs() as u32, (y - ((self.location.y as i32) << 4)) as u32, (z - chunk_top_z).abs() as u32);
        return self.data[ChunkShape::linearize([x + 1, y + 1, z + 1]) as usize]
    }
}

impl TurtleWorld {
    pub fn new() -> Self {
        Self {
            pallete: Vec::new(),
            pallete_hashmap: HashMap::new(),
            chunks: HashMap::with_hasher(DumbHasherBuilder),
        }
    }

    pub fn to_bytes(&self) -> Result<Bytes, Box<dyn Error>> {
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

        write_usize!(self.pallete.len());
        for block_name in &self.pallete {
            write_slice!(str::as_bytes(&block_name))
        }
        write_usize!(self.chunks.len());

        for (_, chunk) in &self.chunks {
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

    pub fn from_bytes(mut bytes: Bytes) -> Result<Self, Box<dyn Error>>{

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
        let pallete = pallete?;

        let pallete_hashmap = pallete
            .iter()
            .enumerate()
            .map(|(id, string)| {
                (str::to_owned(string), id)
            })
            .collect();

        assert_len!(8);
        let chunks_len = bytes.get_i64_le().try_into()?;
        let chunks: Result<HashMap<TurtleLocation, TurtleChunk, DumbHasherBuilder>, String> = (0..chunks_len)
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

                let location = TurtleLocation {
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
            chunks,
            pallete,
            pallete_hashmap
        })
    }

    ///Note: This will return none if the voxel is air
    pub fn get_chunk_by_block_xyz(&self, x: i32, y: i32, z: i32) -> Option<TurtleVoxel> {
        let chunk_y: i8 = match y.try_into().ok() {
            Some(val) => val,
            None => return None
        };

        let (chunk_x, chunk_y, chunk_z) = (x << 4, chunk_y << 4, z << 4);
        let chunk_loc = TurtleLocation::xyz(chunk_x, chunk_y, chunk_z);

        let chunk = match self.chunks.get(&chunk_loc).ok_or(format!("Chunk ({chunk_x} {chunk_y} {chunk_z}) Block ({x} {y} {z}) does not exist")) {
            Ok(val) => val,
            Err(_) => return None
        };

        let voxel = chunk.get_global_block_xyz(x, y, z);
        return if voxel.id == 0 {
            None
        } else {
            Some(voxel) 
        }
    }

    #[must_use]
    fn get_pallete_index(&mut self, item: &str) -> usize {
        match self.pallete_hashmap.get(item) {
            Some(id) => *id,
            None => {
                self.pallete.push(into_byte_string(item.into()));
                let id = self.pallete.len() - 1;
                self.pallete_hashmap.insert(item.into(), id);
                id
            },
        }
    }
}

struct DumbHasher {
    hash: u64
}

#[derive(Default)]
struct DumbHasherBuilder;

impl BuildHasher for DumbHasherBuilder {
    type Hasher = DumbHasher;

    fn build_hasher(&self) -> Self::Hasher {
        DumbHasher {
            hash: 0
        }
    }
}

impl Hasher for DumbHasher {
    fn finish(&self) -> u64 {
        self.hash 
    }

    fn write(&mut self, bytes: &[u8]) {
        assert!(bytes.len() == 8);
        let bytes: [u8; 8] = bytes.try_into().unwrap();
        self.hash = u64::from_le_bytes(bytes)
    }
}

impl Hash for TurtleLocation {
    fn hash<H: Hasher>(&self, state: &mut H) {
        let x = self.x.to_le_bytes();
        let z = self.z.to_le_bytes();
        let hash = [x[0], x[1], x[2], self.y as u8, 0, z[0], z[1], z[2]];

        hash.hash(state);
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
    use crate::world_structure::TurtleLocation;
    use ndshape::ConstShape;

    #[test]
    fn test_encoding_and_decoding() {
        let mut world = TurtleWorld::new();

        let _ = world.get_pallete_index("hello world");
        let loc = TurtleLocation::xyz(0, 0, 0);
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

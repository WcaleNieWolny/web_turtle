use std::{collections::HashMap, hash::{Hasher, Hash, BuildHasher}, error::Error, ops::Deref};

use bytes::{Bytes, BytesMut, BufMut, Buf};
use bytestring::ByteString;

#[derive(Clone, Eq, PartialEq)]
pub struct TurtleLocation {
    pub x: i32,
    pub y: i8,
    pub z: i32,

}

pub struct TurtleWorld {
    pallete: Vec<ByteString>,
    pallete_hashmap: HashMap<String, usize>, //Used to convert name of block into pallete index
    chunks: HashMap<TurtleLocation, TurtleChunk, DumbHasherBuilder>,
}

pub struct TurtleChunk {
    location: TurtleLocation,
    data: Bytes
}

impl TurtleWorld {
    pub fn new() -> Self {
        Self {
            pallete: Vec::new(),
            pallete_hashmap: HashMap::new(),
            chunks: HashMap::with_hasher(DumbHasherBuilder),
        }
    }

    pub fn to_bytes(self) -> Result<Bytes, Box<dyn Error>> {
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
            write_slice!(chunk.data.deref());
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

                Ok((location.clone(), TurtleChunk {
                    location,
                    data
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

    use crate::world_structure::TurtleWorld;

    #[test]
    fn test_encoding_and_decoding() {
        let mut world = TurtleWorld::new();
        let byte_str = ByteString::from_static("Hello world");
        world.pallete.push(byte_str);

        let bytes = world.to_bytes().expect("Cannot serialize!");

        let deserialized = TurtleWorld::from_bytes(bytes).expect("Cannot deserialize");
    }
}

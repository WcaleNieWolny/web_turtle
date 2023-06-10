use std::collections::HashMap;

use bytes::Bytes;
use bytestring::ByteString;

pub struct TurtleLocation {
    pub x: i32,
    pub y: i8,
    pub z: i32,

}

pub struct TurtleWorld {
    pub pallete: Vec<ByteString>,
    pub chunks: HashMap<TurtleLocation, TurtleChunk> 
}

pub struct TurtleChunk {
    location: TurtleLocation,
    data: Bytes
}

#[inline(always)]
pub fn into_byte_string(data: String) -> ByteString {
    unsafe {
        ByteString::from_bytes_unchecked(data.into())
    }
}

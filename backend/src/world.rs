/// Converts material string into hex string of color
pub fn block_color(material: &str) -> String {
    let hash = seahash::hash(material.as_bytes());
    let hash: [u8; 8] = hash.to_le_bytes();
    
    return format!("#{:02x}{:02x}{:02x}", hash[0], hash[4], hash[7]);
}

pub fn block_to_rgb(material: &str) -> (u8, u8, u8) {
    let hash = seahash::hash(material.as_bytes());
    let hash: [u8; 8] = hash.to_le_bytes();

    return (hash[0], hash[4], hash[7])
}

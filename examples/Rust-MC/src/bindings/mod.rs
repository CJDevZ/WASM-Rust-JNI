pub mod events;

fn get_int<const N: usize, T>(message: &[u8], offset: usize, from_bytes: fn([u8; N]) -> T) -> T {
    let bytes: [u8; N] = message[offset..offset + N]
        .try_into()
        .expect("slice with incorrect length");
    from_bytes(bytes)
}

pub fn get_u8(message: &[u8], offset: usize) -> u8 {
    get_int(message, offset, u8::from_le_bytes)
}

pub fn get_u16(message: &[u8], offset: usize) -> u16 {
    get_int(message, offset, u16::from_le_bytes)
}

pub fn get_i32(message: &[u8], offset: usize) -> i32 {
    get_int(message, offset, i32::from_le_bytes)
}

pub fn get_u32(message: &[u8], offset: usize) -> u32 {
    get_int(message, offset, u32::from_le_bytes)
}

pub fn get_u64(message: &[u8], offset: usize) -> u64 {
    get_int(message, offset, u64::from_le_bytes)
}

pub type Trit = i8;  
pub const RADIX: i32 = 3;
pub const BYTE_LENGTH: usize = 48;
pub const TRIT_LENGTH: usize = 243;
pub const INT_LENGTH: usize = BYTE_LENGTH / 4;
/// `3**242/2`
pub const HALF_3: [u32; 12] = [
    0xa5ce8964,
    0x9f007669,
    0x1484504f,
    0x3ade00d9,
    0x0c24486e,
    0x50979d57,
    0x79a4c702,
    0x48bbae36,
    0xa9f6808b,
    0xaa06a805,
    0xa87fabdf,
    0x5e69ebef,
];
//! Simple archive data in `.arc`, `.chr`, or `.mot` files.
//!
//! XC3: `chr/{ch,en,oj,wp}/*.{chr,mot}`
use std::io::Cursor;

use crate::{
    bc::Bc, eva::Eva, parse_count32_offset32, parse_offset32_count32, parse_ptr32,
    parse_string_ptr32,
};
use binrw::{binread, BinRead, BinReaderExt, BinResult, NullString};
use xc3_write::{round_up, Xc3Write, Xc3WriteOffsets};

#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets)]
#[br(magic(b"1RAS"))]
#[xc3(magic(b"1RAS"))]
#[xc3(align_after(2048))]
pub struct Sar1 {
    // TODO: calculate this when writing.
    pub file_size: u32,
    pub version: u32,

    #[br(parse_with = parse_count32_offset32)]
    #[xc3(count_offset(u32, u32))]
    pub entries: Vec<Entry>,

    pub unk_offset: u32, // pointer to start of data?

    pub unk4: u32,
    pub unk5: u32,

    #[br(map = |x: NullString| x.to_string(), pad_size_to = 128)]
    #[xc3(pad_size_to(128))]
    pub name: String,
}

#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets)]
pub struct Entry {
    #[br(parse_with = parse_offset32_count32)]
    #[xc3(offset_count(u32, u32), align(64))]
    pub entry_data: Vec<u8>,

    /// Hash of [name](#structfield.name) using [hash_str_crc].
    pub name_hash: u32,

    #[br(map = |x: NullString| x.to_string(), pad_size_to = 52)]
    #[xc3(pad_size_to(52))]
    pub name: String,
}

impl Entry {
    /// Attempt to read an item from the data for this entry.
    pub fn read_data<T>(&self) -> BinResult<T>
    where
        for<'a> T: BinRead<Args<'a> = ()>,
    {
        Cursor::new(&self.entry_data).read_le()
    }
}

// character collision?
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets)]
#[br(magic(b"CHCL"))]
#[xc3(magic(b"CHCL"))]
#[xc3(align_after(64))]
pub struct ChCl {
    pub version: u32,
    pub unk1: u32,

    #[br(parse_with = parse_ptr32)]
    #[xc3(offset(u32))]
    pub inner: ChClInner,

    // TODO: padding?
    pub unks: [u32; 10],
}

#[derive(Debug, BinRead, Xc3Write)]
pub struct ChClInner {
    #[br(parse_with = parse_offset32_count32)]
    #[xc3(offset_count(u32, u32))]
    pub unk1: Vec<[f32; 26]>,

    #[br(parse_with = parse_offset32_count32)]
    #[xc3(offset_count(u32, u32))]
    pub unk2: Vec<ChClUnk2>,

    #[br(parse_with = parse_offset32_count32)]
    #[xc3(offset_count(u32, u32), align(8))]
    pub unk3: Vec<u16>,

    #[br(parse_with = parse_offset32_count32)]
    #[xc3(offset_count(u32, u32), align(2))]
    pub unk4: Vec<u16>,

    #[br(parse_with = parse_offset32_count32)]
    #[xc3(offset_count(u32, u32), align(2))]
    pub unk5: Vec<u16>,

    #[br(parse_with = parse_offset32_count32)]
    #[xc3(offset_count(u32, u32), align(2))]
    pub unk6: Vec<u16>,

    #[br(parse_with = parse_offset32_count32)]
    #[xc3(offset_count(u32, u32))]
    pub unk7: Vec<ChClUnk7>,

    // TODO: padding?
    pub unks: [u32; 4],
}

#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets)]
pub struct ChClUnk2 {
    pub unk1: [[f32; 4]; 4],

    // TODO: bone names?
    #[br(parse_with = parse_string_ptr32)]
    #[xc3(offset(u32))]
    pub name: String,
}

#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets)]
pub struct ChClUnk7 {
    pub unk1: [[f32; 4]; 3],
    // TODO: Pointer to Idcm?
}

#[binread]
#[derive(Debug, Xc3Write, Xc3WriteOffsets)]
#[br(stream = r)]
#[br(magic(b"IDCM"))]
#[xc3(base_offset)]
#[xc3(magic(b"IDCM"))]
pub struct Idcm {
    // Subtract the magic size.
    #[br(temp, try_calc = r.stream_position().map(|p| p - 4))]
    base_offset: u64,

    pub version: u32,

    #[br(parse_with = parse_offset32_count32, offset = base_offset)]
    #[xc3(offset_count(u32, u32))]
    pub unk1: Vec<[u32; 19]>,

    #[br(parse_with = parse_offset32_count32, offset = base_offset)]
    #[xc3(offset_count(u32, u32))]
    pub unk2: Vec<[u32; 3]>,

    #[br(parse_with = parse_offset32_count32, offset = base_offset)]
    #[xc3(offset_count(u32, u32))]
    pub unk3: Vec<u64>,

    #[br(parse_with = parse_offset32_count32, offset = base_offset)]
    #[xc3(offset_count(u32, u32))]
    pub unk4: Vec<[u32; 17]>,

    #[br(parse_with = parse_offset32_count32, offset = base_offset)]
    #[xc3(offset_count(u32, u32))]
    pub unk5: Vec<u32>,

    #[br(parse_with = parse_offset32_count32, offset = base_offset)]
    #[xc3(offset_count(u32, u32))]
    pub unk6: Vec<u32>,

    #[br(parse_with = parse_offset32_count32, offset = base_offset)]
    #[xc3(offset_count(u32, u32))]
    pub unk7: Vec<[u32; 4]>,

    #[br(parse_with = parse_offset32_count32, offset = base_offset)]
    #[xc3(offset_count(u32, u32))]
    pub unk8: Vec<[f32; 4]>,

    #[br(parse_with = parse_offset32_count32, offset = base_offset)]
    #[xc3(offset_count(u32, u32))]
    pub unk9: Vec<u32>,

    pub unk10: u64,
    // TODO: more fields
}

// TODO: Is the padding always aligned?
// "effpnt" or "effect" "point"?
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets)]
#[br(magic(b"CSVB"))]
#[xc3(magic(b"CSVB"))]
#[xc3(align_after(64))]
pub struct Csvb {
    pub item_count: u16,
    pub unk_count: u16,
    pub unk_section_length: u32,
    pub string_section_length: u32,

    // TODO: Why do we need to divide here?
    #[br(count = unk_count / 8)]
    pub unks: Vec<u16>,

    #[br(count = item_count)]
    pub unk6: Vec<CvsbItem>,

    #[br(count = unk_section_length)]
    pub unk_section: Vec<u8>,

    #[br(count = string_section_length)]
    pub string_section: Vec<u8>,
}

#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets)]
pub struct CvsbItem {
    // TODO: Offsets relative to start of string section.
    pub name1_offset: u16,
    pub name2_offset: u16,
    pub unk3: u32,
    pub unk4: u32,
    pub unk5: u32,
}

impl<'a> Xc3WriteOffsets for ChClInnerOffsets<'a> {
    fn write_offsets<W: std::io::Write + std::io::Seek>(
        &self,
        writer: &mut W,
        base_offset: u64,
        data_ptr: &mut u64,
    ) -> BinResult<()> {
        // Different order than field order.
        self.unk1.write_full(writer, base_offset, data_ptr)?;
        let unk2 = self.unk2.write_offset(writer, base_offset, data_ptr)?;
        if !self.unk7.data.is_empty() {
            self.unk7.write_full(writer, base_offset, data_ptr)?;
        }
        self.unk3.write_full(writer, base_offset, data_ptr)?;
        if !self.unk4.data.is_empty() {
            self.unk4.write_full(writer, base_offset, data_ptr)?;
        }
        self.unk5.write_full(writer, base_offset, data_ptr)?;
        self.unk6.write_full(writer, base_offset, data_ptr)?;

        // Strings appear at the end of the file.
        *data_ptr = round_up(*data_ptr, 4);
        for item in unk2.0 {
            item.name.write_full(writer, base_offset, data_ptr)?;
        }

        Ok(())
    }
}

// TODO: Create a dedicated hash module.
// Data for `mm::mtl::HashStrCrc()` in the Xenoblade 2 binary from ghidra.
const CRC: [u32; 256] = [
    0x46d1e192, 0x66edf9aa, 0x927fc9e5, 0xa53baacc, 0x29b47658, 0x5a411a01, 0x0e66d5bd, 0x0dd5b1db,
    0xcb38340e, 0x04d4ebb6, 0x98bc4f54, 0x36f20f2c, 0x4a3047ed, 0x1ec1e0eb, 0x568c0c1f, 0x6a731432,
    0x81367fc6, 0xe3e25237, 0xe7f64884, 0x0fa59f64, 0x4f3109de, 0xf02d61f5, 0x5daec03b, 0x7f740e83,
    0x056ff2d8, 0x2026cc0a, 0x7ac2112d, 0x82c55605, 0xb0911ef2, 0xa7b88e4c, 0x89dca282, 0x4b254d27,
    0x7694a6d3, 0xd229eadd, 0x8e8f3738, 0x5bee7a55, 0x012eb6ab, 0x08dd28c8, 0xb5abc274, 0xbc7931f0,
    0xf2396ed5, 0xe4e43d97, 0x943f4b7f, 0x85d0293d, 0xaed83a88, 0xc8f932fc, 0xc5496f20, 0xe9228173,
    0x9b465b7d, 0xfda26680, 0x1ddeab35, 0x0c4f25cb, 0x86e32faf, 0xe59fa13a, 0xe192e2c4, 0xf147da1a,
    0x67620a8d, 0x5c9a24c5, 0xfe6afde2, 0xacad0250, 0xd359730b, 0xf35203b3, 0x96a4b44d, 0xfbcacea6,
    0x41a165ec, 0xd71e53ac, 0x835f39bf, 0x6b6bde7e, 0xd07085ba, 0x79064e07, 0xee5b20c3, 0x3b90bd65,
    0x5827aef4, 0x4d12d31c, 0x9143496e, 0x6c485976, 0xd9552733, 0x220f6895, 0xe69def19, 0xeb89cd70,
    0xc9bb9644, 0x93ec7e0d, 0x2ace3842, 0x2b6158da, 0x039e9178, 0xbb5367d7, 0x55682285, 0x4315d891,
    0x19fd8906, 0x7d8d4448, 0xb4168a03, 0x40b56a53, 0xaa3e69e0, 0xa25182fe, 0xad34d16c, 0x720c4171,
    0x9dc3b961, 0x321db563, 0x8b801b9e, 0xf5971893, 0x14cc1251, 0x8f4ae962, 0xf65aff1e, 0x13bd9dee,
    0x5e7c78c7, 0xddb61731, 0x73832c15, 0xefebdd5b, 0x1f959aca, 0xe801fb22, 0xa89826ce, 0x30b7165d,
    0x458a4077, 0x24fec52a, 0x849b065f, 0x3c6930cd, 0xa199a81d, 0xdb768f30, 0x2e45c64a, 0xff2f0d94,
    0x4ea97917, 0x6f572acf, 0x653a195c, 0x17a88c5a, 0x27e11fb5, 0x3f09c4c1, 0x2f87e71b, 0xea1493e4,
    0xd4b3a55e, 0xbe6090be, 0xaf6cd9d9, 0xda58ca00, 0x612b7034, 0x31711dad, 0x6d7db041, 0x8ca786b7,
    0x09e8bf7a, 0xc3c4d7ea, 0xa3cd77a8, 0x7700f608, 0xdf3de559, 0x71c9353f, 0x9fd236fb, 0x1675d43e,
    0x390d9e9a, 0x21ba4c6b, 0xbd1371e8, 0x90338440, 0xd5f163d2, 0xb140fef9, 0x52f50b57, 0x3710cf67,
    0x4c11a79c, 0xc6d6624e, 0x3dc7afa9, 0x34a69969, 0x70544a26, 0xf7d9ec98, 0x7c027496, 0x1bfb3ba3,
    0xb3b1dc8f, 0x9a241039, 0xf993f5a4, 0x15786b99, 0x26e704f7, 0x51503c04, 0x028bb3b8, 0xede5600c,
    0x9cb22b29, 0xb6ff339b, 0x7e771c43, 0xc71c05f1, 0x604ca924, 0x695eed60, 0x688ed0bc, 0x3e0b232f,
    0xf8a39c11, 0xbae6e67c, 0xb8cf75e1, 0x970321a7, 0x5328922b, 0xdef3df2e, 0x8d0443b0, 0x2885e3ae,
    0x6435eed1, 0xcc375e81, 0xa98495f6, 0xe0bff114, 0xb2da3e4f, 0xc01b5adf, 0x507e0721, 0x6267a36a,
    0x181a6df8, 0x7baff0c0, 0xfa6d6c13, 0x427250b2, 0xe2f742d6, 0xcd5cc723, 0x2d218be7, 0xb91fbbb1,
    0x9eb946d0, 0x1c180810, 0xfc81d602, 0x0b9c3f52, 0xc2ea456f, 0x1165b2c9, 0xabf4ad75, 0x0a56fc8c,
    0x12e0f818, 0xcadbcba1, 0x2586be56, 0x952c9b46, 0x07c6a43c, 0x78967df3, 0x477b2e49, 0x2c5d7b6d,
    0x8a637272, 0x59acbcb4, 0x74a0e447, 0xc1f8800f, 0x35c015dc, 0x230794c2, 0x4405f328, 0xec2adba5,
    0xd832b845, 0x6e4ed287, 0x48e9f7a2, 0xa44be89f, 0x38cbb725, 0xbf6ef4e6, 0xdc0e83fa, 0x54238d12,
    0xf4f0c1e3, 0xa60857fd, 0xc43c64b9, 0x00c851ef, 0x33d75f36, 0x5fd39866, 0xd1efa08a, 0xa0640089,
    0x877a978b, 0x99175d86, 0x57dfacbb, 0xceb02de9, 0xcf4d5c09, 0x3a8813d4, 0xb7448816, 0x63fa5568,
    0x06be014b, 0xd642fa7b, 0x10aa7c90, 0x8082c88e, 0x1afcba79, 0x7519549d, 0x490a87ff, 0x8820c3a0,
];

/// `mm::mtl::hashStrCrc` function in the Xenoblade 2 binary.
pub fn hash_str_crc(s: &str) -> u32 {
    // Adapted from code decompiled with ghidra.
    if s.is_empty() {
        0
    } else {
        let mut result = 0u32;
        for b in s.bytes().take(CRC.len() - 1) {
            result = CRC[((result ^ b as u32) & 0xFF) as usize] ^ (result >> 8);
        }
        CRC[s.len().min(CRC.len() - 1)] ^ (result >> 8)
    }
}

/// `mm::mtl::hashCrc` function in the Xenoblade 2 binary.
pub fn hash_crc(bytes: &[u8]) -> u32 {
    // Adapted from code decompiled with ghidra.
    if bytes.is_empty() {
        0
    } else {
        let mut result = bytes.len() as u32;
        for b in bytes {
            result = CRC[((result ^ *b as u32) & 0xFF) as usize] ^ (result >> 8);
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use crate::sar1::hash_str_crc;

    #[test]
    fn hash_str_crc_empty() {
        assert_eq!(0, hash_str_crc(""));
    }

    #[test]
    fn hash_str_crc_sar1_names() {
        // xeno3/chr/ch/ch01011000_battle.mot
        assert_eq!(0x41f7dce, hash_str_crc("break.anm"));
        assert_eq!(0x47a5352, hash_str_crc("smash.anm"));
        assert_eq!(0x4893855, hash_str_crc("bound.anm"));
        assert_eq!(0x36f1fdda, hash_str_crc("down_01.anm"));
        assert_eq!(0xf328480, hash_str_crc("sp_attack_01_08.eva"));
    }

    #[test]
    fn hash_str_crc_out_of_bounds() {
        // This should not panic.
        hash_str_crc(&"a".repeat(256));
        hash_str_crc(&"a".repeat(512));
    }
}

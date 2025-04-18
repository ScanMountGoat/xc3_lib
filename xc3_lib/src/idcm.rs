//! Collisions in `.idcm` files or embedded in other files.
//!
//! # File Paths
//! | Game | Versions | File Patterns |
//! | --- | --- | --- |
//! | Xenoblade 1 DE | 10003 | `map/*.wiidcm` |
//! | Xenoblade 2 | 10003 | `map/*.wiidcm` |
//! | Xenoblade 3 | 10003 | `map/*.idcm` |
use crate::{
    parse_offset32_count32, parse_offset32_inner_count32, parse_offset32_inner_count8, parse_ptr32,
    parse_string_ptr32, StringOffset32,
};
use binrw::{args, binread, BinRead};
use xc3_write::{Xc3Write, Xc3WriteOffsets};

#[binread]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, Xc3Write, PartialEq, Clone)]
#[br(stream = r)]
#[br(magic(b"IDCM"))]
#[xc3(base_offset)]
#[xc3(magic(b"IDCM"))]
pub struct Idcm {
    // Subtract the magic size.
    #[br(temp, try_calc = r.stream_position().map(|p| p - 4))]
    base_offset: u64,

    pub version: u32,

    // TODO: find a nicer way to detect wiidcm vs idcm.
    #[br(temp, restore_position)]
    offset_count_offset: [u32; 3],

    #[br(temp, restore_position, seek_before = std::io::SeekFrom::Start(base_offset + 160))]
    next_offset: u32,

    #[br(parse_with = parse_offset32_count32)]
    #[br(args { offset: base_offset, inner: estimate_mesh_size(offset_count_offset, next_offset) })]
    #[xc3(offset_count(u32, u32))]
    pub meshes: Vec<MeshVersioned>,

    /// Independent groups of faces.
    #[br(parse_with = parse_offset32_count32)]
    #[br(args { offset: base_offset, inner: base_offset })]
    #[xc3(offset_count(u32, u32))]
    pub face_groups: Vec<FaceGroup>,

    #[br(parse_with = parse_offset32_count32, offset = base_offset)]
    #[xc3(offset_count(u32, u32))]
    pub groups: Vec<Group>,

    #[br(parse_with = parse_offset32_count32, offset = base_offset)]
    #[xc3(offset_count(u32, u32))]
    pub unk4: Vec<[u32; 2]>,

    #[br(parse_with = parse_offset32_count32, offset = base_offset)]
    #[xc3(offset_count(u32, u32))]
    pub unk5: Vec<u32>,

    #[br(parse_with = parse_offset32_count32, offset = base_offset)]
    #[xc3(offset_count(u32, u32))]
    pub unk6: Vec<u32>,

    #[br(parse_with = parse_offset32_count32, offset = base_offset)]
    #[xc3(offset_count(u32, u32))]
    pub unk7: Vec<[u32; 3]>,

    #[br(parse_with = parse_offset32_count32, offset = base_offset)]
    #[xc3(offset_count(u32, u32))]
    pub vertices: Vec<[f32; 4]>,

    #[br(parse_with = parse_offset32_count32, offset = base_offset)]
    #[xc3(offset_count(u32, u32))]
    pub unk9: Vec<Unk9>,

    pub unk10: u64,

    #[br(parse_with = parse_offset32_inner_count32, offset = base_offset)]
    #[xc3(offset_inner_count(u32, self.instances.mesh_indices.len() as u32))]
    pub instances: MeshInstances,

    #[br(parse_with = parse_offset32_count32, offset = base_offset)]
    #[xc3(offset_count(u32, u32))]
    pub unk13: Vec<[f32; 8]>,

    #[br(parse_with = parse_offset32_count32)]
    #[br(args { offset: base_offset, inner: base_offset })]
    #[xc3(offset_count(u32, u32))]
    pub unk19: Vec<Unk19>,

    pub unks1_1: u32,

    #[br(parse_with = parse_offset32_count32, offset = base_offset)]
    #[xc3(offset_count(u32, u32))]
    pub unk18: Vec<[u32; 10]>,

    pub unks1_3: [u32; 2],

    /// Names for each of the [Mesh] in [meshes](#structfield.meshes).
    #[br(parse_with = parse_ptr32)]
    #[br(args { offset: base_offset, inner: args! { count: meshes.len(), inner: base_offset } })]
    #[xc3(offset(u32))]
    pub mesh_names: Vec<StringOffset32>,

    pub unk21: u32,

    #[br(parse_with = parse_ptr32, offset = base_offset)]
    #[xc3(offset(u32))]
    pub unk15: [f32; 10],

    #[br(parse_with = parse_offset32_count32, offset = base_offset)]
    #[xc3(offset_count(u32, u32))]
    pub unks1_2: Vec<u32>, // TODO: type?

    #[br(parse_with = parse_offset32_count32)]
    #[br(args{ offset: base_offset, inner: base_offset})]
    #[xc3(offset_count(u32, u32))]
    pub unk16: Vec<Unk16>,

    #[br(parse_with = parse_offset32_count32, offset = base_offset)]
    #[xc3(offset_count(u32, u32))]
    pub unk17: Vec<[u32; 4]>,

    #[br(parse_with = parse_offset32_count32, offset = base_offset)]
    #[xc3(offset_count(u32, u32))]
    pub unks1_4: Vec<u32>, // TODO: type?

    pub unks: [u32; 12], // TODO: padding?
}

// TODO: Create an entire separate type for wiidcm?
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
#[br(import_raw(size: u32))]
pub enum MeshVersioned {
    #[br(pre_assert(size == 20))]
    MeshLegacy(MeshLegacy),

    #[br(pre_assert(size == 60))]
    Mesh(Mesh),
}

/// .idcm mesh
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct Mesh {
    pub unk1: u32,
    /// Index into [face_groups](struct.Idcm.html#structfield.face_groups).
    pub face_group_start_index: u32,
    /// Index into [face_groups](struct.Idcm.html#structfield.face_groups).
    pub face_group_start_index2: u32,
    /// The number of groups in [face_groups](struct.Idcm.html#structfield.face_groups).
    pub face_group_count: u32,
    /// The number of groups in [face_groups](struct.Idcm.html#structfield.face_groups).
    pub face_group_count2: u32,
    pub unk6: u32,
    pub unk7: u32,
    pub unk8: u32,
    pub unk9: u32,
    pub unk: [u32; 6],
}

/// .wiidcm mesh
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct MeshLegacy {
    pub unk1: u32,
    /// Index into [face_groups](struct.Idcm.html#structfield.face_groups).
    pub face_group_start_index: u32,
    /// The number of groups in [face_groups](struct.Idcm.html#structfield.face_groups).
    pub face_group_count: u32,
    pub unk2: u32,
    pub unk3: u32,
}

/// A single triangle fan.
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
#[br(import_raw(base_offset: u64))]
pub struct FaceGroup {
    // TODO: Offsets into the buffer aren't in any particular order?
    /// Indices into [vertices](struct.Idcm.html#structfield.vertices).
    #[br(parse_with = parse_offset32_inner_count8, offset = base_offset)]
    #[xc3(offset_inner_count(u32, self.faces.unk1.len() as u8))]
    pub faces: Faces,

    /// Index into [groups](struct.Idcm.html#structfield.groups).
    pub group_index: u8,

    pub unk2: u16,
    pub unk3: u32,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
#[br(import_raw(count: u8))]
pub struct Faces {
    #[br(count = count)]
    pub unk1: Vec<u16>,

    #[br(count = count + 2)]
    pub vertex_indices: Vec<u16>,
    // TODO: additional index data?
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct Group {
    pub count: u32,
    pub start_index: u32,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct Unk9 {
    // TODO: half float?
    pub unk1: u16,
    pub unk2: u16,
    pub unk3: u16,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
#[br(import_raw(count: u32))]
pub struct MeshInstances {
    #[br(count = count)]
    pub transforms: Vec<InstanceTransform>,

    // (mesh_index, ???)
    #[br(count = count)]
    pub mesh_indices: Vec<(u16, u16)>,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct InstanceTransform {
    /// Row-major global transform of the instance.
    pub transform: [[f32; 4]; 4],
    pub unk2: [[f32; 4]; 4],
    pub unk3: [u32; 8],
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
#[br(import_raw(base_offset: u64))]
pub struct Unk16 {
    #[br(parse_with = parse_string_ptr32, offset = base_offset)]
    #[xc3(offset(u32))]
    pub unk1: String,
    pub unk2: u32,
    // TODO: Why does this not always work?
    // pub unk3: u32,
    // pub unk4: u32,
    // pub unk5: u32,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
#[br(import_raw(base_offset: u64))]
pub struct Unk19 {
    #[br(parse_with = parse_ptr32, offset = base_offset)]
    #[xc3(offset(u32))]
    pub unk1: u32,
    pub unk2: u32, // TODO: offset into floats?
    pub unk3: u32,
}

impl Xc3WriteOffsets for IdcmOffsets<'_> {
    type Args = ();

    fn write_offsets<W: std::io::prelude::Write + std::io::prelude::Seek>(
        &self,
        writer: &mut W,
        _base_offset: u64,
        data_ptr: &mut u64,
        endian: xc3_write::Endian,
        _args: Self::Args,
    ) -> xc3_write::Xc3Result<()> {
        let base_offset = self.base_offset;
        // Different order than field order.
        self.unk15
            .write_full(writer, base_offset, data_ptr, endian, ())?;
        self.meshes
            .write_full(writer, base_offset, data_ptr, endian, ())?;
        self.unk17
            .write_full(writer, base_offset, data_ptr, endian, ())?;
        let unk2 = self
            .face_groups
            .write(writer, base_offset, data_ptr, endian)?;
        self.groups
            .write_full(writer, base_offset, data_ptr, endian, ())?;
        self.unk4
            .write_full(writer, base_offset, data_ptr, endian, ())?;
        self.unk18
            .write_full(writer, base_offset, data_ptr, endian, ())?;
        let unk16 = self.unk16.write(writer, base_offset, data_ptr, endian)?;
        self.unk5
            .write_full(writer, base_offset, data_ptr, endian, ())?;
        self.unk6
            .write_full(writer, base_offset, data_ptr, endian, ())?;
        self.unk7
            .write_full(writer, base_offset, data_ptr, endian, ())?;
        self.vertices
            .write_full(writer, base_offset, data_ptr, endian, ())?;
        self.unk9
            .write_full(writer, base_offset, data_ptr, endian, ())?;

        // TODO: A lot of empty lists go here?
        *data_ptr += 12;

        self.instances
            .write_full(writer, base_offset, data_ptr, endian, ())?;
        self.unks1_2
            .write_full(writer, base_offset, data_ptr, endian, ())?;
        self.unks1_4
            .write_full(writer, base_offset, data_ptr, endian, ())?;

        self.unk13
            .write_full(writer, base_offset, data_ptr, endian, ())?;
        let unk19 = self.unk19.write(writer, base_offset, data_ptr, endian)?;

        for u in unk2.0 {
            u.write_offsets(writer, base_offset, data_ptr, endian, ())?;
        }

        for u in unk19.0 {
            u.write_offsets(writer, base_offset, data_ptr, endian, ())?;
        }

        self.mesh_names
            .write_full(writer, base_offset, data_ptr, endian, ())?;

        for u in unk16.0 {
            u.write_offsets(writer, base_offset, data_ptr, endian, ())?;
        }

        Ok(())
    }
}

fn estimate_mesh_size(offset_count_offset: [u32; 3], next_offset: u32) -> u32 {
    let next_offset = if next_offset == 0 {
        offset_count_offset[2]
    } else {
        next_offset
    };
    (next_offset - offset_count_offset[0]) / offset_count_offset[1]
}

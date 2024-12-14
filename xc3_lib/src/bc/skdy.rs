use std::{cell::RefCell, rc::Rc};

use crate::{parse_ptr64, parse_string_ptr64};
use binrw::{binread, BinRead};
use xc3_write::{Xc3Write, Xc3WriteOffsets};

use super::{BcList, StringSection};

// TODO: skeleton dynamics?
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
#[br(magic(b"SKDY"))]
#[xc3(magic(b"SKDY"))]
pub struct Skdy {
    #[br(parse_with = parse_ptr64)]
    #[xc3(offset(u64))]
    pub dynamics: Dynamics,
}

#[binread]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, Xc3Write, PartialEq, Clone)]
pub struct Dynamics {
    pub unk1: BcList<()>,
    pub unk2: u64,

    #[br(temp, restore_position)]
    offset: u64,

    #[br(parse_with = parse_ptr64)]
    #[xc3(offset(u64))]
    pub unk3: DynamicsUnk1,

    // TODO: not always present?
    #[br(parse_with = parse_ptr64)]
    #[br(if(offset >= 80))]
    #[xc3(offset(u64))]
    pub unk4: Option<DynamicsUnk2>,

    #[br(parse_with = parse_ptr64)]
    #[br(if(offset >= 88))]
    #[xc3(offset(u64))]
    pub unk5: Option<DynamicsUnk3>,
}

// TODO: Collisions?
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, PartialEq, Clone)]
pub struct DynamicsUnk1 {
    pub spheres: BcList<Sphere>,
    pub capsules: BcList<Capsule>,
    pub planes: BcList<Plane>,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, PartialEq, Clone)]
pub struct Sphere {
    pub unk1: u32,
    pub unk2: i32,

    // CO_SPHERE_
    #[br(parse_with = parse_string_ptr64)]
    #[xc3(offset(u64))]
    pub name: String,

    #[br(parse_with = parse_string_ptr64)]
    #[xc3(offset(u64))]
    pub bone_name: String,

    pub unk4: u32,
    pub unk5: i32,

    pub unk6: [f32; 9],
    // TODO: padding from alignment?
    pub unk7: [i32; 3],
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, PartialEq, Clone)]
pub struct Capsule {
    pub unk1: u32,
    pub unk2: i32,

    // CO_CAPSULE_
    #[br(parse_with = parse_string_ptr64)]
    #[xc3(offset(u64))]
    pub name: String,

    #[br(parse_with = parse_string_ptr64)]
    #[xc3(offset(u64))]
    pub bone_name: String,

    pub unk4: u32,
    pub unk5: i32,

    pub unk6: [f32; 10],
    // TODO: padding from alignment?
    pub unk7: [i32; 2],
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, PartialEq, Clone)]
pub struct Plane {
    pub unk1: u32,
    pub unk2: i32,

    // CO_PLANE_
    #[br(parse_with = parse_string_ptr64)]
    #[xc3(offset(u64))]
    pub name: String,

    #[br(parse_with = parse_string_ptr64)]
    #[xc3(offset(u64))]
    pub bone_name: String,

    pub unk4: u32,
    pub unk5: i32,

    pub unk6: [f32; 8],
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, PartialEq, Clone)]
pub struct DynamicsUnk2 {
    pub unk1: BcList<DynamicsUnk2Item>,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, PartialEq, Clone)]
pub struct DynamicsUnk2Item {
    // DS_
    #[br(parse_with = parse_string_ptr64)]
    #[xc3(offset(u64))]
    pub name: String,

    pub unk1: BcList<DynamicsUnk2ItemUnk1>,
    pub unk2: BcList<[f32; 4]>,
    pub sticks: BcList<Stick>,
    pub springs: BcList<Spring>,
    pub unk5: BcList<()>,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, PartialEq, Clone)]
pub struct DynamicsUnk2ItemUnk1 {
    // DN_
    #[br(parse_with = parse_string_ptr64)]
    #[xc3(offset(u64))]
    pub name1: String,

    // DJ_
    // TODO: Why does this not always parse properly?
    // TODO: is there a file revision that doesn't have this?
    #[br(parse_with = parse_string_ptr64)]
    #[xc3(offset(u64))]
    pub name2: String,

    pub unk1: [f32; 7],
    pub unk2: u32,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, PartialEq, Clone)]
pub struct Stick {
    // DC_STICK_
    #[br(parse_with = parse_string_ptr64)]
    #[xc3(offset(u64))]
    pub name: String,

    pub unk1: [f32; 7],
    pub unk2: u32,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, PartialEq, Clone)]
pub struct Spring {
    // DC_SPRING_
    #[br(parse_with = parse_string_ptr64)]
    #[xc3(offset(u64))]
    pub name: String,

    pub unk1: [f32; 5],
    // TODO: padding from alignment?
    pub unk5: i32,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, PartialEq, Clone)]
pub struct DynamicsUnk3 {
    pub unk1: BcList<DynamicsUnk3Item>,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, PartialEq, Clone)]
pub struct DynamicsUnk3Item {
    // CY_S_BG_
    #[br(parse_with = parse_string_ptr64)]
    #[xc3(offset(u64))]
    pub name: String,

    /// The name of the [DynamicsUnk2ItemUnk1].
    #[br(parse_with = parse_string_ptr64)]
    #[xc3(offset(u64))]
    pub name2: String,

    /// The name of the [DynamicsUnk2ItemUnk1].
    #[br(parse_with = parse_string_ptr64)]
    #[xc3(offset(u64))]
    pub name3: String,

    pub unk6: [f32; 6],

    pub unk4: u32,
    // TODO: padding from alignment?
    pub unk5: i32,
}

impl<'a> Xc3WriteOffsets for DynamicsOffsets<'a> {
    type Args = ();

    fn write_offsets<W: std::io::Write + std::io::Seek>(
        &self,
        writer: &mut W,
        base_offset: u64,
        data_ptr: &mut u64,
        endian: xc3_write::Endian,
        _args: Self::Args,
    ) -> xc3_write::Xc3Result<()> {
        let string_section = Rc::new(RefCell::new(StringSection::default()));

        if !self.unk1.0.data.is_empty() {
            self.unk1
                .write_offsets(writer, base_offset, data_ptr, endian, ())?;
        }
        self.unk3.write_full(
            writer,
            base_offset,
            data_ptr,
            endian,
            string_section.clone(),
        )?;
        self.unk4.write_full(
            writer,
            base_offset,
            data_ptr,
            endian,
            string_section.clone(),
        )?;
        self.unk5.write_full(
            writer,
            base_offset,
            data_ptr,
            endian,
            string_section.clone(),
        )?;

        string_section.borrow().write(writer, data_ptr, 8, endian)?;

        Ok(())
    }
}

impl<'a> Xc3WriteOffsets for DynamicsUnk1Offsets<'a> {
    type Args = Rc<RefCell<StringSection>>;

    fn write_offsets<W: std::io::Write + std::io::Seek>(
        &self,
        writer: &mut W,
        base_offset: u64,
        data_ptr: &mut u64,
        endian: xc3_write::Endian,
        args: Self::Args,
    ) -> xc3_write::Xc3Result<()> {
        self.spheres
            .write_offsets(writer, base_offset, data_ptr, endian, args.clone())?;
        self.capsules
            .write_offsets(writer, base_offset, data_ptr, endian, args.clone())?;
        self.planes
            .write_offsets(writer, base_offset, data_ptr, endian, args.clone())?;
        Ok(())
    }
}

impl<'a> Xc3WriteOffsets for SphereOffsets<'a> {
    type Args = Rc<RefCell<StringSection>>;

    fn write_offsets<W: std::io::Write + std::io::Seek>(
        &self,
        _writer: &mut W,
        _base_offset: u64,
        _data_ptr: &mut u64,
        _endian: xc3_write::Endian,
        args: Self::Args,
    ) -> xc3_write::Xc3Result<()> {
        args.borrow_mut().insert_offset(&self.name);
        args.borrow_mut().insert_offset(&self.bone_name);
        Ok(())
    }
}

impl<'a> Xc3WriteOffsets for CapsuleOffsets<'a> {
    type Args = Rc<RefCell<StringSection>>;

    fn write_offsets<W: std::io::Write + std::io::Seek>(
        &self,
        _writer: &mut W,
        _base_offset: u64,
        _data_ptr: &mut u64,
        _endian: xc3_write::Endian,
        args: Self::Args,
    ) -> xc3_write::Xc3Result<()> {
        args.borrow_mut().insert_offset(&self.name);
        args.borrow_mut().insert_offset(&self.bone_name);
        Ok(())
    }
}

impl<'a> Xc3WriteOffsets for PlaneOffsets<'a> {
    type Args = Rc<RefCell<StringSection>>;

    fn write_offsets<W: std::io::Write + std::io::Seek>(
        &self,
        _writer: &mut W,
        _base_offset: u64,
        _data_ptr: &mut u64,
        _endian: xc3_write::Endian,
        args: Self::Args,
    ) -> xc3_write::Xc3Result<()> {
        args.borrow_mut().insert_offset(&self.name);
        args.borrow_mut().insert_offset(&self.bone_name);
        Ok(())
    }
}

impl<'a> Xc3WriteOffsets for DynamicsUnk2Offsets<'a> {
    type Args = Rc<RefCell<StringSection>>;

    fn write_offsets<W: std::io::Write + std::io::Seek>(
        &self,
        writer: &mut W,
        base_offset: u64,
        data_ptr: &mut u64,
        endian: xc3_write::Endian,
        args: Self::Args,
    ) -> xc3_write::Xc3Result<()> {
        self.unk1
            .write_offsets(writer, base_offset, data_ptr, endian, args.clone())?;
        Ok(())
    }
}

impl<'a> Xc3WriteOffsets for DynamicsUnk2ItemOffsets<'a> {
    type Args = Rc<RefCell<StringSection>>;

    fn write_offsets<W: std::io::Write + std::io::Seek>(
        &self,
        writer: &mut W,
        base_offset: u64,
        data_ptr: &mut u64,
        endian: xc3_write::Endian,
        args: Self::Args,
    ) -> xc3_write::Xc3Result<()> {
        args.borrow_mut().insert_offset(&self.name);
        self.unk1
            .write_offsets(writer, base_offset, data_ptr, endian, args.clone())?;
        self.unk2
            .write_offsets(writer, base_offset, data_ptr, endian, ())?;
        self.sticks
            .write_offsets(writer, base_offset, data_ptr, endian, args.clone())?;
        self.springs
            .write_offsets(writer, base_offset, data_ptr, endian, args.clone())?;
        self.unk5
            .write_offsets(writer, base_offset, data_ptr, endian, ())?;
        Ok(())
    }
}

impl<'a> Xc3WriteOffsets for DynamicsUnk2ItemUnk1Offsets<'a> {
    type Args = Rc<RefCell<StringSection>>;

    fn write_offsets<W: std::io::Write + std::io::Seek>(
        &self,
        _writer: &mut W,
        _base_offset: u64,
        _data_ptr: &mut u64,
        _endian: xc3_write::Endian,
        args: Self::Args,
    ) -> xc3_write::Xc3Result<()> {
        args.borrow_mut().insert_offset(&self.name1);
        args.borrow_mut().insert_offset(&self.name2);
        Ok(())
    }
}

impl<'a> Xc3WriteOffsets for StickOffsets<'a> {
    type Args = Rc<RefCell<StringSection>>;

    fn write_offsets<W: std::io::Write + std::io::Seek>(
        &self,
        _writer: &mut W,
        _base_offset: u64,
        _data_ptr: &mut u64,
        _endian: xc3_write::Endian,
        args: Self::Args,
    ) -> xc3_write::Xc3Result<()> {
        args.borrow_mut().insert_offset(&self.name);
        Ok(())
    }
}

impl<'a> Xc3WriteOffsets for SpringOffsets<'a> {
    type Args = Rc<RefCell<StringSection>>;

    fn write_offsets<W: std::io::Write + std::io::Seek>(
        &self,
        _writer: &mut W,
        _base_offset: u64,
        _data_ptr: &mut u64,
        _endian: xc3_write::Endian,
        args: Self::Args,
    ) -> xc3_write::Xc3Result<()> {
        args.borrow_mut().insert_offset(&self.name);
        Ok(())
    }
}

impl<'a> Xc3WriteOffsets for DynamicsUnk3Offsets<'a> {
    type Args = Rc<RefCell<StringSection>>;

    fn write_offsets<W: std::io::Write + std::io::Seek>(
        &self,
        writer: &mut W,
        base_offset: u64,
        data_ptr: &mut u64,
        endian: xc3_write::Endian,
        args: Self::Args,
    ) -> xc3_write::Xc3Result<()> {
        self.unk1
            .write_offsets(writer, base_offset, data_ptr, endian, args)
    }
}

impl<'a> Xc3WriteOffsets for DynamicsUnk3ItemOffsets<'a> {
    type Args = Rc<RefCell<StringSection>>;

    fn write_offsets<W: std::io::Write + std::io::Seek>(
        &self,
        _writer: &mut W,
        _base_offset: u64,
        _data_ptr: &mut u64,
        _endian: xc3_write::Endian,
        args: Self::Args,
    ) -> xc3_write::Xc3Result<()> {
        args.borrow_mut().insert_offset(&self.name);
        args.borrow_mut().insert_offset(&self.name2);
        args.borrow_mut().insert_offset(&self.name3);
        Ok(())
    }
}

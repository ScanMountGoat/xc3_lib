//! Animation and skeleton data in `.anm` or `.motstm_data` files or [Sar1](crate::sar1::Sar1) archives.
use crate::{parse_offset64_count32, parse_ptr64, parse_string_ptr64};
use binrw::{args, binread, BinRead};
use xc3_write::{WriteFull, Xc3Write, Xc3WriteOffsets};

use anim::Anim;
use asmb::Asmb;
use skdy::Skdy;
use skel::Skel;

pub mod anim;
pub mod asmb;
pub mod skdy;
pub mod skel;

// TODO: is the 64 byte alignment on the sar1 entry size?
// TODO: Add class names from xenoblade 2 binary where appropriate.
// Assume the BC is at the beginning of the reader to simplify offsets.
#[binread]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
#[br(magic(b"BC\x00\x00"))]
#[br(stream = r)]
#[xc3(magic(b"BC\x00\x00"))]
#[xc3(align_after(64))]
pub struct Bc {
    pub unk1: u32,
    // TODO: not always equal to the sar1 size?
    pub data_size: u32,
    pub address_count: u32,

    #[br(parse_with = parse_ptr64)]
    #[xc3(offset(u64))]
    pub data: BcData,

    // TODO: A list of offsets to data items?
    // TODO: relocatable addresses?
    #[br(parse_with = parse_ptr64)]
    #[br(args { inner: args! { count: address_count as usize}})]
    #[xc3(offset(u64), align(8, 0xff))]
    pub addresses: Vec<u64>,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub enum BcData {
    #[br(magic(2u32))]
    #[xc3(magic(2u32))]
    Skdy(Skdy),

    #[br(magic(4u32))]
    #[xc3(magic(4u32))]
    Anim(Anim),

    #[br(magic(6u32))]
    #[xc3(magic(6u32))]
    Skel(Skel),

    #[br(magic(7u32))]
    #[xc3(magic(7u32))]
    Asmb(Asmb),
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct Transform {
    pub translation: [f32; 4],
    pub rotation_quaternion: [f32; 4],
    pub scale: [f32; 4],
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct StringOffset {
    #[br(parse_with = parse_string_ptr64)]
    #[xc3(offset(u64))]
    pub name: String,
}

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, Xc3Write, Xc3WriteOffsets, PartialEq, Clone)]
pub struct BcOffset<T>
where
    T: Xc3Write + 'static,
    for<'a> T: BinRead<Args<'a> = ()>,
    for<'a> T::Offsets<'a>: Xc3WriteOffsets<Args = ()>,
{
    #[br(parse_with = parse_ptr64)]
    #[xc3(offset(u64))]
    pub value: T,
}

#[binread]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, PartialEq, Clone)]
pub struct BcListN<T, const N: u64>
where
    T: 'static,
    for<'a> T: BinRead<Args<'a> = ()>,
{
    #[br(parse_with = parse_offset64_count32)]
    pub elements: Vec<T>,

    // TODO: Does this field do anything?
    // TODO: Don't actually store this field?
    #[br(assert(unk1 == -1))]
    pub unk1: i32,
}

pub struct BcListNOffsets<'a, T, const N: u64>(xc3_write::Offset<'a, u64, Vec<T>>);

impl<T, const N: u64> Xc3Write for BcListN<T, N>
where
    T: Xc3Write + 'static,
    for<'a> T: BinRead<Args<'a> = ()>,
{
    type Offsets<'a>
        = BcListNOffsets<'a, T, N>
    where
        T: 'a;

    fn xc3_write<W: std::io::Write + std::io::Seek>(
        &self,
        writer: &mut W,
        endian: xc3_write::Endian,
    ) -> xc3_write::Xc3Result<Self::Offsets<'_>> {
        let offset =
            xc3_write::Offset::new(writer.stream_position()?, &self.elements, Some(N), 0xff);
        0u64.xc3_write(writer, endian)?;
        (self.elements.len() as u32).xc3_write(writer, endian)?;
        (-1i32).xc3_write(writer, endian)?;

        Ok(BcListNOffsets(offset))
    }
}

impl<'a, T, const N: u64> Xc3WriteOffsets for BcListNOffsets<'a, T, N>
where
    T: Xc3Write + 'static,
    <T as Xc3Write>::Offsets<'a>: Xc3WriteOffsets,
    <<T as Xc3Write>::Offsets<'a> as Xc3WriteOffsets>::Args: Clone,
    Vec<T>: WriteFull,
{
    type Args = <Vec<T> as WriteFull>::Args;

    fn write_offsets<W: std::io::Write + std::io::Seek>(
        &self,
        writer: &mut W,
        base_offset: u64,
        data_ptr: &mut u64,
        endian: xc3_write::Endian,
        args: Self::Args,
    ) -> xc3_write::Xc3Result<()> {
        self.0
            .write_full(writer, base_offset, data_ptr, endian, args)
    }
}

pub type BcList<T> = BcListN<T, 4>;
pub type BcList2<T> = BcListN<T, 2>;
pub type BcList8<T> = BcListN<T, 8>;

// TODO: Use this for all instances of BcList?
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, PartialEq, Clone)]
pub enum BcListCount<T> {
    List(Vec<T>),
    /// An empty list that still specifies a count.
    NullOffsetCount(u32),
}

impl<T> BinRead for BcListCount<T>
where
    T: 'static,
    for<'a> T: BinRead<Args<'a> = ()>,
{
    type Args<'a> = ();

    // TODO: Derive this somehow?
    fn read_options<R: std::io::Read + std::io::Seek>(
        reader: &mut R,
        endian: binrw::Endian,
        _args: Self::Args<'_>,
    ) -> binrw::BinResult<Self> {
        let offset = u64::read_options(reader, endian, ())?;
        let count = u32::read_options(reader, endian, ())?;

        let pos = reader.stream_position()?;
        let unk = i32::read_options(reader, endian, ())?;
        if unk != -1 {
            return Err(binrw::Error::AssertFail {
                pos,
                message: format!("expected -1 but found {unk}"),
            });
        }

        if offset == 0 {
            Ok(Self::NullOffsetCount(count))
        } else {
            crate::parse_vec(reader, endian, Default::default(), offset, count as usize)
                .map(Self::List)
        }
    }
}

pub enum BcListCountOffsets<'a, T> {
    List(xc3_write::Offset<'a, u64, Vec<T>>),
    NullOffsetCount,
}

impl<T> Xc3Write for BcListCount<T>
where
    T: Xc3Write + 'static,
{
    type Offsets<'a>
        = BcListCountOffsets<'a, T>
    where
        T: 'a;

    fn xc3_write<W: std::io::Write + std::io::Seek>(
        &self,
        writer: &mut W,
        endian: xc3_write::Endian,
    ) -> xc3_write::Xc3Result<Self::Offsets<'_>> {
        let offsets = match self {
            BcListCount::List(elements) => {
                let offset =
                    xc3_write::Offset::new(writer.stream_position()?, elements, Some(8), 0xff);
                0u64.xc3_write(writer, endian)?;
                (elements.len() as u32).xc3_write(writer, endian)?;
                BcListCountOffsets::List(offset)
            }
            BcListCount::NullOffsetCount(count) => {
                0u64.xc3_write(writer, endian)?;
                count.xc3_write(writer, endian)?;
                BcListCountOffsets::NullOffsetCount
            }
        };
        (-1i32).xc3_write(writer, endian)?;

        Ok(offsets)
    }
}

impl<'a, T> Xc3WriteOffsets for BcListCountOffsets<'a, T>
where
    T: Xc3Write + 'static,
    T::Offsets<'a>: Xc3WriteOffsets<Args = ()>,
    Vec<T>: WriteFull<Args = ()>,
{
    type Args = ();

    fn write_offsets<W: std::io::Write + std::io::Seek>(
        &self,
        writer: &mut W,
        base_offset: u64,
        data_ptr: &mut u64,
        endian: xc3_write::Endian,
        _args: Self::Args,
    ) -> xc3_write::Xc3Result<()> {
        match self {
            BcListCountOffsets::List(offset) => {
                if !offset.data.is_empty() {
                    offset.write_full(writer, base_offset, data_ptr, endian, ())
                } else {
                    Ok(())
                }
            }
            BcListCountOffsets::NullOffsetCount => Ok(()),
        }
    }
}

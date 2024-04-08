//! Effects in .wiefb files.
//!
//! # File Paths
//!
//! | Game | Versions | File Patterns |
//! | --- | --- | --- |
//! | Xenoblade Chronicles 1 DE | |  |
//! | Xenoblade Chronicles 2 |  | `effect/**/*.wiefb` |
//! | Xenoblade Chronicles 3 |  |  |
use binrw::BinRead;

#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[derive(Debug, BinRead, PartialEq, Clone)]
#[br(magic(b"efb0"))]
pub struct Efb0 {
    version: (u16, u16),
    // TODO: embedded mxmd, mibl, hcps?
}

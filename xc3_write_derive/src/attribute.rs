use proc_macro2::{Ident, TokenStream};
use syn::{ext::IdentExt, parenthesized, Attribute, LitInt};

pub struct FieldOptions {
    pub field_type: Option<FieldType>,
    pub align: Option<u64>,
    pub pad_size_to: Option<u64>,
}

// TODO: Separate count field similar to #[bw(calc(...))]?
pub enum FieldType {
    Skip,
    SavePosition,
    SharedOffset,
    Offset(Ident),
    OffsetCount(Ident, Ident),
    CountOffset(Ident, Ident),
}

impl FieldOptions {
    pub fn from_attrs(attrs: &[Attribute]) -> Self {
        let mut field_type = None;
        let mut align = None;
        let mut pad_size_to = None;

        for a in attrs {
            if a.path().is_ident("xc3") {
                // TODO: add types like offset32 or offset64_count32
                // TODO: separate offset and count fields?
                let _ = a.parse_nested_meta(|meta| {
                    if meta.path.is_ident("offset") {
                        // #[xc3(offset(u32))]
                        field_type = Some(FieldType::Offset(parse_ident(&meta)?));
                    } else if meta.path.is_ident("offset_count") {
                        // #[xc3(offset_count(u32, u32))]
                        let (offset, count) = parse_two_idents(&meta)?;
                        field_type = Some(FieldType::OffsetCount(offset, count));
                    } else if meta.path.is_ident("count_offset") {
                        // #[xc3(count_offset(u32, u32)]
                        let (count, offset) = parse_two_idents(&meta)?;
                        field_type = Some(FieldType::CountOffset(count, offset));
                    } else if meta.path.is_ident("align") {
                        // #[xc3(align(4096))]
                        align = Some(parse_u64(&meta)?);
                    } else if meta.path.is_ident("pad_size_to") {
                        // #[xc3(pad_size_to(128))]
                        pad_size_to = Some(parse_u64(&meta)?);
                    } else if meta.path.is_ident("shared_offset") {
                        // #[xc3(shared_offset)]
                        field_type = Some(FieldType::SharedOffset);
                    } else if meta.path.is_ident("save_position") {
                        // #[xc3(save_position)]
                        field_type = Some(FieldType::SavePosition);
                    } else if meta.path.is_ident("skip") {
                        // #[xc3(skip)]
                        field_type = Some(FieldType::Skip);
                    }
                    Ok(())
                });
            }
        }

        Self {
            field_type,
            align,
            pad_size_to,
        }
    }
}

fn parse_u64(meta: &syn::meta::ParseNestedMeta<'_>) -> Result<u64, syn::Error> {
    let content;
    parenthesized!(content in meta.input);
    let lit: LitInt = content.parse().unwrap();
    lit.base10_parse()
}

fn parse_ident(meta: &syn::meta::ParseNestedMeta<'_>) -> Result<Ident, syn::Error> {
    let content;
    parenthesized!(content in meta.input);
    let ty: Ident = content.parse()?;
    Ok(ty)
}

fn parse_two_idents(meta: &syn::meta::ParseNestedMeta<'_>) -> Result<(Ident, Ident), syn::Error> {
    let content;
    parenthesized!(content in meta.input);
    let types = content.parse_terminated(Ident::parse_any, syn::Token![,])?;
    Ok((types[0].clone(), types[1].clone()))
}

pub struct TypeOptions {
    pub magic: Option<TokenStream>,
    pub has_base_offset: bool,
    pub align_after: Option<u64>,
}

impl TypeOptions {
    pub fn from_attrs(attrs: &[Attribute]) -> Self {
        let mut magic = None;
        let mut has_base_offset = false;
        let mut align_after = None;

        for a in attrs {
            if a.path().is_ident("xc3") {
                let _ = a.parse_nested_meta(|meta| {
                    if meta.path.is_ident("magic") {
                        // #[xc3(magic(b"MAGIC"))]
                        let content;
                        parenthesized!(content in meta.input);
                        let lit: TokenStream = content.parse().unwrap();
                        magic = Some(lit);
                    } else if meta.path.is_ident("base_offset") {
                        // #[xc3(base_offset)]
                        has_base_offset = true;
                    } else if meta.path.is_ident("align_after") {
                        // #[xc3(align_after(4096))]
                        align_after = Some(parse_u64(&meta)?);
                    }
                    Ok(())
                });
            }
        }

        Self {
            magic,
            has_base_offset,
            align_after,
        }
    }
}

pub struct VariantOptions {
    pub magic: Option<TokenStream>,
}

impl VariantOptions {
    pub fn from_attrs(attrs: &[Attribute]) -> Self {
        let mut magic = None;

        for a in attrs {
            if a.path().is_ident("xc3") {
                let _ = a.parse_nested_meta(|meta| {
                    if meta.path.is_ident("magic") {
                        // #[xc3(magic(b"MAGIC"))]
                        // #[xc3(magic(5u32))]
                        let content;
                        parenthesized!(content in meta.input);
                        let lit: TokenStream = content.parse().unwrap();
                        magic = Some(lit);
                    }
                    Ok(())
                });
            }
        }

        Self { magic }
    }
}

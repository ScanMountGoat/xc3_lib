use syn::{parenthesized, Attribute, LitByteStr, LitInt};

pub struct FieldOptions {
    pub field_type: Option<FieldType>,
    pub align: Option<u64>,
    pub pad_size_to: Option<u64>,
}

// TODO: Separate count field similar to #[bw(calc(...))]?
pub enum FieldType {
    Offset16,
    Offset32,
    Offset64,
    Offset32Count32,
    Count32Offset32,
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
                    if meta.path.is_ident("offset16") {
                        // #[xc3(offset16)]
                        field_type = Some(FieldType::Offset16);
                    } else if meta.path.is_ident("offset32") {
                        // #[xc3(offset32)]
                        field_type = Some(FieldType::Offset32);
                    } else if meta.path.is_ident("offset64") {
                        // #[xc3(offset64)]
                        field_type = Some(FieldType::Offset64);
                    } else if meta.path.is_ident("offset32_count32") {
                        // #[xc3(offset32_count32)]
                        field_type = Some(FieldType::Offset32Count32);
                    } else if meta.path.is_ident("count32_offset32") {
                        // #[xc3(count32_offset32)]
                        field_type = Some(FieldType::Count32Offset32);
                    } else if meta.path.is_ident("align") {
                        // TODO: Support constants like PAGE_SIZE?
                        // #[xc3(align(4096))]
                        align = Some(parse_u64(&meta)?);
                    } else if meta.path.is_ident("pad_size_to") {
                        // #[xc3(pad_size_to(128))]
                        pad_size_to = Some(parse_u64(&meta)?);
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

pub struct TypeOptions {
    pub magic: Option<LitByteStr>,
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
                        let lit: LitByteStr = content.parse().unwrap();
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

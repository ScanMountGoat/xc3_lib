use proc_macro::TokenStream;
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::quote;
use syn::{
    parenthesized, parse_macro_input, Attribute, Data, DataStruct, DeriveInput, Fields, Ident,
    LitByteStr,
};

#[proc_macro_derive(Xc3Write, attributes(xc3))]
pub fn xc3_write_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let name = &input.ident;
    let offsets_name = Ident::new(&(input.ident.to_string() + "Offsets"), Span::call_site());

    let (write_fields, offset_fields) = write_field_data(&input.data);

    // Some types need a pointer to the start of the type.
    let has_base_offset = has_base_offset(&input.attrs);
    let base_offset_field = has_base_offset.then_some(quote!(pub base_offset: u64,));
    let base_offset = has_base_offset.then_some(quote!(base_offset,));
    let set_base_offset =
        has_base_offset.then_some(quote!(let base_offset = writer.stream_position()?;));

    let write_magic = file_magic(&input.attrs).map(|m| quote!(#m.write_le(writer)?;));

    quote! {
        pub(crate) struct #offsets_name {
            #base_offset_field
            #(pub #offset_fields: u64),*
        }

        impl crate::write::Xc3Write for #name {
            type Offsets = #offsets_name;

            fn write<W: std::io::Write + std::io::Seek>(
                &self,
                writer: &mut W,
                data_ptr: &mut u64,
            ) -> binrw::BinResult<Self::Offsets> {
                #set_base_offset

                #write_magic

                // Write data and placeholder offsets.
                #(#write_fields)*

                // Point past current write.
                *data_ptr = (*data_ptr).max(writer.stream_position()?);

                // Return positions of offsets to update later.
                Ok(#offsets_name { #base_offset #(#offset_fields),* })
            }
        }
    }
    .into()
}

enum FieldType {
    Offset,
    OffsetCount,
    CountOffset,
}

// TODO: Create an options struct?
fn file_magic(attrs: &[Attribute]) -> Option<LitByteStr> {
    // #[xc3(magic(b"MAGIC"))]
    let mut magic = None;

    for a in attrs {
        if a.path().is_ident("xc3") {
            let _ = a.parse_nested_meta(|meta| {
                if meta.path.is_ident("magic") {
                    let content;
                    parenthesized!(content in meta.input);
                    let lit: LitByteStr = content.parse().unwrap();
                    magic = Some(lit);
                }
                Ok(())
            });
        }
    }

    magic
}

fn has_base_offset(attrs: &[Attribute]) -> bool {
    // #[xc3(base_offset)]
    let mut has_base_offset = false;

    for a in attrs {
        if a.path().is_ident("xc3") {
            let _ = a.parse_nested_meta(|meta| {
                if meta.path.is_ident("base_offset") {
                    has_base_offset = true;
                }
                Ok(())
            });
        }
    }

    has_base_offset
}

fn field_type(attrs: &[Attribute]) -> Option<FieldType> {
    // #[xc3(offset)], #[xc3(count_offset)], #[xc3(offset_count)]
    let mut ty = None;

    for a in attrs {
        if a.path().is_ident("xc3") {
            a.parse_nested_meta(|meta| {
                if meta.path.is_ident("offset") {
                    ty = Some(FieldType::Offset);
                } else if meta.path.is_ident("offset_count") {
                    ty = Some(FieldType::OffsetCount);
                } else if meta.path.is_ident("count_offset") {
                    ty = Some(FieldType::CountOffset);
                }

                Ok(())
            })
            .unwrap();
        }
    }

    ty
}

fn write_field_data(data: &Data) -> (Vec<TokenStream2>, Vec<Ident>) {
    let mut write_fields = Vec::new();
    let mut offset_fields = Vec::new();

    match data {
        syn::Data::Struct(DataStruct {
            fields: Fields::Named(fields),
            ..
        }) => {
            for f in fields.named.iter() {
                let name = f.ident.as_ref().unwrap();

                // Check if we need to write the count.
                // Use a null offset as a placeholder.
                match field_type(&f.attrs) {
                    Some(FieldType::Offset) => {
                        write_fields.push(quote! {
                            let #name = writer.stream_position()?;
                            0u32.write_le(writer)?;
                        });
                        offset_fields.push(name.clone());
                    }
                    Some(FieldType::CountOffset) => {
                        write_fields.push(quote! {
                            (self.#name.len() as u32).write_le(writer)?;
                            let #name = writer.stream_position()?;
                            0u32.write_le(writer)?;
                        });
                        offset_fields.push(name.clone());
                    }
                    Some(FieldType::OffsetCount) => {
                        write_fields.push(quote! {
                            let #name = writer.stream_position()?;
                            0u32.write_le(writer)?;
                            (self.#name.len() as u32).write_le(writer)?;
                        });
                        offset_fields.push(name.clone());
                    }
                    None => write_fields.push(quote! {
                        self.#name.write_le(writer)?;
                    }),
                }
            }
        }
        syn::Data::Enum(_) => todo!(),
        syn::Data::Union(_) => todo!(),
        _ => panic!("Unsupported type"),
    }

    (write_fields, offset_fields)
}

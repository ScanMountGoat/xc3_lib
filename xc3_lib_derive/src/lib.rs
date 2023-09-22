use attribute::{FieldOptions, FieldType, TypeOptions};
use proc_macro::TokenStream;
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::quote;
use syn::{parse_macro_input, Data, DataStruct, DeriveInput, Fields, Ident, Type};

mod attribute;

#[proc_macro_derive(Xc3Write, attributes(xc3))]
pub fn xc3_write_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let name = &input.ident;
    let offsets_name = offsets_name(&input.ident);

    let FieldData {
        write_fields,
        offset_field_names,
        offset_fields,
    } = parse_field_data(&input.data);

    let options = TypeOptions::from_attrs(&input.attrs);

    // Some types need a pointer to the start of the type.
    let base_offset_field = options
        .has_base_offset
        .then_some(quote!(pub base_offset: u64,));
    let base_offset = options.has_base_offset.then_some(quote!(base_offset,));
    let set_base_offset = options
        .has_base_offset
        .then_some(quote!(let base_offset = writer.stream_position()?;));

    let write_magic = options.magic.map(|m| quote!(#m.write_le(writer)?;));

    // TODO: move offset struct generation to the field data?
    quote! {
        pub(crate) struct #offsets_name<'a> {
            #base_offset_field
            #(#offset_fields),*
        }

        impl crate::write::Xc3Write for #name {
            type Offsets<'a> = #offsets_name<'a>;

            fn xc3_write<W: std::io::Write + std::io::Seek>(
                &self,
                writer: &mut W,
                data_ptr: &mut u64,
            ) -> binrw::BinResult<Self::Offsets<'_>> {
                use binrw::BinWrite;
                #set_base_offset

                #write_magic

                // Write data and placeholder offsets.
                #(#write_fields)*

                // Point past current write.
                *data_ptr = (*data_ptr).max(writer.stream_position()?);

                // Return positions of offsets to update later.
                Ok(#offsets_name { #base_offset #(#offset_field_names),* })
            }
        }
    }
    .into()
}

// Share attributes with Xc3Write.
#[proc_macro_derive(Xc3WriteFull, attributes(xc3))]
pub fn xc3_write_full_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let offsets_name = offsets_name(&input.ident);

    let FieldData {
        offset_field_names, ..
    } = parse_field_data(&input.data);

    let options = TypeOptions::from_attrs(&input.attrs);
    let self_base_offset = if options.has_base_offset {
        quote!(self.base_offset;)
    } else {
        quote!(base_offset)
    };

    // TODO: How to handle the base offset?
    let write_fields: Vec<_> = offset_field_names
        .iter()
        .map(|f| quote!(self.#f.write_full(writer, base_offset, data_ptr)?;))
        .collect();

    // The offsets are the last thing to be written.
    // Final alignment should go here instead of Xc3Write.
    // TODO: Share logic with pad_size_to?
    let align_after = options.align_after.map(|align| {
        quote! {
            // Round up the total size.
            let size = writer.stream_position()?;
            let round_up = |x, n| ((x + n - 1) / n) * n;
            let desired_size = round_up(size, #align);
            let padding = desired_size - size;
            writer.write_all(&vec![0u8; padding as usize])?;

            // Point past current write.
            *data_ptr = (*data_ptr).max(writer.stream_position()?);
        }
    });

    // Add a write impl to the offset type to support nested types.
    // Vecs need to be able to write all items before the pointed to data.
    quote! {
        impl<'a> crate::write::Xc3WriteFull for #offsets_name<'a> {
            fn write_full<W: std::io::Write + std::io::Seek>(
                &self,
                writer: &mut W,
                base_offset: u64,
                data_ptr: &mut u64,
            ) -> binrw::BinResult<()> {
                // Assume data is arranged in order by field.
                // TODO: investigate deriving other orderings.
                let base_offset = #self_base_offset;
                #(#write_fields)*

                #align_after

                Ok(())
            }
        }
    }
    .into()
}

fn offsets_name(ident: &Ident) -> Ident {
    Ident::new(&(ident.to_string() + "Offsets"), Span::call_site())
}

struct FieldData {
    write_fields: Vec<TokenStream2>,
    offset_field_names: Vec<Ident>,
    offset_fields: Vec<TokenStream2>,
}

fn parse_field_data(data: &Data) -> FieldData {
    let mut write_fields = Vec::new();
    let mut offset_field_names = Vec::new();
    let mut offset_fields = Vec::new();

    match data {
        syn::Data::Struct(DataStruct {
            fields: Fields::Named(fields),
            ..
        }) => {
            for f in fields.named.iter() {
                let name = f.ident.as_ref().unwrap();
                let ty = &f.ty;

                let options = FieldOptions::from_attrs(&f.attrs);
                let offset = create_offset_struct(name, options.align);

                let pad_size_to = options.pad_size_to.map(|desired_size| {
                    quote! {
                        // Add appropriate padding until desired size.
                        let after_pos = writer.stream_position()?;
                        let size = after_pos - before_pos;
                        let padding = #desired_size - size;
                        writer.write_all(&vec![0u8; padding as usize])?;

                        // Point past current write.
                        *data_ptr = (*data_ptr).max(writer.stream_position()?);
                    }
                });

                // Check if we need to write the count.
                // Use a null offset as a placeholder.
                match options.field_type {
                    Some(FieldType::Offset) => {
                        write_fields.push(quote! {
                            let #name = #offset;
                            0u32.write_le(writer)?;
                        });
                        offset_fields.push(offset_field(name, ty));
                        offset_field_names.push(name.clone());
                    }
                    Some(FieldType::CountOffset) => {
                        write_fields.push(quote! {
                            (self.#name.len() as u32).write_le(writer)?;
                            let #name = #offset;
                            0u32.write_le(writer)?;
                        });
                        offset_fields.push(offset_field(name, ty));
                        offset_field_names.push(name.clone());
                    }
                    Some(FieldType::OffsetCount) => {
                        write_fields.push(quote! {
                            let #name = #offset;
                            0u32.write_le(writer)?;
                            (self.#name.len() as u32).write_le(writer)?;
                        });
                        offset_fields.push(offset_field(name, ty));
                        offset_field_names.push(name.clone());
                    }
                    None => write_fields.push(quote! {
                        let before_pos = writer.stream_position()?;
                        self.#name.xc3_write(writer, data_ptr)?;
                        #pad_size_to
                    }),
                }
            }
        }
        syn::Data::Enum(_) => todo!(),
        syn::Data::Union(_) => todo!(),
        _ => panic!("Unsupported type"),
    }

    FieldData {
        write_fields,
        offset_field_names,
        offset_fields,
    }
}

fn offset_field(name: &Ident, ty: &Type) -> TokenStream2 {
    quote!(pub #name: crate::write::Offset<'a, #ty>)
}

fn create_offset_struct(name: &Ident, alignment: Option<u64>) -> TokenStream2 {
    let alignment = match alignment {
        Some(align) => quote!(Some(#align)),
        None => quote!(None),
    };
    quote!(crate::write::Offset::new(writer.stream_position()?, &self.#name, #alignment))
}

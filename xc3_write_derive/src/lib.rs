use attribute::{FieldOptions, FieldType, Padding, TypeOptions, VariantOptions};
use proc_macro::TokenStream;
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::quote;
use syn::{
    parse_macro_input, Data, DataEnum, DataStruct, DeriveInput, Fields, FieldsNamed, GenericParam,
    Ident, Lifetime, LifetimeParam, Type,
};

mod attribute;

#[proc_macro_derive(Xc3Write, attributes(xc3))]
pub fn xc3_write_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let type_name = &input.ident;

    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    // The lifetime isn't part of the parent struct, so add it here.
    let mut offset_generics = input.generics.clone();
    offset_generics.params.insert(
        0,
        GenericParam::Lifetime(LifetimeParam::new(Lifetime::new(
            "'offsets",
            Span::call_site(),
        ))),
    );
    let offsets = offsets_name(&input.ident);
    let offsets_type = quote!(#offsets #offset_generics);

    let options = TypeOptions::from_attrs(&input.attrs);

    // Some types need a pointer to the start of the type.
    let base_offset_field = options
        .has_base_offset
        .then_some(quote!(pub base_offset: u64,));
    let base_offset = options.has_base_offset.then_some(quote!(base_offset,));
    let set_base_offset = options
        .has_base_offset
        .then_some(quote!(let base_offset = writer.stream_position()?;));

    let write_magic = options
        .magic
        .map(|m| quote!(#m.xc3_write(writer, endian)?;));

    let (write_data, define_offsets, initialize_offsets) = match &input.data {
        Data::Struct(DataStruct {
            fields: Fields::Named(fields),
            ..
        }) => {
            let fields = parse_named_fields(fields);

            let offset_fields = fields.iter().map(|f| &f.offset_field);

            let define_offsets = quote! {
                #[doc(hidden)]
                pub struct #offsets #offset_generics #where_clause {
                    #base_offset_field
                    #(#offset_fields),*
                }
            };

            let offset_field_names = fields.iter().map(|f| &f.name);
            let initialize_offsets = quote! {
                Ok(#offsets { #base_offset #(#offset_field_names),* })
            };

            let write_fields = fields.iter().map(|f| &f.write_impl);
            let write_data = quote!(#(#write_fields)*);

            (write_data, define_offsets, initialize_offsets)
        }
        Data::Enum(DataEnum { variants, .. }) => {
            let offset_fields = variants.iter().map(|variant| {
                let name = &variant.ident;
                match &variant.fields {
                    Fields::Named(_) => todo!(),
                    Fields::Unnamed(unnamed) => {
                        // TODO: Don't assume just one field.
                        let field0 = &unnamed.unnamed.first().unwrap().ty;
                        quote!(#name(<#field0 as ::xc3_write::Xc3Write>::Offsets<'offsets>))
                    }
                    Fields::Unit => quote!(#name),
                }
            });

            let define_offsets = quote! {
                #[doc(hidden)]
                pub enum #offsets_type #where_clause {
                    #(#offset_fields),*
                }
            };

            let write_variants = variants.iter().map(|variant| {
                let name = &variant.ident;
                let variant_options = VariantOptions::from_attrs(&variant.attrs);
                // TODO: Use xc3_write for this?
                let write_magic = variant_options
                    .magic
                    .map(|magic| quote!(#magic.xc3_write(writer, endian)?;));
                match &variant.fields {
                    Fields::Named(_) => todo!(),
                    // TODO: Don't assume one field.
                    Fields::Unnamed(_) => quote! {
                        Self::#name(data) => {
                            #write_magic
                            #offsets::#name(data.xc3_write(writer, endian)?)
                        }
                    },
                    Fields::Unit => quote!(Self::#name => #offsets::#name),
                }
            });
            let write_data = quote! {
                let offsets = match self {
                    #(#write_variants),*
                };
            };

            let initialize_offsets = quote!(Ok(offsets));

            (write_data, define_offsets, initialize_offsets)
        }
        _ => panic!("Unsupported type"),
    };

    let alignment = options
        .align
        .map(|align| quote!(const ALIGNMENT: u64 = #align;));

    quote! {
        #define_offsets

        impl #impl_generics ::xc3_write::Xc3Write for #type_name #ty_generics #where_clause {
            type Offsets<'offsets> = #offsets_type;

            fn xc3_write<W: std::io::Write + std::io::Seek>(
                &self,
                writer: &mut W,
                endian: ::xc3_write::Endian,
            ) -> ::xc3_write::Xc3Result<Self::Offsets<'_>> {
                #set_base_offset

                #write_magic

                // Write data and placeholder offsets.
                #write_data

                // Return positions of offsets to update later.
                #initialize_offsets
            }

            #alignment
        }
    }
    .into()
}

// Share attributes with Xc3Write.
#[proc_macro_derive(Xc3WriteOffsets, attributes(xc3))]
pub fn xc3_write_offsets_derive(input: TokenStream) -> TokenStream {
    let mut input = parse_macro_input!(input as DeriveInput);

    // The lifetime isn't part of the parent struct, so add it here.
    input.generics.params.insert(
        0,
        GenericParam::Lifetime(LifetimeParam::new(Lifetime::new(
            "'offsets",
            Span::call_site(),
        ))),
    );
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    let offsets_name = offsets_name(&input.ident);

    let options = TypeOptions::from_attrs(&input.attrs);
    let self_base_offset = if options.has_base_offset {
        quote!(self.base_offset;)
    } else {
        quote!(base_offset)
    };

    // The offsets are the last thing to be written.
    // Final alignment should go here instead of Xc3Write.
    // TODO: Share logic with pad_size_to?
    let align_after = options.align_after.map(|align| {
        quote! {
            // Round up the total size.
            let size = *data_ptr;
            let desired_size = size.next_multiple_of(#align);
            let padding = desired_size - size;
            writer.seek(std::io::SeekFrom::Start(*data_ptr))?;
            writer.write_all(&vec![0u8; padding as usize])?;
            *data_ptr = writer.stream_position()?;
        }
    });

    let write_offset_fields = match &input.data {
        Data::Struct(DataStruct {
            fields: Fields::Named(fields),
            ..
        }) => {
            let fields = parse_named_fields(fields);

            let write_fields = fields.iter().map(|f| f.write_offset_impl.clone());
            quote!(#(#write_fields)*)
        }
        Data::Enum(DataEnum { variants, .. }) => {
            // TODO: Named fields?
            let write_variants = variants.iter().map(|variant| {
                let name = &variant.ident;
                match &variant.fields {
                    Fields::Named(_) => todo!(),
                    Fields::Unnamed(_) => quote! {
                        // TODO: Don't assume one field.
                        Self::#name(data) => data.write_offsets(writer, base_offset, data_ptr, endian, ())?
                    },
                    Fields::Unit => quote!(Self::#name =>()),
                }
            });

            quote! {
                match self {
                    #(#write_variants),*
                }
            }
        }
        _ => panic!("Unsupported type"),
    };

    // Add a write impl to the offset type to support nested types.
    // Vecs need to be able to write all items before the pointed to data.
    quote! {
        impl #impl_generics ::xc3_write::Xc3WriteOffsets for #offsets_name #ty_generics #where_clause {
            type Args = ();

            fn write_offsets<W: std::io::Write + std::io::Seek>(
                &self,
                writer: &mut W,
                base_offset: u64,
                data_ptr: &mut u64,
                endian: ::xc3_write::Endian,
                args: Self::Args,
            ) -> ::xc3_write::Xc3Result<()> {
                // Assume data is arranged in order by field.
                // TODO: investigate deriving other orderings.
                let base_offset = #self_base_offset;
                #write_offset_fields

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

// Collect writing related information and code for each field.
struct FieldData {
    name: Ident,
    offset_field: TokenStream2,
    write_impl: TokenStream2,
    write_offset_impl: TokenStream2,
}

impl FieldData {
    fn offset(name: &Ident, alignment: Option<Padding>, pointer: &Ident, ty: &Type) -> Self {
        Self {
            name: name.clone(),
            offset_field: offset_field(name, pointer, ty),
            write_impl: write_dummy_offset(name, alignment, pointer),
            write_offset_impl: quote! {
                self.#name.write_full(writer, base_offset, data_ptr, endian, args)?;
            },
        }
    }

    fn shared_offset(name: &Ident, alignment: Option<Padding>, pointer: &Type) -> Self {
        Self {
            name: name.clone(),
            offset_field: quote!(pub #name: ::xc3_write::Offset<'offsets, #pointer, ()>),
            write_impl: write_dummy_shared_offset(name, alignment, pointer),
            write_offset_impl: quote! {
                self.#name.write_full(writer, base_offset, data_ptr, endian, args)?;
            },
        }
    }

    fn field_position(name: &Ident, ty: &Type, should_write: bool) -> Self {
        Self {
            name: name.clone(),
            offset_field: quote!(pub #name: ::xc3_write::FieldPosition<'offsets, #ty>),
            write_impl: write_field_position(name, should_write),
            write_offset_impl: quote!(),
        }
    }
}

fn write_dummy_offset(name: &Ident, alignment: Option<Padding>, pointer: &Ident) -> TokenStream2 {
    let align = match alignment.map(|a| a.size) {
        Some(align) => quote!(Some(#align)),
        None => quote!(None),
    };
    let padding_byte = alignment.map(|a| a.value).unwrap_or_default();

    quote! {
        let #name = ::xc3_write::Offset::new(writer.stream_position()?, &self.#name, #align, #padding_byte);
        // Assume 0 is the default for the pointer type.
        #pointer::default().xc3_write(writer, endian)?;
    }
}

fn write_dummy_shared_offset(
    name: &Ident,
    alignment: Option<Padding>,
    pointer: &Type,
) -> TokenStream2 {
    let align = match alignment.map(|a| a.size) {
        Some(align) => quote!(Some(#align)),
        None => quote!(None),
    };
    let padding_byte = alignment.map(|a| a.value).unwrap_or_default();

    quote! {
        let #name = ::xc3_write::Offset::new(writer.stream_position()?, &(), #align, #padding_byte);
        // Assume 0 is the default for the pointer type.
        #pointer::default().xc3_write(writer, endian)?;
    }
}

fn write_field_position(name: &Ident, should_write: bool) -> TokenStream2 {
    if should_write {
        quote! {
            let #name = ::xc3_write::FieldPosition::new(writer.stream_position()?, &self.#name);
            self.#name.xc3_write(writer, endian)?;
        }
    } else {
        quote! {
            let #name = ::xc3_write::FieldPosition::new(writer.stream_position()?, &self.#name);
        }
    }
}

fn parse_named_fields(fields: &FieldsNamed) -> Vec<FieldData> {
    let mut offset_fields = Vec::new();

    for f in fields.named.iter() {
        let name = f.ident.as_ref().unwrap();
        let ty = &f.ty;

        let options = FieldOptions::from_attrs(&f.attrs);

        let pad_size_to = options.pad_size_to.map(|desired_size| {
            // TODO: padding value?
            let desired_size = desired_size.size;
            quote! {
                // Add appropriate padding until desired size.
                let after_pos = writer.stream_position()?;
                let size = after_pos - before_pos;
                let padding = #desired_size - size;
                writer.write_all(&vec![0u8; padding as usize])?;
            }
        });

        // Check if we need to write the count.
        // Use a null offset as a placeholder.
        // TODO: Reduce repeated code?
        let mut offset_field = match &options.field_type {
            Some(FieldType::Offset(offset_ty)) => {
                FieldData::offset(name, options.align, &offset_ty, ty)
            }
            Some(FieldType::CountOffset(count_ty, offset_ty)) => {
                let write_offset = write_dummy_offset(name, options.align, &offset_ty);

                FieldData {
                    name: name.clone(),
                    offset_field: offset_field(name, &offset_ty, ty),
                    write_impl: quote! {
                        (self.#name.len() as #count_ty).xc3_write(writer, endian)?;
                        #write_offset
                    },
                    write_offset_impl: quote! {
                        self.#name.write_full(writer, base_offset, data_ptr, endian, args)?;
                    },
                }
            }
            Some(FieldType::OffsetCount(offset_ty, count_ty)) => {
                let write_offset = write_dummy_offset(name, options.align, &offset_ty);

                FieldData {
                    name: name.clone(),
                    offset_field: offset_field(name, &offset_ty, ty),
                    write_impl: quote! {
                        #write_offset
                        (self.#name.len() as #count_ty).xc3_write(writer, endian)?;
                    },
                    write_offset_impl: quote! {
                        self.#name.write_full(writer, base_offset, data_ptr, endian, args)?;
                    },
                }
            }
            Some(FieldType::SharedOffset) => {
                // Shared offsets don't actually contain any data.
                // The pointer type is the type of the field itself.
                FieldData::shared_offset(name, options.align, ty)
            }
            Some(FieldType::SavePosition) => {
                // Store the information for later shared offsets.
                FieldData::field_position(name, ty, !options.skip)
            }
            Some(FieldType::OffsetSize(offset_ty, size_ty)) => {
                let write_offset = write_dummy_offset(name, options.align, &offset_ty);
                FieldData {
                    name: name.clone(),
                    offset_field: offset_field(name, &offset_ty, ty),
                    write_impl: quote! {
                        #write_offset
                        {
                            use binrw::BinWrite;
                            let mut cur = std::io::Cursor::new(Vec::new());
                            self.#name.xc3_write(&mut cur, endian)?;
                            cur.into_inner().len() as #size_ty
                        }.xc3_write(writer, endian)?;
                    },
                    write_offset_impl: quote! {
                        self.#name.write_full(writer, base_offset, data_ptr, endian, args)?;
                    },
                }
            }
            Some(FieldType::OffsetInnerCount(offset_ty, count_expr)) => {
                let write_offset = write_dummy_offset(name, options.align, &offset_ty);

                FieldData {
                    name: name.clone(),
                    offset_field: offset_field(name, &offset_ty, ty),
                    write_impl: quote! {
                        #write_offset
                        (#count_expr).xc3_write(writer, endian)?;
                    },
                    write_offset_impl: quote! {
                        self.#name.write_full(writer, base_offset, data_ptr, endian, args)?;
                    },
                }
            }
            None => {
                // Also include fields not marked as offsets in the struct.
                // The field type may have offsets that need to be written later.
                let write_impl = if options.pad_size_to.is_some() {
                    quote! {
                        let before_pos = writer.stream_position()?;
                        let #name = self.#name.xc3_write(writer, endian)?;
                        #pad_size_to
                    }
                } else {
                    quote! {
                        let #name = self.#name.xc3_write(writer, endian)?;
                    }
                };
                FieldData {
                    name: name.clone(),
                    offset_field: quote!(pub #name: <#ty as ::xc3_write::Xc3Write>::Offsets<'offsets>),
                    write_impl,
                    write_offset_impl: quote! {
                        // This field isn't an Offset<T>, so just call write_offsets.
                        self.#name.write_offsets(writer, base_offset, data_ptr, endian, args)?;
                    },
                }
            }
        };
        // Save position already checks for the skip attribute.
        if !matches!(options.field_type, Some(FieldType::SavePosition)) && options.skip {
            offset_field.write_impl = quote!();
        }

        offset_fields.push(offset_field);
    }

    offset_fields
}

fn offset_field(name: &Ident, pointer: &Ident, ty: &Type) -> TokenStream2 {
    quote!(pub #name: ::xc3_write::Offset<'offsets, #pointer, #ty>)
}

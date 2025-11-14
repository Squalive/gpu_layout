mod std140;

use proc_macro2::Span;
use quote::quote;
use syn::{Data, DataStruct, DeriveInput, Field, Fields, FieldsNamed, parse_macro_input};

#[proc_macro_derive(GpuLayout)]
pub fn derive_gpu_layout(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let fields = match get_named_struct_fields(&input.data) {
        Ok(f) => f,
        Err(e) => return e.into_compile_error().into(),
    };

    let field_datas: Vec<_> = fields
        .named
        .iter()
        .map(|field| FieldData {
            field: field.clone(),
            size: None,
            alignment: None,
        })
        .collect();

    let std140_derive = std140::derive(&input, &field_datas);

    fn gen_body<'a>(
        field_datas: &'a [FieldData],
        get_main: impl Fn(&proc_macro2::Ident) -> proc_macro2::TokenStream + 'a,
        get_padding: impl Fn(proc_macro2::TokenStream) -> proc_macro2::TokenStream + 'a,
    ) -> impl Iterator<Item = proc_macro2::TokenStream> + 'a {
        field_datas.iter().enumerate().map(move |(i, data)| {
            let ident = data.ident();

            let padding = {
                let i = proc_macro2::Literal::usize_suffixed(i);
                quote! { <Self as ::gpu_layout::GpuLayout<Layout>>::METADATA.padding(#i) }
            };

            let main = get_main(ident);
            let padding = get_padding(padding);

            quote! {
                #main
                #padding
            }
        })
    }

    let num_of_fields = &proc_macro2::Literal::usize_suffixed(field_datas.len());

    let field_types = field_datas.iter().map(|data| &data.field.ty);
    let field_types_1 = field_datas.iter().map(|data| &data.field.ty);

    let write_info_buffer_body = gen_body(
        &field_datas,
        |ident| {
            quote! {
                ::gpu_layout::WriteInto::<Layout>::write_into(&self.#ident, writer);
            }
        },
        |padding| {
            quote! {
                ::gpu_layout::Writer::advance(writer, #padding as ::core::primitive::usize);
            }
        },
    );

    let read_from_buffer_body = gen_body(
        &field_datas,
        |ident| {
            quote! {
                ::gpu_layout::ReadFrom::<Layout>::read_from(&mut self.#ident, reader);
            }
        },
        |padding| {
            quote! {
                ::gpu_layout::Reader::advance(reader, #padding as ::core::primitive::usize);
            }
        },
    );

    let name = &input.ident;
    let mut generics = input.generics.clone();
    generics
        .params
        .push(syn::GenericParam::Type(syn::parse_quote!(Layout)));
    let (impl_generics, _, _) = generics.split_for_impl();
    let (_, ty_generics, _) = input.generics.split_for_impl();

    quote! {
        #std140_derive

        impl #impl_generics ::gpu_layout::WriteInto<Layout> for #name #ty_generics
		where
			Self: ::gpu_layout::GpuLayout<Layout, ExtraMetadata = ::gpu_layout::StructMetadata<#num_of_fields>>,
			#( for<'__> #field_types: ::gpu_layout::WriteInto<Layout>, )*
		{
			#[inline]
			fn write_into<B: ::gpu_layout::BufferMut>(&self, writer: &mut ::gpu_layout::Writer<B>) {
				#( #write_info_buffer_body )*
			}
		}

		impl #impl_generics ::gpu_layout::ReadFrom<Layout> for #name #ty_generics
		where
			Self: ::gpu_layout::GpuLayout<Layout, ExtraMetadata = ::gpu_layout::StructMetadata<#num_of_fields>>,
			#( for<'__> #field_types_1: ::gpu_layout::ReadFrom<Layout>, )*
		{
			#[inline]
			fn read_from<B: ::gpu_layout::BufferRef>(&mut self, reader: &mut ::gpu_layout::Reader<B>) {
				#( #read_from_buffer_body )*
			}
		}
    }
    .into()
}

fn get_named_struct_fields(data: &Data) -> syn::Result<&FieldsNamed> {
    match data {
        Data::Struct(DataStruct {
            fields: Fields::Named(fields),
            ..
        }) if !fields.named.is_empty() => Ok(fields),
        _ => Err(syn::Error::new(
            Span::call_site(),
            "Only non empty structs with named fields are supported",
        )),
    }
}

struct FieldData {
    field: Field,
    size: Option<(u32, Span)>,
    alignment: Option<(u32, Span)>,
}

impl FieldData {
    fn alignment(&self, layout: &proc_macro2::TokenStream) -> proc_macro2::TokenStream {
        if let Some((alignment, _)) = self.alignment {
            let alignment = proc_macro2::Literal::u64_suffixed(alignment as _);
            quote! { ::gpu_layout::AlignmentValue::new(#alignment) }
        } else {
            let ty = &self.field.ty;
            quote! { <#ty as ::gpu_layout::GpuLayout<#layout>>::METADATA.alignment() }
        }
    }

    fn size(&self, layout: &proc_macro2::TokenStream) -> proc_macro2::TokenStream {
        if let Some((size, _)) = self.size {
            let size = proc_macro2::Literal::u64_suffixed(size as _);
            quote! { #size }
        } else {
            let ty = &self.field.ty;
            quote! { <#ty as ::gpu_layout::GpuLayout<#layout>>::METADATA.min_size().0.get() }
        }
    }

    fn min_size(&self, layout: &proc_macro2::TokenStream) -> proc_macro2::TokenStream {
        if let Some((size, _)) = self.size {
            let size = proc_macro2::Literal::u64_suffixed(size as _);
            quote! { #size }
        } else {
            let ty = &self.field.ty;
            quote! { <#ty as ::gpu_layout::GpuLayout<#layout>>::METADATA.min_size().get() }
        }
    }

    fn extra_padding(&self, layout: &proc_macro2::TokenStream) -> Option<proc_macro2::TokenStream> {
        self.size.as_ref().map(|(size, _)| {
            let size = proc_macro2::Literal::u64_suffixed(*size as u64);
            let ty = &self.field.ty;
            let original_size =
                quote! { <#ty as ::gpu_layout::GpuLayoutSize<#layout>>::SIZE.get() };
            quote! { #size.saturating_sub(#original_size) }
        })
    }

    fn ident(&self) -> &proc_macro2::Ident {
        self.field.ident.as_ref().unwrap()
    }
}

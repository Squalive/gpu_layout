use quote::quote;
use syn::DeriveInput;

use crate::FieldData;

pub(super) fn derive(input: &DeriveInput, field_datas: &[FieldData]) -> proc_macro2::TokenStream {
    let last_field_index = field_datas.len() - 1;

    let layout_ts = quote! { ::gpu_layout::Std140 };

    let num_of_fields = &proc_macro2::Literal::usize_suffixed(field_datas.len());

    let alignments = field_datas.iter().map(|data| data.alignment(&layout_ts));

    let paddings = field_datas.iter().enumerate().map(|(i, current)| {
        let is_first = i == 0;
        let is_last = i == field_datas.len() - 1;

        let mut out = proc_macro2::TokenStream::new();

        if !is_first {
            let prev_i = i - 1;
            let alignment = current.alignment(&layout_ts);
            let extra_padding = field_datas
                .get(prev_i)
                .and_then(|prev| prev.extra_padding(&layout_ts))
                .map(|e| quote! { + #e });
            out.extend(quote! {
                offsets[#i] = #alignment.round_up(offset);

                let padding = #alignment.padding_needed_for(offset);
                offset += padding;
                paddings[#prev_i] = padding #extra_padding;
            });
        }

        if is_last {
            return out;
        }

        let size = current.size(&layout_ts);
        out.extend(quote! { offset += #size; });

        if is_last {
            let extra_padding = current
                .extra_padding(&layout_ts)
                .map(|extra_padding| quote! {+ #extra_padding});
            out.extend(quote! {
                paddings[#i] = struct_alignment.padding_needed_for(offset) #extra_padding;
            });
        }

        out
    });

    let last_field = field_datas.last().unwrap();
    let last_field_min_size = last_field.min_size(&layout_ts);

    let field_types = field_datas.iter().map(|data| &data.field.ty);
    let all_other = field_types.clone().take(last_field_index);
    let last_field_type = &last_field.field.ty;

    let name = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    quote! {
        impl #impl_generics ::gpu_layout::GpuLayout<#layout_ts> for #name #ty_generics #where_clause
        where
            #( #all_other: ::gpu_layout::GpuLayout<#layout_ts> + ::gpu_layout::GpuLayoutSize<#layout_ts>, )*
            #last_field_type: ::gpu_layout::GpuLayout<#layout_ts>
        {
            type ExtraMetadata = ::gpu_layout::StructMetadata<#num_of_fields>;
            const METADATA: ::gpu_layout::Metadata<Self::ExtraMetadata> = {
                let struct_alignment = ::gpu_layout::AlignmentValue::max([ ::gpu_layout::AlignmentValue::new(16), #( #alignments, )* ]);

                let extra = {
                    let mut paddings = [0; #num_of_fields];
                    let mut offsets = [0; #num_of_fields];
                    let mut offset = 0;
                    #( #paddings )*
                    ::gpu_layout::StructMetadata { offsets, paddings }
                };

                let min_size = {
                    let mut offset = extra.offsets[#num_of_fields - 1];
                    offset += #last_field_min_size;
                    ::gpu_layout::SizeValue::new(struct_alignment.round_up(offset))
                };

                ::gpu_layout::Metadata {
                    alignment: struct_alignment,
                    min_size,
                    is_pod: false,
                    extra,
                }
            };
        }
    }
}

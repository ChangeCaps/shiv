use proc_macro2::{Ident, Span, TokenStream};
use quote::quote;
use syn::{parse_macro_input, Data, DeriveInput, Fields};

#[proc_macro_derive(Component)]
pub fn derive_component(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    let expanded = quote! {
        impl #impl_generics shiv::world::Component for #name #ty_generics #where_clause {
            type Storage = shiv::storage::SparseStorage;
        }
    };

    proc_macro::TokenStream::from(expanded)
}

macro_rules! impl_label {
    ($input:ident, $trait:ident, $id:ident) => {{
        let ident = Ident::new(stringify!($trait), Span::call_site());
        let id = Ident::new(stringify!($id), Span::call_site());

        let name = &$input.ident;

        let label_impl = label_fn_impl(&$input, &id);

        let (impl_generics, ty_generics, where_clause) = $input.generics.split_for_impl();

        quote! {
            #[automatically_derived]
            impl #impl_generics shiv::schedule::#ident for #name #ty_generics #where_clause {
                #[inline]
                fn label(self) -> shiv::schedule::#id {
                    #label_impl
                }
            }
        }
    }};
}

#[proc_macro_derive(StageLabel)]
pub fn derive_stage_label(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let expanded = impl_label!(input, StageLabel, StageLabelId);
    proc_macro::TokenStream::from(expanded)
}

#[proc_macro_derive(SystemLabel)]
pub fn derive_system_label(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let expanded = impl_label!(input, SystemLabel, SystemLabelId);
    proc_macro::TokenStream::from(expanded)
}

fn label_fn_impl(input: &DeriveInput, id: &Ident) -> TokenStream {
    match input.data {
        Data::Struct(ref data) => {
            if !matches!(data.fields, Fields::Unit) {
                unimplemented!("Only unit structs are supported");
            }

            let name = input.ident.to_string();

            quote! {
                shiv::schedule::#id::from_raw_parts::<Self>(#name, 0u32)
            }
        }
        Data::Enum(ref data) => {
            let name = input.ident.to_string();

            let variants = data.variants.iter().enumerate().map(|(i, variant)| {
                if !matches!(variant.fields, Fields::Unit) {
                    unimplemented!("Only unit variants are supported");
                }

                let variant_ident = &variant.ident;
                let variant_name = variant.ident.to_string();
                let name = format!("{}::{}", name, variant_name);

                let i = i as u32;

                quote! {
                    Self::#variant_ident => shiv::schedule::#id::from_raw_parts::<Self>(#name, #i)
                }
            });

            quote! {
                match self {
                    #(#variants),*
                }
            }
        }
        _ => unimplemented!("Only structs and enums are supported"),
    }
}

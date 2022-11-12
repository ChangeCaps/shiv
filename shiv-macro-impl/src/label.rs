use proc_macro2::{Ident, Span, TokenStream};
use quote::quote;
use syn::{Data, DeriveInput, Fields, Path};

pub fn derive_label(input: DeriveInput, shiv: Path, label: &str) -> proc_macro2::TokenStream {
    let ident = Ident::new(label, Span::call_site());
    let id = Ident::new(&format!("{label}Id"), Span::call_site());

    let name = &input.ident;

    let label_impl = label_fn_impl(&input, &id, &shiv);

    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    quote! {
        #[automatically_derived]
        impl #impl_generics #shiv::schedule::#ident for #name #ty_generics #where_clause {
            #[inline]
            fn label(self) -> #shiv::schedule::#id {
                #label_impl
            }
        }
    }
}

fn label_fn_impl(input: &DeriveInput, id: &Ident, shiv: &Path) -> TokenStream {
    match input.data {
        Data::Struct(ref data) => {
            if !matches!(data.fields, Fields::Unit) {
                unimplemented!("Only unit structs are supported");
            }

            let name = input.ident.to_string();

            quote! {
                #shiv::schedule::#id::from_raw_parts::<Self>(#name, 0u32)
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
                    Self::#variant_ident => #shiv::schedule::#id::from_raw_parts::<Self>(#name, #i)
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

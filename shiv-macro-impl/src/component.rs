use quote::quote;
use syn::{DeriveInput, Path};

pub fn derive_component(input: DeriveInput, shiv: Path) -> proc_macro2::TokenStream {
    let name = input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    quote! {
        impl #impl_generics #shiv::world::Component for #name #ty_generics #where_clause {
            type Storage = #shiv::storage::DenseStorage;
        }
    }
}

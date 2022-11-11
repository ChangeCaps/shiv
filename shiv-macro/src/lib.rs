mod bundle;
mod label;
mod system_param;

use quote::quote;
use syn::{parse_macro_input, DeriveInput};

#[proc_macro_derive(SystemParam)]
pub fn derive_system_param(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    system_param::derive_system_param(input)
}

#[proc_macro_derive(Component)]
pub fn derive_component(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    let expanded = quote! {
        impl #impl_generics shiv::world::Component for #name #ty_generics #where_clause {
            type Storage = shiv::storage::DenseStorage;
        }
    };

    proc_macro::TokenStream::from(expanded)
}

#[proc_macro_derive(Bundle)]
pub fn derive_bundle(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    bundle::derive_bundle(input)
}

#[proc_macro_derive(StageLabel)]
pub fn derive_stage_label(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    label::derive_label(input, "StageLabel")
}

#[proc_macro_derive(SystemLabel)]
pub fn derive_system_label(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    label::derive_label(input, "SystemLabel")
}

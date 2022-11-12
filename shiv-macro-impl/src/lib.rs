pub mod bundle;
pub mod component;
pub mod label;
pub mod system_param;
pub use syn;

#[macro_export]
macro_rules! implement {
    ($path:expr) => {
        #[proc_macro_derive(SystemParam)]
        pub fn derive_system_param(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
            let input = $crate::syn::parse_macro_input!(input as $crate::syn::DeriveInput);
            ::std::convert::From::from($crate::system_param::derive_system_param(input, $path))
        }

        #[proc_macro_derive(Component)]
        pub fn derive_component(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
            let input = $crate::syn::parse_macro_input!(input as $crate::syn::DeriveInput);
            ::std::convert::From::from($crate::component::derive_component(input, $path))
        }

        #[proc_macro_derive(Bundle)]
        pub fn derive_bundle(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
            let input = $crate::syn::parse_macro_input!(input as $crate::syn::DeriveInput);
            ::std::convert::From::from($crate::bundle::derive_bundle(input, $path))
        }

        #[proc_macro_derive(StageLabel)]
        pub fn derive_stage_label(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
            let input = $crate::syn::parse_macro_input!(input as $crate::syn::DeriveInput);
            ::std::convert::From::from($crate::label::derive_label(input, $path, "StageLabel"))
        }

        #[proc_macro_derive(SystemLabel)]
        pub fn derive_system_label(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
            let input = $crate::syn::parse_macro_input!(input as $crate::syn::DeriveInput);
            ::std::convert::From::from($crate::label::derive_label(input, $path, "SystemLabel"))
        }
    };
}

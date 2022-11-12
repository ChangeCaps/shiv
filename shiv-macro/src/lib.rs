fn shiv_path() -> syn::Path {
    match proc_macro_crate::crate_name("shiv") {
        Ok(found) => match found {
            proc_macro_crate::FoundCrate::Itself => syn::parse_quote!(shiv),
            proc_macro_crate::FoundCrate::Name(name) => {
                let ident: syn::Ident = syn::parse_str(&name).unwrap();
                syn::parse_quote!(::#ident)
            }
        },
        Err(_) => syn::parse_quote!(shiv),
    }
}

shiv_macro_impl::implement!(shiv_path());

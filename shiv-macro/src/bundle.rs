use proc_macro2::Span;
use quote::quote;
use syn::{
    parse_macro_input, parse_quote, Data, DeriveInput, Fields, Generics, Index, Member, Type,
};

pub fn derive_bundle(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let mut input = parse_macro_input!(input as DeriveInput);
    let name = input.ident;

    let types = types(&input.data);
    let members = members(&input.data);

    let count = types.len();

    add_generics(&mut input.generics);
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    let expanded = quote! {
        #[automatically_derived]
        unsafe impl #impl_generics shiv::bundle::Bundle for #name #ty_generics #where_clause {
            type Iter = std::array::IntoIter<*mut ::std::primitive::u8, #count>;

            #[inline]
            fn components(components: &mut shiv::world::Components) -> ::std::vec::Vec<shiv::world::ComponentId> {
                ::std::vec![#(components.init_component::<#types>()),*]
            }

            #[inline]
            unsafe fn get_components(bundle: *mut Self) -> Self::Iter {
                std::array::IntoIter::new([#(unsafe { &mut (*bundle).#members as *mut _ as *mut ::std::primitive::u8 }),*])
            }
        }
    };

    proc_macro::TokenStream::from(expanded)
}

fn add_generics(generics: &mut Generics) {
    for param in generics.type_params_mut() {
        param.bounds.push(parse_quote!(shiv::world::Component));
    }
}

fn types(data: &Data) -> Vec<Type> {
    match data {
        Data::Struct(data) => field_types(&data.fields),
        _ => unimplemented!("Bundle can only be derived for structs"),
    }
}

fn members(data: &Data) -> Vec<Member> {
    match data {
        Data::Struct(data) => field_members(&data.fields),
        _ => unimplemented!("Bundle can only be derived for structs"),
    }
}

fn field_types(fields: &Fields) -> Vec<Type> {
    match fields {
        Fields::Named(fields) => fields.named.iter().map(|field| field.ty.clone()).collect(),
        Fields::Unnamed(fields) => fields
            .unnamed
            .iter()
            .map(|field| field.ty.clone())
            .collect(),
        Fields::Unit => Vec::new(),
    }
}

fn field_members(fields: &Fields) -> Vec<Member> {
    match fields {
        Fields::Named(fields) => fields
            .named
            .iter()
            .map(|field| Member::Named(field.ident.clone().unwrap()))
            .collect(),
        Fields::Unnamed(fields) => fields
            .unnamed
            .iter()
            .enumerate()
            .map(|(i, _)| {
                Member::Unnamed(Index {
                    index: i as u32,
                    span: Span::call_site(),
                })
            })
            .collect(),
        Fields::Unit => Vec::new(),
    }
}

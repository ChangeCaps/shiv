use proc_macro2::TokenStream;
use quote::quote;
use syn::{
    parse_macro_input, parse_quote, punctuated::Punctuated, token::Comma, Data, DeriveInput,
    GenericParam, Generics, Ident, Index, Type,
};

pub fn derive_system_param(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    validate_lifetimes(&input.generics);

    let fields = fields(&input.data);
    let field_idents = field_idents(&input.data);

    let state_generics = state_generics(&input.generics);
    let fetch_generics = fetch_generics(&input.generics);
    let read_only_generics = read_only_generics(&input.generics);

    let (state_impl_generics, state_ty_generics, state_where_clause) =
        state_generics.split_for_impl();
    let (fetch_impl_generics, _, _) = fetch_generics.split_for_impl();
    let (_, _, read_only_where_clause) = read_only_generics.split_for_impl();

    let marker_generics = marker_generics(&input.generics);
    let fetch_ty_generics = fetch_ty_generics(&input.generics, &fields);

    let indices = (0..fields.len()).map(|i| Index::from(i));

    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    let vis = input.vis;
    let name = input.ident;

    let expanded = quote! {
        const _: () = {
            #[automatically_derived]
            impl #impl_generics shiv::system::SystemParam for #name #ty_generics #where_clause {
                type Fetch = FetchState<#fetch_ty_generics>;
            }

            #vis struct FetchState #state_ty_generics #state_where_clause {
                state: __TSystemParamState,
                marker: ::std::marker::PhantomData<fn() -> (#marker_generics)>,
            }

            #[automatically_derived]
            unsafe impl #state_impl_generics shiv::system::ReadOnlySystemParamFetch for
                FetchState #state_ty_generics #read_only_where_clause
            {
            }

            #[automatically_derived]
            unsafe impl #state_impl_generics shiv::system::SystemParamState for FetchState
                #state_ty_generics #state_where_clause
            {
                #[inline]
                fn init(
                    world: &mut shiv::world::World,
                    meta: &mut shiv::system::SystemMeta,
                ) -> Self {
                    Self {
                        state: __TSystemParamState::init(world, meta),
                        marker: ::std::marker::PhantomData,
                    }
                }

                #[inline]
                fn apply(&mut self, world: &mut shiv::world::World) {
                    self.state.apply(world);
                }
            }

            #[automatically_derived]
            impl #fetch_impl_generics shiv::system::SystemParamFetch<'w, 's> for
                FetchState<#fetch_ty_generics> #state_where_clause
            {
                type Item = #name #ty_generics;

                #[inline]
                #[allow(dead_code)]
                unsafe fn get_param(
                    &'s mut self,
                    meta: &shiv::system::SystemMeta,
                    world: &'w shiv::world::World,
                    change_ticks: ::std::primitive::u32,
                ) -> Self::Item {
                    let param = shiv::system::SystemParamFetch::get_param(
                        &mut self.state,
                        meta,
                        world,
                        change_ticks
                    );

                    #name {#(#field_idents: param.#indices,)*}
                }
            }
        };
    };

    proc_macro::TokenStream::from(expanded)
}

fn validate_lifetimes(generics: &Generics) {
    for lifetime in generics.lifetimes() {
        let ident = &lifetime.lifetime.ident;

        if !(ident == "w" || ident == "s") {
            panic!(
                "Invalid lifetime: {}, only valid lifetimes are 'w and 's",
                ident
            );
        }
    }
}

fn has_lifetime(generics: &Generics, lifetime: &str) -> bool {
    for lt in generics.lifetimes() {
        if lt.lifetime.ident == lifetime {
            return true;
        }
    }

    false
}

fn fetch_generics(generics: &Generics) -> Generics {
    let mut generics = generics.clone();

    if !has_lifetime(&generics, "w") {
        generics.params.push(parse_quote!('w));
    }

    if !has_lifetime(&generics, "s") {
        generics.params.push(parse_quote!('s));
    }

    generics
}

fn state_generics(generics: &Generics) -> Generics {
    let mut generics = generics.clone();

    generics.params = generics
        .params
        .clone()
        .into_pairs()
        .filter(|param| match param.value() {
            syn::GenericParam::Lifetime(_) => false,
            _ => true,
        })
        .collect();

    generics.params.push(parse_quote!(
        __TSystemParamState: shiv::system::SystemParamState
    ));

    generics.make_where_clause().predicates.push(parse_quote!(
        Self: ::std::marker::Send + ::std::marker::Sync + 'static
    ));

    generics
}

fn read_only_generics(generics: &Generics) -> Generics {
    let mut generics = generics.clone();

    let where_clause = generics.make_where_clause();
    where_clause.predicates.push(parse_quote!(
        __TSystemParamState: shiv::system::ReadOnlySystemParamFetch
    ));
    where_clause.predicates.push(parse_quote!(
        Self: for<'w, 's> shiv::system::SystemParamFetch<'w, 's>
    ));

    generics
}

fn marker_generics(generics: &Generics) -> Punctuated<TokenStream, Comma> {
    let mut marker_generics = Punctuated::<TokenStream, Comma>::new();
    for generic in generics.params.iter() {
        if let GenericParam::Type(ty) = generic {
            let ident = &ty.ident;
            marker_generics.push(parse_quote!(#ident));
        }
    }

    marker_generics
}

fn fetch_ty_generics(generics: &Generics, fields: &[Type]) -> Punctuated<TokenStream, Comma> {
    let mut fetch_ty_generics = Punctuated::<TokenStream, Comma>::new();
    for generic in generics.params.iter() {
        if let GenericParam::Type(ty) = generic {
            let ident = &ty.ident;
            fetch_ty_generics.push(parse_quote!(#ident));
        }
    }

    fetch_ty_generics.push(quote!((#(<#fields as shiv::system::SystemParam>::Fetch,)*)));

    fetch_ty_generics
}

fn fields(data: &Data) -> Vec<Type> {
    match data {
        Data::Struct(s) => match &s.fields {
            syn::Fields::Named(fields) => {
                fields.named.iter().map(|field| field.ty.clone()).collect()
            }
            syn::Fields::Unnamed(_) => unimplemented!("Unnamed fields are not supported"),
            syn::Fields::Unit => Vec::new(),
        },
        _ => unimplemented!("Only structs are supported"),
    }
}

fn field_idents(data: &Data) -> Vec<Ident> {
    match data {
        Data::Struct(s) => match &s.fields {
            syn::Fields::Named(fields) => fields
                .named
                .iter()
                .map(|field| field.ident.clone().unwrap())
                .collect(),
            syn::Fields::Unnamed(_) => unimplemented!("Unnamed fields are not supported"),
            syn::Fields::Unit => Vec::new(),
        },
        _ => unimplemented!("Only structs are supported"),
    }
}

use std::collections::HashSet;

use fuel_abi_types::abi::full_program::FullTypeDeclaration;
use proc_macro2::{Ident, TokenStream};
use quote::quote;

use crate::{
    error::{Result, error},
    program_bindings::{
        custom_types::utils::extract_generic_parameters,
        generated_code::GeneratedCode,
        resolved_type::ResolvedType,
        utils::{Component, Components, tokenize_generics},
    },
};

/// Returns a TokenStream containing the declaration, `Parameterize`,
/// `Tokenizable` and `TryFrom` implementations for the enum described by the
/// given TypeDeclaration.
pub(crate) fn expand_custom_enum(
    type_decl: &FullTypeDeclaration,
    no_std: bool,
) -> Result<GeneratedCode> {
    let enum_type_path = type_decl.custom_type_path()?;
    let enum_ident = enum_type_path.ident().unwrap();

    let components = Components::new(&type_decl.components, false, enum_type_path.parent())?;
    if components.is_empty() {
        return Err(error!("enum must have at least one component"));
    }
    let generics = extract_generic_parameters(type_decl);

    let code = enum_decl(enum_ident, &components, &generics, no_std);

    let enum_code = GeneratedCode::new(code, HashSet::from([enum_ident.into()]), no_std);

    Ok(enum_code.wrap_in_mod(enum_type_path.parent()))
}

fn maybe_impl_error(enum_ident: &Ident, components: &Components) -> Option<TokenStream> {
    components.has_error_messages().then(|| {
        let display_match_branches = components.iter().map(|Component{ident, resolved_type, error_message}| {
            let error_msg = error_message.as_deref().expect("error message is there - checked above");
            if let ResolvedType::Unit = resolved_type {
                quote! {#enum_ident::#ident =>  ::std::write!(f, "{}", #error_msg)}
            } else {
                quote! {#enum_ident::#ident(val) => ::std::write!(f, "{}: {:?}", #error_msg, val)}
            }
        });

        let custom_display_impl = quote! {
            impl ::std::fmt::Display for #enum_ident {
                fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                   match &self {
                    #(#display_match_branches,)*
                   }
                }
            }
        };

        quote! {
            #custom_display_impl
            impl ::std::error::Error for #enum_ident{}
        }
    })
}

fn enum_decl(
    enum_ident: &Ident,
    components: &Components,
    generics: &[Ident],
    no_std: bool,
) -> TokenStream {
    let maybe_disable_std = no_std.then(|| quote! {#[NoStd]});

    let enum_variants = components.as_enum_variants();
    let unused_generics_variant = components.generate_variant_for_unused_generics(generics);
    let (_, generics_w_bounds) = tokenize_generics(generics);
    let maybe_impl_error = maybe_impl_error(enum_ident, components);

    quote! {
        #[allow(clippy::enum_variant_names)]
        #[derive(
            Clone,
            Debug,
            Eq,
            PartialEq,
            ::fuels::macros::Parameterize,
            ::fuels::macros::Tokenizable,
            ::fuels::macros::TryFrom,
        )]
        #maybe_disable_std
        pub enum #enum_ident #generics_w_bounds {
            #(#enum_variants,)*
            #unused_generics_variant
        }
        #maybe_impl_error
    }
}

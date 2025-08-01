use fuel_abi_types::abi::full_program::FullABIFunction;
use proc_macro2::TokenStream;
use quote::{ToTokens, quote};

use crate::{
    error::Result,
    program_bindings::{
        resolved_type::TypeResolver,
        utils::{Component, Components},
    },
    utils::{TypePath, safe_ident},
};

#[derive(Debug)]
pub(crate) struct FunctionGenerator {
    name: String,
    args: Components,
    output_type: TokenStream,
    body: TokenStream,
    docs: Vec<String>,
}

impl FunctionGenerator {
    pub fn new(fun: &FullABIFunction) -> Result<Self> {
        // All abi-method-calling Rust functions are currently generated at the top-level-mod of
        // the Program in question (e.g. abigen_bindings::my_contract_mod`). If we ever nest
        // these functions in a deeper mod we would need to propagate the mod to here instead of
        // just hard-coding the default path.
        let args = Components::new(fun.inputs(), true, TypePath::default())?;

        // We are not checking that the ABI contains non-SDK supported types so that the user can
        // still interact with an ABI even if some methods will fail at runtime.
        let output_type = TypeResolver::default().resolve(fun.output())?;
        Ok(Self {
            name: fun.name().to_string(),
            args,
            output_type: output_type.to_token_stream(),
            body: Default::default(),
            docs: vec![],
        })
    }

    pub fn set_name(&mut self, name: String) -> &mut Self {
        self.name = name;
        self
    }

    pub fn set_body(&mut self, body: TokenStream) -> &mut Self {
        self.body = body;
        self
    }

    pub fn set_docs(&mut self, docs: Vec<String>) -> &mut Self {
        self.docs = docs;
        self
    }

    pub fn fn_selector(&self) -> TokenStream {
        let name = &self.name;
        quote! {::fuels::core::codec::encode_fn_selector(#name)}
    }

    pub fn tokenized_args(&self) -> TokenStream {
        let arg_names = self.args.iter().map(|Component { ident, .. }| {
            quote! {#ident}
        });

        quote! {[#(::fuels::core::traits::Tokenizable::into_token(#arg_names)),*]}
    }

    pub fn set_output_type(&mut self, output_type: TokenStream) -> &mut Self {
        self.output_type = output_type;
        self
    }

    pub fn output_type(&self) -> &TokenStream {
        &self.output_type
    }

    pub fn generate(&self) -> TokenStream {
        let name = safe_ident(&self.name);
        let docs: Vec<TokenStream> = self
            .docs
            .iter()
            .map(|doc| {
                quote! { #[doc = #doc] }
            })
            .collect();

        let arg_declarations = self.args.iter().map(
            |Component {
                 ident,
                 resolved_type,
                 ..
             }| {
                quote! { #ident: #resolved_type }
            },
        );

        let output_type = self.output_type();
        let body = &self.body;

        let params = quote! { &self, #(#arg_declarations),* };

        quote! {
            #(#docs)*
            pub fn #name(#params) -> #output_type {
                #body
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use fuel_abi_types::abi::full_program::{FullTypeApplication, FullTypeDeclaration};
    use pretty_assertions::assert_eq;

    use super::*;

    #[test]
    fn correct_fn_selector_resolving_code() -> Result<()> {
        let function = given_a_fun();
        let sut = FunctionGenerator::new(&function)?;

        let fn_selector_code = sut.fn_selector();

        let expected = quote! {
            ::fuels::core::codec::encode_fn_selector("test_function")
        };
        assert_eq!(fn_selector_code.to_string(), expected.to_string());

        Ok(())
    }

    #[test]
    fn correct_tokenized_args() -> Result<()> {
        let function = given_a_fun();
        let sut = FunctionGenerator::new(&function)?;

        let tokenized_args = sut.tokenized_args();

        assert_eq!(
            tokenized_args.to_string(),
            "[:: fuels :: core :: traits :: Tokenizable :: into_token (arg_0)]"
        );

        Ok(())
    }

    #[test]
    fn tokenizes_correctly() -> Result<()> {
        // given
        let function = given_a_fun();
        let mut sut = FunctionGenerator::new(&function)?;

        sut.set_docs(vec![
            " This is a doc".to_string(),
            " This is another doc".to_string(),
        ])
        .set_body(quote! {this is ze body});

        // when
        let tokenized: TokenStream = sut.generate();

        // then
        let expected = quote! {
            #[doc = " This is a doc"]
            #[doc = " This is another doc"]
            pub fn test_function(&self, arg_0: self::CustomStruct<::core::primitive::u8>) -> self::CustomStruct<::core::primitive::u64> {
                this is ze body
            }
        };

        // then
        assert_eq!(tokenized.to_string(), expected.to_string());

        Ok(())
    }

    fn given_a_fun() -> FullABIFunction {
        let generic_type_t = FullTypeDeclaration {
            type_field: "generic T".to_string(),
            components: vec![],
            type_parameters: vec![],
            alias_of: None,
        };
        let custom_struct_type = FullTypeDeclaration {
            type_field: "struct CustomStruct".to_string(),
            components: vec![FullTypeApplication {
                name: "field_a".to_string(),
                type_decl: generic_type_t.clone(),
                type_arguments: vec![],
                error_message: None,
            }],
            type_parameters: vec![generic_type_t],
            alias_of: None,
        };

        let fn_output = FullTypeApplication {
            name: "".to_string(),
            type_decl: custom_struct_type.clone(),
            type_arguments: vec![FullTypeApplication {
                name: "".to_string(),
                type_decl: FullTypeDeclaration {
                    type_field: "u64".to_string(),
                    components: vec![],
                    type_parameters: vec![],
                    alias_of: None,
                },
                type_arguments: vec![],
                error_message: None,
            }],
            error_message: None,
        };
        let fn_inputs = vec![FullTypeApplication {
            name: "arg_0".to_string(),
            type_decl: custom_struct_type,
            type_arguments: vec![FullTypeApplication {
                name: "".to_string(),
                type_decl: FullTypeDeclaration {
                    type_field: "u8".to_string(),
                    components: vec![],
                    type_parameters: vec![],
                    alias_of: None,
                },
                type_arguments: vec![],
                error_message: None,
            }],
            error_message: None,
        }];

        FullABIFunction::new("test_function".to_string(), fn_inputs, fn_output, vec![])
            .expect("Hand crafted function known to be correct")
    }
}

use proc_macro::{self, TokenStream};
use proc_macro2 as pm2;
use quote::{quote, quote_spanned};
use syn::{
    parse::Parse, parse_macro_input, punctuated::Punctuated, DeriveInput,
};

/// An attribute of the form: `#[ctx(Type: add, get, mut)]`.
struct CtxAttr {
    typ: pm2::Ident,
    req_traits: Vec<pm2::Ident>,
}

impl Parse for CtxAttr {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let typ = input.parse()?;
        input.parse::<syn::Token![:]>()?;
        let req_traits =
            Punctuated::<syn::Ident, syn::Token![,]>::parse_separated_nonempty(
                input,
            )?;

        Ok(Self {
            typ,
            req_traits: req_traits.into_iter().collect(),
        })
    }
}

/// Define a derive macro for the `filament::ir::Ctx` and `filament::ir::MutCtx`
/// traits that delegates to particular fields.
///
/// The usage is as follows:
/// ```
/// #[derive(Ctx)]
/// struct Foo {
///     #[ctx(Port: Add, Get)]
///     foo: IndexStore<ir::Port>
///     #[ctx(Param: Get, Add, Mut)]
///     bar: IndexStore<ir::Param>
/// }
/// ```
///
/// Note that the fields must already implement the `Ctx`, `AddCtx`, and `MutCtx` traits
/// for the two attributes.  The macro does not implement the trait for the
/// structure itself, it simply delegates to the fields.
#[proc_macro_derive(Ctx, attributes(ctx))]
pub fn derive(input: TokenStream) -> TokenStream {
    let DeriveInput {
        ident: struct_name,
        data,
        ..
    } = parse_macro_input!(input);

    let output = match data {
        syn::Data::Struct(syn::DataStruct {
            fields: syn::Fields::Named(fields),
            ..
        }) => {
            let fields: Vec<pm2::TokenStream> = fields
                .named
                .iter()
                .flat_map(|field| {
                    for attr in &field.attrs {
                        if attr.path().is_ident("ctx") {
                            let ctx = match attr.parse_args::<CtxAttr>() {
                                Ok(ctx_attr) => ctx_attr,
                                Err(e) => {
                                    return Some(
                                        TokenStream::from(e.to_compile_error())
                                            .into(),
                                    );
                                }
                            };
                            return Some(generate_impls(
                                struct_name.clone(),
                                field.ident.clone().unwrap(),
                                ctx,
                            ));
                        }
                    }
                    None::<pm2::TokenStream>
                })
                .collect::<Vec<_>>();
            quote! {
                #(#fields)*
            }
        }
        _ => panic!("Derive Ctx only supported on structs"),
    };
    output.into()
}

/// Generate the implementation for the requested traits.
fn generate_impls(
    struct_name: pm2::Ident,
    field_name: pm2::Ident,
    ctxs: CtxAttr,
) -> pm2::TokenStream {
    let mut out = quote!();
    let typ = ctxs.typ;
    for trait_name in ctxs.req_traits {
        if trait_name == "Get" {
            let ctx = quote! {
                impl Ctx<#typ> for #struct_name {
                    fn get(&self, idx: Idx<#typ>) -> &#typ {
                        Ctx::get(&self.#field_name, idx)
                    }
                }
            };
            out.extend(ctx);
        } else if trait_name == "Add" {
            let add_ctx = quote! {
                impl AddCtx<#typ> for #struct_name {
                    fn add(&mut self, val: #typ) -> Idx<#typ> {
                        AddCtx::add(&mut self.#field_name, val)
                    }
                }
            };
            out.extend(add_ctx);
        } else if trait_name == "Mut" {
            let mut_ctx = quote! {
                impl MutCtx<#typ> for #struct_name {
                    fn get_mut(&mut self, idx: Idx<#typ>) -> &mut #typ {
                        MutCtx::get_mut(&mut self.#field_name, idx)
                    }
                    fn delete(&mut self, idx: Idx<#typ>) {
                        MutCtx::delete(&mut self.#field_name, idx)
                    }
                    fn valid(&self, idx: Idx<#typ>) -> bool {
                        MutCtx::valid(&self.#field_name, idx)
                    }
                }
            };
            out.extend(mut_ctx);
        } else {
            // Unknown trait name. Generate an error
            let err = quote_spanned! {
                trait_name.span() =>
                compile_error!("unexpected trait name");
            };
            out.extend(err);
        }
    }

    out
}

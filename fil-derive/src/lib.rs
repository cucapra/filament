use proc_macro::{self, TokenStream};
use quote::quote;
use syn::{parse_macro_input, punctuated::Punctuated, DeriveInput};

/// Define a derive macro for the `filament::ir::Ctx` and `filament::ir::MutCtx`
/// traits that delegates to particular fields.
///
/// The usage is as follows:
/// ```
/// #[derive(Ctx)]
/// struct Foo {
///     #[ctx(ir::Port)]
///     #[add_ctx(ir::Port)]
///     foo: IndexStore<ir::Port>
///     #[mut_ctx(ir::Param)]
///     bar: IndexStore<ir::Param>
/// }
/// ```
///
/// Note that the fields must already implement the `Ctx`, `AddCtx`, and `MutCtx` traits
/// for the two attributes.  The macro does not implement the trait for the
/// structure itself, it simply delegates to the fields.
#[proc_macro_derive(Ctx, attributes(ctx, add_ctx, mut_ctx))]
pub fn derive(input: TokenStream) -> TokenStream {
    let DeriveInput { ident, data, .. } = parse_macro_input!(input);

    let output = match data {
        syn::Data::Struct(syn::DataStruct {
            fields: syn::Fields::Named(fields),
            ..
        }) => {
            let fields = fields.named.iter().flat_map(|field| {
                let mut impl_ctx = None;
                let mut impl_add_ctx = None;
                let mut impl_mut_ctx = None;
                for attr in &field.attrs {
                    // We expect the attribute to look like ctx(Type) and extract the type
                    if attr.path().is_ident("ctx") {
                        let nested = attr.parse_args_with(Punctuated::<syn::Meta, syn::Token![,]>::parse_terminated).unwrap();
                        assert!(nested.len() == 1);
                        impl_ctx = Some(nested[0].path().get_ident().unwrap().clone());
                    }
                    if attr.path().is_ident("add_ctx") {
                        let nested = attr.parse_args_with(Punctuated::<syn::Meta, syn::Token![,]>::parse_terminated).unwrap();
                        assert!(nested.len() == 1);
                        impl_add_ctx = Some(nested[0].path().get_ident().unwrap().clone());
                    }
                    if attr.path().is_ident("mut_ctx") {
                        let nested = attr.parse_args_with(Punctuated::<syn::Meta, syn::Token![,]>::parse_terminated).unwrap();
                        assert!(nested.len() == 1);
                        impl_mut_ctx = Some(nested[0].path().get_ident().unwrap().clone());
                    }
                }
                if impl_ctx.is_none() && impl_mut_ctx.is_none() && impl_add_ctx.is_none() {
                    return None;
                }

                let name = &field.ident;
                let mut out = quote!();
                if let Some(typ) = impl_ctx {
                    let ctx = quote! {
                        impl Ctx<#typ> for #ident {
                            fn get(&self, idx: Idx<#typ>) -> &#typ {
                                Ctx::get(&self.#name, idx)
                            }
                        }
                    };
                    out.extend(ctx);
                }
                if let Some(typ) = impl_add_ctx {
                    let add_ctx = quote! {
                        impl AddCtx<#typ> for #ident {
                            fn add(&mut self, val: #typ) -> Idx<#typ> {
                                AddCtx::add(&mut self.#name, val)
                            }
                        }
                    };
                    out.extend(add_ctx);
                }
                if let Some(typ) = impl_mut_ctx {
                    let mut_ctx = quote! {
                        impl MutCtx<#typ> for #ident {
                            fn get_mut(&mut self, idx: Idx<#typ>) -> &mut #typ {
                                MutCtx::get_mut(&mut self.#name, idx)
                            }
                            fn delete(&mut self, idx: Idx<#typ>) {
                                MutCtx::delete(&mut self.#name, idx)
                            }
                        }
                    };
                    out.extend(mut_ctx);
                }
                Some(out)
            });
            quote! {
                #(#fields)*
            }
        }
        _ => panic!("Derive Ctx only supported on structs"),
    };
    output.into()
}

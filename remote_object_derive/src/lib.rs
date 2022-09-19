use std::{
    collections::{HashMap, HashSet},
    iter::FromIterator,
};

use proc_macro2::{Literal, Span, TokenStream};
use quote::quote;
use syn::{parse_macro_input, DataEnum, DataStruct, DeriveInput, Expr, Fields, Ident, Lit, Type};

#[proc_macro_derive(RemoteObject)]
pub fn derive_remote_object(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = input.ident;
    proc_macro::TokenStream::from(match input.data {
        syn::Data::Struct(data) => process_struct(name, data),
        syn::Data::Enum(data) => process_enum(name, data),
        syn::Data::Union(_) => unimplemented!("Unions are not supported"),
    })
}

fn process_enum(name: Ident, data: DataEnum) -> TokenStream {
    let mut mapping = HashMap::new();
    let mut taken = HashSet::<isize>::new();
    for var in data.variants.iter() {
        if let Some((_, disc)) = var.discriminant.as_ref() {
            if let Expr::Lit(lit) = disc {
                if let Lit::Int(ref intlit) = lit.lit {
                    let parsed = intlit
                        .base10_parse()
                        .expect("Expected valid integer literal");
                    mapping.insert(
                        var.ident.clone(),
                        (parsed, fields_to_idents_and_types(&var.fields)),
                    );
                    taken.insert(parsed);
                } else {
                    panic!("Non-integer discriminants are not supported");
                }
            } else {
                panic!("Non-literal discriminants are not supported");
            }
        }
    }
    let vacant = HashSet::from_iter(0..data.variants.len() as isize);
    let mut vacant = vacant.difference(&taken);
    for var in data.variants {
        if var.discriminant.is_none() {
            mapping.insert(
                var.ident.clone(),
                (
                    *vacant.next().unwrap(),
                    fields_to_idents_and_types(&var.fields),
                ),
            );
        }
    }
    let patterns_push = mapping.iter().map(|(ident, (discr, fields))| {
        if let Some((fields, _)) = fields {
            quote! {
                #ident (#(#fields),*) => {
                    #discr.push(channel)?;
                    #(#fields.push(channel)?);*
                }
            }
        } else {
            quote! {
                #ident => {
                    #discr.push(channel)?;
                }
            }
        }
    });
    let patterns_pull = mapping.iter().map(|(ident, (discr, fields))| {
        if let Some((fields, types)) = fields {
            quote! {
                #discr => {
                    #(
                        let mut #fields = #types ::default();
                        #fields.pull(channel)?;
                    )*
                    #ident (#(#fields),*)
                }
            }
        } else {
            quote! {
                #discr => { #ident }
            }
        }
    });

    TokenStream::from(quote! {
        #[automatically_derived]
        impl remote_object::RemoteObject for #name {
            fn pull<C: std::io::Read>(&mut self, channel: &mut C) -> remote_object::SyncResult<()> {
                let mut discr = 0isize;
                discr.pull(channel)?;
                *self = match discr {
                    #(#patterns_pull),*
                    _ => unreachable!()
                };
                Ok(())
            }
            fn push<C: std::io::Write>(&self, channel: &mut C) -> remote_object::SyncResult<()> {
                match self {
                    #(#patterns_push),*
                    _ => unreachable!()
                }
                Ok(())
            }
        }
    })
}

fn fields_to_idents_and_types(fields: &Fields) -> Option<(Vec<Ident>, Vec<Type>)> {
    if let Fields::Unnamed(unnamed) = fields {
        Some(
            unnamed
                .unnamed
                .iter()
                .enumerate()
                .map(|(idx, field)| {
                    (
                        Ident::new(&format!("field_{idx}"), Span::call_site()),
                        field.ty.clone(),
                    )
                })
                .unzip(),
        )
    } else if let Fields::Unit = fields {
        None
    } else {
        panic!("Named fields in enum variants are not supported",);
    }
}

fn process_struct(name: Ident, data: DataStruct) -> TokenStream {
    let (pull_exprs, push_exprs): (Vec<_>, Vec<_>) = match data.fields {
        syn::Fields::Named(fields) => fields
            .named
            .iter()
            .map(|f| f.ident.clone().unwrap())
            .map(|field| {
                (
                    quote! {
                        self. #field .pull(channel)?;
                    },
                    quote! {
                        self. #field .push(channel)?;
                    },
                )
            })
            .unzip(),
        syn::Fields::Unnamed(fields) => {
            let number = fields.unnamed.len();
            (0..number)
                .map(|n| Literal::usize_unsuffixed(n))
                .map(|number| {
                    (
                        quote! {
                            self. #number .pull(channel)?;
                        },
                        quote! {
                            self. #number .push(channel)?;
                        },
                    )
                })
                .unzip()
        }
        syn::Fields::Unit => return TokenStream::new(),
    };
    TokenStream::from(quote! {
        #[automatically_derived]
        impl remote_object::RemoteObject for #name {
            fn pull<C: std::io::Read>(&mut self, channel: &mut C) -> remote_object::SyncResult<()> {
                #(#pull_exprs)*
                Ok(())
            }
            fn push<C: std::io::Write>(&self, channel: &mut C) -> remote_object::SyncResult<()> {
                #(#push_exprs)*
                Ok(())
            }
        }
    })
}

use proc_macro::TokenStream;
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::{format_ident, quote};
use syn::{Attribute, Expr, Field, Fields, Ident, ItemStruct, Meta, Type, parse_macro_input};

/// Defines the state of an editor from plugins and plain fields
///
/// ```
/// use kwark::plugins;
///
/// #[kwark::state]
/// pub struct MyState {
///     #[default(true)]
///     local_variable: bool,
///     #[plugin]
///     visual: plugins::visual::Plugin,
///     #[plugin(plugins::smooth_scroll::Plugin::new(40.0))]
///     smooth_scroll: plugins::smooth_scroll::Plugin,
/// }
///
/// let editor = kwark::init::<MyState>().unwrap();
/// assert!(editor.state.local_variable);
/// ```
#[proc_macro_attribute]
pub fn state(args: TokenStream, item: TokenStream) -> TokenStream {
    let args = TokenStream2::from(args);
    if !args.is_empty() {
        return syn::Error::new_spanned(args, "`#[kwark::state]` takes no arguments")
            .to_compile_error()
            .into();
    }

    let item = parse_macro_input!(item as ItemStruct);
    expand(item)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

enum Kind {
    Plugin(Option<Expr>),
    Value(Option<Expr>),
}

struct StateField {
    ident: Ident,
    ty: Type,
    kind: Kind,
}

fn expand(mut item: ItemStruct) -> syn::Result<TokenStream2> {
    if !item.generics.params.is_empty() {
        return Err(syn::Error::new_spanned(
            &item.generics,
            "`#[kwark::state]` does not support generics",
        ));
    }

    let mut fields = vec![];
    match &mut item.fields {
        Fields::Named(named) => {
            for field in named.named.iter_mut() {
                fields.push(take_field(field)?);
            }
        }
        Fields::Unit => item.fields = Fields::Named(syn::parse_quote!({})),
        Fields::Unnamed(_) => {
            return Err(syn::Error::new_spanned(
                &item.fields,
                "`#[kwark::state]` requires named fields",
            ));
        }
    }

    let name = &item.ident;

    let has_impls = fields.iter().filter_map(|f| {
        let Kind::Plugin(_) = f.kind else {
            return None;
        };
        let (ident, plugin) = (&f.ident, &f.ty);

        Some(quote! {
            impl ::kwark::Has<#plugin> for #name {
                fn plugin(&self) -> &<#plugin as ::kwark::PluginState>::State {
                    &self.#ident
                }

                fn plugin_mut(&mut self) -> &mut <#plugin as ::kwark::PluginState>::State {
                    &mut self.#ident
                }
            }
        })
    });

    let senders = Ident::new("__kwark_senders", Span::mixed_site());
    let editor = Ident::new("__kwark_editor", Span::mixed_site());

    let mut plugins = vec![];
    let mut values = vec![];
    let mut registers = vec![];

    for f in &fields {
        let (ident, ty) = (&f.ident, &f.ty);

        match &f.kind {
            Kind::Plugin(init) => {
                let var = format_ident!("__kwark_plugin_{}", ident, span = Span::mixed_site());
                let init = match init {
                    Some(expr) => quote!(#expr),
                    None => quote!(<#ty as ::core::default::Default>::default()),
                };

                plugins.push(quote!(let #var: #ty = #init;));
                values.push(quote!(#ident: ::kwark::PluginState::init(&#var, #senders)?,));
                registers.push(quote!(::kwark::Plugin::<Self>::register(&#var, #editor)?;));
            }
            Kind::Value(init) => {
                let init = match init {
                    Some(expr) => quote!(#expr),
                    None => quote!(<#ty as ::core::default::Default>::default()),
                };

                values.push(quote!(#ident: #init,));
            }
        }
    }

    Ok(quote! {
        #item

        #(#has_impls)*

        impl ::kwark::State for #name {
            fn editor() -> ::kwark::anyhow::Result<::kwark::Editor<Self>> {
                #(#plugins)*

                ::kwark::Editor::build(
                    |#senders| {
                        ::core::result::Result::Ok(Self {
                            #(#values)*
                        })
                    },
                    |#editor| {
                        #(#registers)*
                        ::core::result::Result::Ok(())
                    },
                )
            }
        }
    })
}

/// Returns the kind of a field and rewrites its type to the stored type
fn take_field(field: &mut Field) -> syn::Result<StateField> {
    let mut plugin = None;
    let mut default = None;
    let mut kept = vec![];

    for attr in field.attrs.drain(..) {
        if attr.path().is_ident("plugin") {
            if plugin.is_some() {
                return Err(syn::Error::new_spanned(attr, "duplicate `#[plugin]`"));
            }
            plugin = Some(optional_expr(&attr)?);
        } else if attr.path().is_ident("default") {
            if default.is_some() {
                return Err(syn::Error::new_spanned(attr, "duplicate `#[default]`"));
            }
            default = Some((required_expr(&attr)?, attr));
        } else {
            kept.push(attr);
        }
    }
    field.attrs = kept;

    let ident = field.ident.clone().expect("named field");
    let ty = field.ty.clone();

    let kind = match (plugin, default) {
        (Some(_), Some((_, attr))) => {
            return Err(syn::Error::new_spanned(
                attr,
                "plugin fields take their value as `#[plugin(value)]`",
            ));
        }
        (Some(init), None) => {
            field.ty = syn::parse_quote!(<#ty as ::kwark::PluginState>::State);
            Kind::Plugin(init)
        }
        (None, default) => Kind::Value(default.map(|(expr, _)| expr)),
    };

    Ok(StateField { ident, ty, kind })
}

fn optional_expr(attr: &Attribute) -> syn::Result<Option<Expr>> {
    match &attr.meta {
        Meta::Path(_) => Ok(None),
        Meta::List(_) => attr.parse_args().map(Some),
        Meta::NameValue(_) => Err(syn::Error::new_spanned(
            attr,
            "expected `#[plugin]` or `#[plugin(value)]`",
        )),
    }
}

fn required_expr(attr: &Attribute) -> syn::Result<Expr> {
    match &attr.meta {
        Meta::List(_) => attr.parse_args(),
        _ => Err(syn::Error::new_spanned(attr, "expected `#[default(value)]`")),
    }
}

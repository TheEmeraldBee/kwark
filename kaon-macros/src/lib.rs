use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use syn::{
    Attribute, Expr as SynExpr, ExprLit, FnArg, Ident, Item, ItemMod, ItemType, Lit, Meta, Pat,
    Result, Type, parse_macro_input,
};

struct ModuleInput {
    cx_ty: Type,
    items: Vec<Item>,
}

impl ModuleInput {
    fn from_mod(item_mod: ItemMod) -> syn::Result<Self> {
        let Some((_, mut items)) = item_mod.content else {
            return Err(syn::Error::new_spanned(
                item_mod,
                "#[kaon::module] requires a `mod name { .. }` with a body",
            ));
        };

        let cx_pos = items
            .iter()
            .position(|item| matches!(item, Item::Type(t) if t.ident == "Cx"));

        let Some(pos) = cx_pos else {
            return Err(syn::Error::new_spanned(
                &item_mod.ident,
                "#[kaon::module] requires a leading `type Cx = YourStateType;`",
            ));
        };

        let Item::Type(ItemType { ty, .. }) = items.remove(pos) else {
            unreachable!()
        };

        Ok(ModuleInput { cx_ty: *ty, items })
    }
}

struct Leaf {
    path: Vec<String>,
    func: syn::ItemFn,
}

fn collect(items: &[Item], prefix: &mut Vec<String>, out: &mut Vec<Leaf>) -> Result<()> {
    for item in items {
        match item {
            Item::Fn(func) => {
                let mut path = prefix.clone();
                path.push(func.sig.ident.to_string());
                out.push(Leaf {
                    path,
                    func: func.clone(),
                });
            }
            Item::Mod(m) => {
                let Some((_, content)) = &m.content else {
                    return Err(syn::Error::new_spanned(
                        m,
                        "kaon::module! requires `mod name { .. }`",
                    ));
                };
                prefix.push(m.ident.to_string());
                collect(content, prefix, out)?;
                prefix.pop();
            }
            other => {
                return Err(syn::Error::new_spanned(
                    other,
                    "kaon::module! only accepts `fn` and `mod`",
                ));
            }
        }
    }
    Ok(())
}

fn doc_string(attrs: &[Attribute]) -> String {
    let mut lines = Vec::new();
    for attr in attrs {
        if !attr.path().is_ident("doc") {
            continue;
        }
        if let Meta::NameValue(nv) = &attr.meta
            && let SynExpr::Lit(ExprLit {
                lit: Lit::Str(s), ..
            }) = &nv.value
        {
            lines.push(s.value().trim().to_string());
        }
    }
    lines.join("\n")
}

fn has_variadic_attr(attrs: &[Attribute]) -> bool {
    attrs.iter().any(|a| a.path().is_ident("variadic"))
}

fn type_marker(ty: &Type) -> Option<&'static str> {
    let Type::Path(p) = ty else { return None };
    let seg = p.path.segments.last()?;
    Some(match seg.ident.to_string().as_str() {
        "Str" => "Str",
        "Int" => "Int",
        "Float" => "Float",
        "Bool" => "Bool",
        "List" => "List",
        "Method" => "Method",
        "Value" => "Value",
        _ => return None,
    })
}

fn extractor_for(marker: &str) -> (&'static str, proc_macro2::TokenStream) {
    match marker {
        "Str" => ("str", quote! { Some(::kaon::value::Type::Str) }),
        "Int" => ("int", quote! { Some(::kaon::value::Type::Int) }),
        "Float" => ("float", quote! { Some(::kaon::value::Type::Float) }),
        "Bool" => ("bool", quote! { Some(::kaon::value::Type::Bool) }),
        "List" => ("list", quote! { Some(::kaon::value::Type::List) }),
        "Method" => ("method", quote! { Some(::kaon::value::Type::Method) }),
        "Value" => ("value", quote! { None }),
        _ => unreachable!(),
    }
}

fn build_registration(leaf: &Leaf) -> Result<proc_macro2::TokenStream> {
    let name_lit = leaf.path.join("::");
    let desc = doc_string(&leaf.func.attrs);
    let desc_call = (!desc.is_empty()).then(|| quote! { .desc(#desc) });

    let mut arg_calls = Vec::new();
    let mut lets = Vec::new();
    let mut cx_let = None;
    let mut variadic_call = quote! {};

    for (i, arg) in leaf.func.sig.inputs.iter().enumerate() {
        let FnArg::Typed(pat_ty) = arg else {
            return Err(syn::Error::new_spanned(
                arg,
                "kaon::module! functions cannot take `self`",
            ));
        };
        let Pat::Ident(pat_ident) = &*pat_ty.pat else {
            return Err(syn::Error::new_spanned(
                &pat_ty.pat,
                "kaon::module! function arguments must be simple identifiers",
            ));
        };
        let name = pat_ident.ident.to_string();

        if i == 0 && name == "cx" {
            cx_let = Some(quote! { let cx = args.cx(); });
            continue;
        }

        let arg_desc = doc_string(&pat_ty.attrs);
        let ident = Ident::new(&name, pat_ident.ident.span());

        if has_variadic_attr(&pat_ty.attrs) {
            variadic_call = quote! { .variadic(#name, #arg_desc, Some(::kaon::value::Type::List)) };
            lets.push(quote! { let #ident = args.list(#name)?; });
            continue;
        }

        let marker = type_marker(&pat_ty.ty).ok_or_else(|| {
            syn::Error::new_spanned(
                &pat_ty.ty,
                "unsupported argument type: expected Str, Int, Float, Bool, List, Method, or Value",
            )
        })?;
        let (extractor, ty_tokens) = extractor_for(marker);
        let extractor_ident = Ident::new(extractor, Span::call_site());

        arg_calls.push(quote! { .arg(#name, #arg_desc, #ty_tokens) });
        lets.push(quote! { let #ident = args.#extractor_ident(#name)?; });
    }

    lets.extend(cx_let);

    let block = &leaf.func.block;

    Ok(quote! {
        engine.register(#name_lit, ::kaon::engine::FunctionBuilder::new()
            #desc_call
            #(#arg_calls)*
            #variadic_call
            .build(|args: &mut ::kaon::engine::Args<'_, Cx>| {
                #(#lets)*
                #block
            })
        );
    })
}

/// Expands nested `mod`/`fn` items into an `Engine::register` function
#[proc_macro_attribute]
pub fn module(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let item_mod = parse_macro_input!(item as ItemMod);
    let parsed = match ModuleInput::from_mod(item_mod) {
        Ok(parsed) => parsed,
        Err(e) => return e.to_compile_error().into(),
    };

    let mut leaves = Vec::new();
    if let Err(e) = collect(&parsed.items, &mut Vec::new(), &mut leaves) {
        return e.to_compile_error().into();
    }

    let mut registrations = Vec::new();
    for leaf in &leaves {
        match build_registration(leaf) {
            Ok(ts) => registrations.push(ts),
            Err(e) => return e.to_compile_error().into(),
        }
    }

    let cx_ty = &parsed.cx_ty;

    let expanded = quote! {
        pub fn register(engine: &mut ::kaon::engine::Engine<#cx_ty>) {
            type Cx = #cx_ty;
            #(#registrations)*
        }
    };

    expanded.into()
}

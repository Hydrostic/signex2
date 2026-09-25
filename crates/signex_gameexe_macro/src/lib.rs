use proc_macro::TokenStream;
use quote::quote;
use std::collections::HashSet;
use syn::{Expr, ExprArray, ExprLit, Fields, ItemStruct, Lit, Type, parse_macro_input};

#[derive(Default)]
struct ArrayOptions {
    count: Option<String>,
    value: bool,
    range: bool,
    index_by_capacity: bool,
    default_count: Option<Expr>,
    count_min: Option<Expr>,
    count_max: Option<Expr>,
    item_default: Option<Expr>,
}

#[derive(Default)]
struct Options {
    name: Option<String>,
    flatten: bool,
    value: bool,
    update: bool,
    parse_with: Option<Expr>,
    default: Option<Expr>,
    default_with: Option<Expr>,
    min: Option<Expr>,
    max: Option<Expr>,
    bounds: Option<ExprArray>,
    validate_with: Option<Expr>,
    validate_with_context: Option<Expr>,
    array: Option<ArrayOptions>,
}

fn options(field: &mut syn::Field) -> syn::Result<Options> {
    let mut result = Options::default();
    let mut kept = Vec::new();
    for attr in std::mem::take(&mut field.attrs) {
        if !attr.path().is_ident("gameexe") {
            kept.push(attr);
            continue;
        }
        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("name") {
                let value: syn::LitStr = meta.value()?.parse()?;
                result.name = Some(value.value());
            } else if meta.path.is_ident("flatten") {
                result.flatten = true;
            } else if meta.path.is_ident("value") {
                result.value = true;
            } else if meta.path.is_ident("update") {
                result.update = true;
            } else if meta.path.is_ident("parse_with") {
                result.parse_with = Some(meta.value()?.parse()?);
            } else if meta.path.is_ident("default") {
                result.default = Some(meta.value()?.parse()?);
            } else if meta.path.is_ident("default_with") {
                result.default_with = Some(meta.value()?.parse()?);
            } else if meta.path.is_ident("min") {
                result.min = Some(meta.value()?.parse()?);
            } else if meta.path.is_ident("max") {
                result.max = Some(meta.value()?.parse()?);
            } else if meta.path.is_ident("bounds") {
                result.bounds = Some(meta.value()?.parse()?);
            } else if meta.path.is_ident("validate_with") {
                result.validate_with = Some(meta.value()?.parse()?);
            } else if meta.path.is_ident("validate_with_context") {
                result.validate_with_context = Some(meta.value()?.parse()?);
            } else if meta.path.is_ident("array") {
                let mut array = ArrayOptions::default();
                meta.parse_nested_meta(|nested| {
                    if nested.path.is_ident("count") {
                        let value: syn::LitStr = nested.value()?.parse()?;
                        array.count = Some(value.value());
                    } else if nested.path.is_ident("range") {
                        let value: syn::LitBool = nested.value()?.parse()?;
                        array.range = value.value();
                    } else if nested.path.is_ident("value") {
                        let value: syn::LitBool = nested.value()?.parse()?;
                        array.value = value.value();
                    } else if nested.path.is_ident("index_by_capacity") {
                        let value: syn::LitBool = nested.value()?.parse()?;
                        array.index_by_capacity = value.value();
                    } else if nested.path.is_ident("default_count") {
                        array.default_count = Some(nested.value()?.parse()?);
                    } else if nested.path.is_ident("count_min") {
                        array.count_min = Some(nested.value()?.parse()?);
                    } else if nested.path.is_ident("count_max") {
                        array.count_max = Some(nested.value()?.parse()?);
                    } else if nested.path.is_ident("item_default") {
                        array.item_default = Some(nested.value()?.parse()?);
                    } else {
                        return Err(nested.error("unknown gameexe array option"));
                    }
                    Ok(())
                })?;
                result.array = Some(array);
            } else {
                return Err(meta.error("unknown gameexe option"));
            }
            Ok(())
        })?;
    }
    field.attrs = kept;
    Ok(result)
}

fn array_element(ty: &Type) -> Option<&Type> {
    let Type::Path(path) = ty else { return None };
    let segment = path.path.segments.last()?;
    if segment.ident != "GArray" {
        return None;
    }
    let syn::PathArguments::AngleBracketed(args) = &segment.arguments else {
        return None;
    };
    let syn::GenericArgument::Type(inner) = args.args.first()? else {
        return None;
    };
    Some(inner)
}

fn is_value(ty: &Type) -> bool {
    match ty {
        Type::Tuple(_) => true,
        Type::Path(path) => path.path.segments.last().is_some_and(|segment| {
            matches!(
                segment.ident.to_string().as_str(),
                "i32" | "u32" | "bool" | "String" | "Vec"
            )
        }),
        _ => false,
    }
}

fn literal_int(expr: &Expr) -> Option<i64> {
    let Expr::Lit(ExprLit {
        lit: Lit::Int(value),
        ..
    }) = expr
    else {
        return None;
    };
    value.base10_parse().ok()
}

fn expand(
    mut item: ItemStruct,
    value_field: Option<String>,
) -> syn::Result<proc_macro2::TokenStream> {
    if !item.generics.params.is_empty() {
        return Err(syn::Error::new_spanned(
            &item.generics,
            "generic #[gameexe] structs are not supported",
        ));
    }
    let Fields::Named(fields) = &mut item.fields else {
        return Err(syn::Error::new_spanned(
            &item,
            "#[gameexe] requires named fields",
        ));
    };
    let mut defaults = Vec::new();
    let mut arms = Vec::new();
    let mut self_value = None;
    let mut fallback = None;
    let mut names = HashSet::new();
    for field in &mut fields.named {
        let cfg = options(field)?;
        let ident = field.ident.as_ref().unwrap();
        let name = cfg
            .name
            .unwrap_or_else(|| ident.to_string().to_ascii_uppercase());
        if !names.insert(name.to_ascii_uppercase()) {
            return Err(syn::Error::new_spanned(
                &*field,
                "duplicate gameexe field name",
            ));
        }
        let ty = &field.ty;
        if value_field.as_deref() == Some(&ident.to_string()) {
            if !is_value(ty) && !cfg.value {
                return Err(syn::Error::new_spanned(
                    &*field,
                    "value_field must name a value field",
                ));
            }
            let validate = cfg.validate_with.as_ref().map(|validator| quote! {
                #validator(&parsed).map_err(|message| ::signex_asset::gameexe::GameexeError::new(span.clone(), stringify!(#ident), message))?;
            });
            self_value = Some(quote! {
                if path.is_empty() {
                    let parsed = <#ty as ::signex_asset::gameexe::GameexeValue>::parse_value(value, stringify!(#ident))?;
                    #validate
                    self.#ident = parsed;
                    return Ok(());
                }
            });
        }
        if cfg.default.is_some() && cfg.default_with.is_some() {
            return Err(syn::Error::new_spanned(
                &*field,
                "use either default or default_with",
            ));
        }
        if let Some(inner) = array_element(ty) {
            let Some(array) = cfg.array else {
                return Err(syn::Error::new_spanned(
                    &*field,
                    "GArray requires #[gameexe(array(...))]",
                ));
            };
            let count = array.count.unwrap_or_else(|| "CNT".to_owned());
            let count_enabled = !count.is_empty();
            let allow_range = array.range;
            let index_by_capacity = array.index_by_capacity;
            let item_is_value = is_value(inner) || array.value;
            let default_count = array
                .default_count
                .ok_or_else(|| syn::Error::new_spanned(&*field, "array requires default_count"))?;
            let count_min = array
                .count_min
                .ok_or_else(|| syn::Error::new_spanned(&*field, "array requires count_min"))?;
            let count_max = array
                .count_max
                .ok_or_else(|| syn::Error::new_spanned(&*field, "array requires count_max"))?;
            if let (Some(default), Some(min), Some(max)) = (
                literal_int(&default_count),
                literal_int(&count_min),
                literal_int(&count_max),
            ) {
                if min < 0 || min > default || default > max {
                    return Err(syn::Error::new_spanned(
                        &*field,
                        "invalid array count bounds/default",
                    ));
                }
            }
            let make_item = if let Some(expr) = array.item_default {
                quote!(#expr)
            } else if item_is_value {
                quote!(|_| ::core::default::Default::default())
            } else {
                quote!(|_| <#inner as ::signex_asset::gameexe::GameexeNode>::gameexe_default())
            };
            defaults.push(quote!(#ident: ::signex_asset::gameexe::GArray::new(#default_count, #count_min, #count_max, #make_item)));
            let apply_array = if item_is_value {
                quote!(apply_value)
            } else if index_by_capacity {
                quote!(apply_capacity)
            } else {
                quote!(apply)
            };
            if cfg.flatten {
                if fallback.is_some() {
                    return Err(syn::Error::new_spanned(
                        &*field,
                        "only one flattened array is supported",
                    ));
                }
                fallback = Some(quote! {
                    if path.first().is_some_and(|part| part.contains('-')) && !#allow_range {
                        return Err(::signex_asset::gameexe::GameexeError::new(span, head, "range key is not supported for this array"));
                    }
                    self.#ident.#apply_array(path, value, span)
                });
            }
            if !cfg.flatten {
                arms.push(quote! {
                #name => {
                    if !#count_enabled && tail.first().is_some_and(|part| part.eq_ignore_ascii_case("CNT")) {
                        return Err(::signex_asset::gameexe::GameexeError::new(span, #name, "CNT is not supported for this array"));
                    }
                    if tail.first().is_some_and(|part| part.contains('-')) && !#allow_range {
                        return Err(::signex_asset::gameexe::GameexeError::new(span, #name, "range key is not supported for this array"));
                    }
                    if #count_enabled && tail.len() == 1 && tail[0].eq_ignore_ascii_case(#count) {
                        let number = <i32 as ::signex_asset::gameexe::GameexeValue>::parse_value(value, #name)?;
                        self.#ident.set_count(number, span, #name)
                    } else {
                        self.#ident.#apply_array(tail, value, span)
                    }
                }
            });
            }
            continue;
        }
        if cfg.array.is_some() {
            return Err(syn::Error::new_spanned(
                &*field,
                "array option requires GArray<T>",
            ));
        }
        if let (Some(default), Some(min)) = (
            cfg.default.as_ref().and_then(literal_int),
            cfg.min.as_ref().and_then(literal_int),
        ) {
            if default < min {
                return Err(syn::Error::new_spanned(&*field, "default below minimum"));
            }
        }
        if let (Some(default), Some(max)) = (
            cfg.default.as_ref().and_then(literal_int),
            cfg.max.as_ref().and_then(literal_int),
        ) {
            if default > max {
                return Err(syn::Error::new_spanned(&*field, "default above maximum"));
            }
        }
        let field_is_value = is_value(ty) || cfg.value;
        let default = if let Some(expr) = cfg.default {
            quote!(#expr)
        } else if let Some(expr) = cfg.default_with {
            quote!(#expr())
        } else if field_is_value {
            quote!(::core::default::Default::default())
        } else {
            quote!(<#ty as ::signex_asset::gameexe::GameexeNode>::gameexe_default())
        };
        defaults.push(quote!(#ident: #default));
        if field_is_value {
            if cfg.update {
                if cfg.min.is_some()
                    || cfg.max.is_some()
                    || cfg.bounds.is_some()
                    || cfg.validate_with.is_some()
                    || cfg.parse_with.is_some()
                {
                    return Err(syn::Error::new_spanned(
                        &*field,
                        "update cannot be combined with value checks or parse_with",
                    ));
                }
                arms.push(quote! {
                    #name => {
                        if !tail.is_empty() { return Err(::signex_asset::gameexe::GameexeError::new(span, #name, "unexpected path suffix")); }
                        <#ty as ::signex_asset::gameexe::GameexeValue>::update_value(&mut self.#ident, value, #name)
                    }
                });
                continue;
            }
            let mut checks = Vec::new();
            if cfg.bounds.is_some() && (cfg.min.is_some() || cfg.max.is_some()) {
                return Err(syn::Error::new_spanned(
                    &*field,
                    "combine tuple bounds only with validate_with",
                ));
            }
            if let Some(bounds) = cfg.bounds {
                let Type::Tuple(tuple_ty) = ty else {
                    return Err(syn::Error::new_spanned(
                        &*field,
                        "bounds requires tuple field",
                    ));
                };
                if bounds.elems.len() != tuple_ty.elems.len() {
                    return Err(syn::Error::new_spanned(
                        &*field,
                        "tuple bounds arity mismatch",
                    ));
                }
                for (index, pair) in bounds.elems.iter().enumerate() {
                    let Expr::Tuple(pair) = pair else {
                        return Err(syn::Error::new_spanned(pair, "expected (min, max)"));
                    };
                    if pair.elems.len() != 2 {
                        return Err(syn::Error::new_spanned(pair, "expected (min, max)"));
                    }
                    let min = &pair.elems[0];
                    let max = &pair.elems[1];
                    let index = syn::Index::from(index);
                    checks.push(quote! {
                        if parsed.#index < (#min) || parsed.#index > (#max) {
                            return Err(::signex_asset::gameexe::GameexeError::new(span.clone(), #name, concat!("tuple element ", stringify!(#index), " outside bounds")));
                        }
                    });
                }
            }
            if let Some(min) = cfg.min {
                checks.push(quote!(if parsed < (#min) { return Err(::signex_asset::gameexe::GameexeError::new(span.clone(), #name, "value below minimum")); }));
            }
            if let Some(max) = cfg.max {
                checks.push(quote!(if parsed > (#max) { return Err(::signex_asset::gameexe::GameexeError::new(span.clone(), #name, "value above maximum")); }));
            }
            if let Some(validator) = cfg.validate_with {
                checks.push(quote!(#validator(&parsed).map_err(|message| ::signex_asset::gameexe::GameexeError::new(span.clone(), #name, message))?;));
            }
            if let Some(validator) = cfg.validate_with_context {
                checks.push(quote!(#validator(self, &parsed).map_err(|message| ::signex_asset::gameexe::GameexeError::new(span.clone(), #name, message))?;));
            }
            let parse_value = if let Some(parser) = cfg.parse_with {
                quote!(#parser(value, #name)?)
            } else {
                quote!(<#ty as ::signex_asset::gameexe::GameexeValue>::parse_value(value, #name)?)
            };
            arms.push(quote! {
                #name => {
                    if !tail.is_empty() { return Err(::signex_asset::gameexe::GameexeError::new(span, #name, "unexpected path suffix")); }
                    let parsed = #parse_value;
                    #(#checks)*
                    self.#ident = parsed;
                    Ok(())
                }
            });
        } else {
            if cfg.min.is_some()
                || cfg.max.is_some()
                || cfg.bounds.is_some()
                || cfg.validate_with.is_some()
            {
                return Err(syn::Error::new_spanned(
                    &*field,
                    "value checks require scalar or tuple field",
                ));
            }
            arms.push(quote!(#name => self.#ident.apply(tail, value, span)));
        }
    }
    if value_field.is_some() && self_value.is_none() {
        return Err(syn::Error::new_spanned(
            &item,
            "value_field does not name a field",
        ));
    }
    let ident = &item.ident;
    let fallback = fallback.unwrap_or_else(|| {
        quote!(Err(::signex_asset::gameexe::GameexeError::new(
            span,
            head,
            "unknown field"
        )))
    });
    Ok(quote! {
        #item
        impl ::signex_asset::gameexe::GameexeNode for #ident {
            fn gameexe_default() -> Self { Self { #(#defaults,)* } }
            fn apply(&mut self, path: &[&str], value: &[::signex_asset::gameexe::SpannedToken<'_>], span: ::signex_asset::gameexe::Span) -> Result<(), ::signex_asset::gameexe::GameexeError> {
                #self_value
                let Some((&head, tail)) = path.split_first() else {
                    return Err(::signex_asset::gameexe::GameexeError::new(span, stringify!(#ident), "expected field name"));
                };
                let upper = head.to_ascii_uppercase();
                match upper.as_str() {
                    #(#arms,)*
                    _ => { #fallback },
                }
            }
        }
    })
}

#[proc_macro_attribute]
pub fn gameexe(args: TokenStream, input: TokenStream) -> TokenStream {
    let value_field = if args.is_empty() {
        None
    } else {
        let option = parse_macro_input!(args as syn::MetaNameValue);
        if !option.path.is_ident("value_field") {
            return syn::Error::new_spanned(option, "expected value_field = \"field_name\"")
                .to_compile_error()
                .into();
        }
        let Expr::Lit(ExprLit {
            lit: Lit::Str(value),
            ..
        }) = option.value
        else {
            return syn::Error::new_spanned(option.value, "value_field must be a string")
                .to_compile_error()
                .into();
        };
        Some(value.value())
    };
    let item = parse_macro_input!(input as ItemStruct);
    match expand(item, value_field) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

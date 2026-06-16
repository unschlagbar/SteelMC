use proc_macro2::TokenStream;
use quote::quote;
use syn::{Fields, ItemStruct, parse2};

/// Attribute macro for block behavior structs.
///
/// Strips `#[json_arg(...)]` field attributes (which are only read by the build script
/// scanning source files) and passes the struct through unchanged.
pub fn block_behavior(_attr: TokenStream, item: TokenStream) -> TokenStream {
    strip_json_arg_attrs(item, "block_behavior")
}

/// Attribute macro for item behavior structs.
///
/// Strips `#[json_arg(...)]` field attributes (which are only read by the build script
/// scanning source files) and passes the struct through unchanged.
pub fn item_behavior(_attr: TokenStream, item: TokenStream) -> TokenStream {
    strip_json_arg_attrs(item, "item_behavior")
}

/// Attribute macro for entity behavior structs.
///
/// Strips `#[json_arg(...)]` field attributes and emits an `EntityIdentifier` impl using
/// the `class = "..."` value so the struct can be identified at runtime by its string id.
pub fn entity_behavior(attr: TokenStream, item: TokenStream) -> TokenStream {
    let mut class_name: Option<syn::LitStr> = None;
    let mut identifier: Option<syn::LitStr> = None;
    let parser = syn::meta::parser(|meta| {
        if meta.path.is_ident("class") {
            class_name = Some(meta.value()?.parse()?);
            Ok(())
        } else if meta.path.is_ident("identifier") {
            identifier = Some(meta.value()?.parse()?);
            Ok(())
        } else if let Ok(value) = meta.value() {
            // Consume and ignore any other value-bearing keys (e.g. json_arg-related).
            let _: syn::Lit = value.parse()?;
            Ok(())
        } else {
            Ok(())
        }
    });
    syn::parse::Parser::parse2(parser, attr)
        .unwrap_or_else(|e| panic!("#[entity_behavior]: {e}"));
    let class_name =
        class_name.expect("#[entity_behavior] requires `class = \"...\"`");

    // The runtime downcast key defaults to `class`, but entities whose Mojang
    // class name differs from their registry identifier (e.g. `MinecartChest`
    // vs `chest_minecart`) pass an explicit `identifier = "..."` so downcasting
    // matches `entity_type().key`.
    let raw = identifier.unwrap_or(class_name).value();
    let (namespace, path) = if let Some((ns, p)) = raw.split_once(':') {
        (ns.to_owned(), p.to_owned())
    } else {
        ("minecraft".to_owned(), raw)
    };

    let stripped = strip_json_arg_attrs(item, "entity_behavior");
    let input: ItemStruct = parse2(stripped.clone())
        .unwrap_or_else(|_| panic!("#[entity_behavior] can only be applied to structs"));
    let name = &input.ident;

    quote! {
        #stripped

        impl crate::entity::EntityIdentifier for #name {
            const KEY: steel_utils::Identifier = steel_utils::Identifier::new_static(#namespace, #path);
        }
    }
}

fn strip_json_arg_attrs(item: TokenStream, macro_name: &str) -> TokenStream {
    let mut input: ItemStruct =
        parse2(item).unwrap_or_else(|_| panic!("#[{macro_name}] can only be applied to structs"));

    if let Fields::Named(ref mut fields) = input.fields {
        for field in &mut fields.named {
            field.attrs.retain(|attr| !attr.path().is_ident("json_arg"));
        }
    }

    quote! { #input }
}

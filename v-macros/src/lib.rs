extern crate proc_macro;

use std::collections::HashSet;

use proc_macro::TokenStream;
use quote::quote;
use syn::meta::parser;
use syn::parse::Parser;
use syn::{parse_macro_input, Data, DeriveInput, Fields, LitStr};

/// 为具名字段结构体追加基础模型字段，同时保留原字段与属性
/// 字段：
/// - `id: i64`
/// - `created_at: chrono::DateTime<chrono::Utc>`
/// - `updated_at: chrono::DateTime<chrono::Utc>`
/// - `deleted_at: Option<chrono::DateTime<chrono::Utc>>`
#[proc_macro_attribute]
pub fn base_model(_args: TokenStream, input: TokenStream) -> TokenStream {
    let input_ast = parse_macro_input!(input as DeriveInput);

    let attrs = input_ast.attrs.clone();
    let vis = input_ast.vis.clone();
    let ident = input_ast.ident.clone();
    let generics = input_ast.generics.clone();
    let (_impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let fields_named = match &input_ast.data {
        Data::Struct(s) => match &s.fields {
            Fields::Named(named) => named,
            Fields::Unnamed(_) | Fields::Unit => {
                let err = syn::Error::new_spanned(&ident, "#[base_model] 仅支持具名字段结构体");
                return err.to_compile_error().into();
            }
        },
        _ => {
            let err = syn::Error::new_spanned(&ident, "#[base_model] 仅能应用于结构体");
            return err.to_compile_error().into();
        }
    };

    let mut existing: HashSet<String> = HashSet::new();
    for f in fields_named.named.iter() {
        if let Some(id) = &f.ident {
            existing.insert(id.to_string());
        }
    }

    let original_fields = &fields_named.named;
    let orig_fields: Vec<_> = original_fields.iter().collect();

    let mut extra_fields = Vec::new();
    if !existing.contains("id") {
        extra_fields.push(quote! {
            #[doc = "主键标识"]
            pub id: i64,
        });
    }
    if !existing.contains("created_at") {
        extra_fields.push(quote! {
            #[doc = "创建时间戳"]
            pub created_at: chrono::DateTime<chrono::Utc>,
        });
    }
    if !existing.contains("updated_at") {
        extra_fields.push(quote! {
            #[doc = "更新时间戳"]
            pub updated_at: chrono::DateTime<chrono::Utc>,
        });
    }
    if !existing.contains("deleted_at") {
        extra_fields.push(quote! {
            #[doc = "软删除时间（可空）"]
            pub deleted_at: Option<chrono::DateTime<chrono::Utc>>,
        });
    }

    // 过滤掉原始的 #[derive(...)]，避免重复派生导致实现冲突；保留其他属性（如 #[model]）
    let _filtered_attrs: Vec<syn::Attribute> = attrs
        .into_iter()
        .filter(|a| !a.path().is_ident("derive"))
        .collect();

    let expanded = quote! {
        #(#_filtered_attrs)*
        #[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
        #vis struct #ident #ty_generics #where_clause {
            #(#orig_fields,)*
            #(#extra_fields)*
        }
    };

    TokenStream::from(expanded)
}

/// 标注模型的表名与分组，并为结构体生成关联常量；可选自动实现 `Model`。
/// 使用示例：
/// #[v_macros::model(table_name = "base_sys_conf", group_name = "default")]
/// struct BaseSysConf { ... }
/// 生成：
/// impl BaseSysConf { const TABLE_NAME: &str = "base_sys_conf"; const TABLE_GROUP: &str = "default"; }
/// 如需自动实现 Trait，可使用：
/// #[v_macros::model(table_name = "base_sys_conf", group_name = "default", auto_impl = true, trait_path = "v::db::database::Model")]
/// trait_path 可省略，默认使用 `v::db::database::Model`。
#[proc_macro_attribute]
pub fn model(args: TokenStream, input: TokenStream) -> TokenStream {
    let mut table_name: Option<String> = None;
    let mut group_name: String = "default".to_string();
    let mut auto_impl: bool = false; // 默认不自动实现，避免强依赖
    let mut trait_path: Option<String> = None;

    let parser = parser(|meta| {
        if meta.path.is_ident("table_name") {
            let s: LitStr = meta.value()?.parse()?;
            table_name = Some(s.value());
            Ok(())
        } else if meta.path.is_ident("group_name") {
            let s: LitStr = meta.value()?.parse()?;
            group_name = s.value();
            Ok(())
        } else if meta.path.is_ident("auto_impl") {
            let b: syn::LitBool = meta.value()?.parse()?;
            auto_impl = b.value();
            Ok(())
        } else if meta.path.is_ident("trait_path") {
            let s: LitStr = meta.value()?.parse()?;
            trait_path = Some(s.value());
            Ok(())
        } else {
            // 忽略未知键
            Ok(())
        }
    });

    if let Err(err) = parser.parse(args) {
        return err.to_compile_error().into();
    }

    let input_ast = parse_macro_input!(input as DeriveInput);

    // 仅允许应用于结构体
    match &input_ast.data {
        Data::Struct(_) => {}
        _ => {
            return syn::Error::new_spanned(&input_ast.ident, "#[model] 仅能应用于结构体")
                .to_compile_error()
                .into();
        }
    }

    let ident = input_ast.ident.clone();
    let generics = input_ast.generics.clone();
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let table = match table_name {
        Some(t) => t,
        None => {
            return syn::Error::new_spanned(
                &ident,
                "#[model] 需要提供 table，例如 table = \"my_table\"",
            )
            .to_compile_error()
            .into();
        }
    };

    let item_tokens = quote! { #input_ast };

    let impl_tokens = quote! {
        impl #impl_generics #ident #ty_generics #where_clause {
            pub const TABLE_NAME: &'static str = #table;
            pub const TABLE_GROUP: &'static str = #group_name;
        }
    };

    // 可选自动实现指定的 Model Trait，减少样板代码；默认路径为 v::db::database::Model
    if auto_impl {
        let trait_path_str = trait_path.unwrap_or_else(|| "v::db::database::Model".to_string());
        match syn::parse_str::<syn::Path>(&trait_path_str) {
            Ok(path) => {
                let trait_impl_tokens = quote! {
                    impl #impl_generics #path for #ident #ty_generics #where_clause {
                        fn table_name() -> &'static str { Self::TABLE_NAME }
                        fn group_name() -> &'static str { Self::TABLE_GROUP }
                    }
                };
                TokenStream::from(quote! { #item_tokens #impl_tokens #trait_impl_tokens })
            }
            Err(e) => {
                syn::Error::new_spanned(
                    &ident,
                    format!("#[model] trait_path 解析失败: {}", e)
                ).to_compile_error().into()
            }
        }
    } else {
        TokenStream::from(quote! { #item_tokens #impl_tokens })
    }
}

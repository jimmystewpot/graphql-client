extern crate proc_macro;

/// Derive-related code. This will be moved into graphql_query_derive.
mod attributes;

use graphql_client_codegen::{
    generate_module_token_stream, CodegenMode, GraphQLClientCodegenOptions,
};
use std::{
    env,
    path::{Path, PathBuf},
};

use proc_macro2::TokenStream;

#[proc_macro_derive(GraphQLQuery, attributes(graphql))]
pub fn derive_graphql_query(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    match graphql_query_derive_inner(input) {
        Ok(ts) => ts,
        Err(err) => {
            println!("DERIVE_ERROR: {:?}", err);
            err.to_compile_error().into()
        }
    }
}

fn graphql_query_derive_inner(
    input: proc_macro::TokenStream,
) -> Result<proc_macro::TokenStream, syn::Error> {
    println!("graphql_query_derive_inner input: {}", input);
    let input = TokenStream::from(input);
    let ast = syn::parse2(input)?;
    let (query_path, schema_path) = build_query_and_schema_path(&ast)?;
    let options = build_graphql_client_derive_options(&ast, query_path.clone())?;

    println!("graphql_query_derive_inner:\nquery_path: {},\nschema_path: {},\noptions: {:#?}", query_path.display(), schema_path.display(), options);
    generate_module_token_stream(query_path, &schema_path, options)
        .map(Into::into)
        .map_err(|err| {
            syn::Error::new_spanned(
                ast,
                format!("Failed to generate GraphQLQuery impl: {}", err),
            )
        })
}

fn build_query_and_schema_path(input: &syn::DeriveInput) -> Result<(PathBuf, PathBuf), syn::Error> {
    let cargo_manifest_dir = env::var("CARGO_MANIFEST_DIR").map_err(|_err| {
        syn::Error::new_spanned(
            input,
            "Error checking that the CARGO_MANIFEST_DIR env variable is defined.",
        )
    })?;

    let query_path = attributes::extract_attr(input, "query_path")?;
    let query_path = format!("{}/{}", cargo_manifest_dir, query_path);
    let query_path = Path::new(&query_path).to_path_buf();
    let schema_path = attributes::extract_attr(input, "schema_path")?;
    let schema_path = Path::new(&cargo_manifest_dir).join(schema_path);
    Ok((query_path, schema_path))
}

fn build_graphql_client_derive_options(
    input: &syn::DeriveInput,
    query_path: PathBuf,
) -> Result<GraphQLClientCodegenOptions, syn::Error> {
    let variables_derives = attributes::extract_attr(input, "variables_derives").ok();
    let response_derives = attributes::extract_attr(input, "response_derives").ok();
    let custom_scalars_module = attributes::extract_attr(input, "custom_scalars_module").ok();
    let extern_enums = attributes::extract_attr_list(input, "extern_enums").ok();
    let fragments_other_variant: bool = attributes::extract_fragments_other_variant(input);
    let skip_serializing_none: bool = attributes::extract_skip_serializing_none(input);

    let mut options = GraphQLClientCodegenOptions::new(CodegenMode::Derive);
    options.set_query_file(query_path);
    options.set_fragments_other_variant(fragments_other_variant);
    options.set_skip_serializing_none(skip_serializing_none);

    if let Some(variables_derives) = variables_derives {
        options.set_variables_derives(variables_derives);
    };

    if let Some(response_derives) = response_derives {
        options.set_response_derives(response_derives);
    };

    // The user can determine what to do about deprecations.
    if let Ok(deprecation_strategy) = attributes::extract_deprecation_strategy(input) {
        options.set_deprecation_strategy(deprecation_strategy);
    };

    // The user can specify the normalization strategy.
    if let Ok(normalization) = attributes::extract_normalization(input) {
        options.set_normalization(normalization);
    };

    // The user can give a path to a module that provides definitions for the custom scalars.
    if let Some(custom_scalars_module) = custom_scalars_module {
        let custom_scalars_module = syn::parse_str(&custom_scalars_module)?;

        options.set_custom_scalars_module(custom_scalars_module);
    }

    // The user can specify a list of enums types that are defined externally, rather than generated by this library
    if let Some(extern_enums) = extern_enums {
        options.set_extern_enums(extern_enums);
    }

    options.set_struct_ident(input.ident.clone());
    options.set_module_visibility(input.vis.clone());
    options.set_operation_name(input.ident.to_string());
    options.set_serde_path(syn::parse_quote!(graphql_client::_private::serde));

    Ok(options)
}

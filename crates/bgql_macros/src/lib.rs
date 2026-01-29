//! Procedural macros for Better GraphQL SDK.
//!
//! Provides type-safe macro definitions for GraphQL operations and resolvers.
//!
//! # Example
//!
//! ```ignore
//! use bgql_macros::{graphql_operation, resolver};
//!
//! // Define a typed operation
//! graphql_operation! {
//!     query GetUser($id: ID!) -> GetUserData {
//!         user(id: $id) {
//!             id
//!             name
//!             email
//!         }
//!     }
//! }
//!
//! // Define a typed resolver
//! #[resolver(Query)]
//! async fn get_user(ctx: &Context, args: GetUserArgs) -> Result<User> {
//!     let user_id = ctx.get::<CurrentUserId>()?;
//!     // ...
//! }
//! ```

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};
use syn::{
    parse_macro_input, punctuated::Punctuated, Attribute, DeriveInput, Fields, FnArg, Ident,
    ItemFn, ItemStruct, LitStr, ReturnType, Token, Type,
};

/// Derive macro for typed GraphQL operations.
///
/// # Example
///
/// ```ignore
/// #[derive(TypedOperation)]
/// #[operation(
///     query = "query GetUser($id: ID!) { user(id: $id) { id name } }",
///     name = "GetUser"
/// )]
/// pub struct GetUser {
///     pub id: String,
/// }
/// ```
#[proc_macro_derive(TypedOperation, attributes(operation))]
pub fn derive_typed_operation(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let name = &input.ident;
    let (query, op_name, kind) = parse_operation_attrs(&input.attrs);

    let variables_type = format_ident!("{}Variables", name);
    let response_type = format_ident!("{}Response", name);

    let expanded = quote! {
        impl ::bgql_sdk::typed::TypedOperation for #name {
            type Variables = #variables_type;
            type Response = #response_type;

            const OPERATION: &'static str = #query;
            const OPERATION_NAME: &'static str = #op_name;
            const KIND: ::bgql_sdk::typed::OperationKind = ::bgql_sdk::typed::OperationKind::#kind;
        }
    };

    TokenStream::from(expanded)
}

fn parse_operation_attrs(attrs: &[Attribute]) -> (String, String, Ident) {
    let mut query = String::new();
    let mut name = String::new();
    let mut kind = format_ident!("Query");

    for attr in attrs {
        if attr.path().is_ident("operation") {
            let _ = attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("query") {
                    let value: LitStr = meta.value()?.parse()?;
                    query = value.value();
                } else if meta.path.is_ident("mutation") {
                    let value: LitStr = meta.value()?.parse()?;
                    query = value.value();
                    kind = format_ident!("Mutation");
                } else if meta.path.is_ident("name") {
                    let value: LitStr = meta.value()?.parse()?;
                    name = value.value();
                }
                Ok(())
            });
        }
    }

    (query, name, kind)
}

/// Attribute macro for typed resolvers.
///
/// # Example
///
/// ```ignore
/// #[resolver(Query, "user")]
/// async fn get_user(
///     ctx: &TypedContext,
///     args: GetUserArgs,
/// ) -> SdkResult<User> {
///     // ...
/// }
/// ```
#[proc_macro_attribute]
pub fn resolver(attr: TokenStream, item: TokenStream) -> TokenStream {
    let args = parse_macro_input!(attr as ResolverArgs);
    let input = parse_macro_input!(item as ItemFn);

    let fn_name = &input.sig.ident;
    let fn_vis = &input.vis;
    let fn_block = &input.block;
    let fn_asyncness = &input.sig.asyncness;

    let type_name = &args.type_name;
    let field_name = args
        .field_name
        .as_ref()
        .map(|s| s.value())
        .unwrap_or_else(|| fn_name.to_string());

    // Extract argument types
    let (parent_type, args_type, ctx_type) = extract_resolver_arg_types(&input.sig.inputs);

    // Extract return type
    let return_type = match &input.sig.output {
        ReturnType::Default => quote! { () },
        ReturnType::Type(_, ty) => quote! { #ty },
    };

    let expanded = quote! {
        #fn_vis #fn_asyncness fn #fn_name(
            __parent: #parent_type,
            __args: #args_type,
            __ctx: &#ctx_type,
            __info: &::bgql_sdk::ResolverInfo,
        ) -> #return_type {
            // Destructure args for the inner function
            let args = __args;
            let ctx = __ctx;
            let parent = __parent;

            #fn_block
        }

        impl ::bgql_sdk::typed::ResolverRegistration for #fn_name {
            const TYPE_NAME: &'static str = stringify!(#type_name);
            const FIELD_NAME: &'static str = #field_name;
        }
    };

    TokenStream::from(expanded)
}

struct ResolverArgs {
    type_name: Ident,
    field_name: Option<LitStr>,
}

impl syn::parse::Parse for ResolverArgs {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let type_name: Ident = input.parse()?;

        let field_name = if input.peek(Token![,]) {
            input.parse::<Token![,]>()?;
            Some(input.parse()?)
        } else {
            None
        };

        Ok(ResolverArgs {
            type_name,
            field_name,
        })
    }
}

fn extract_resolver_arg_types(
    inputs: &Punctuated<FnArg, Token![,]>,
) -> (TokenStream2, TokenStream2, TokenStream2) {
    let mut parent_type = quote! { () };
    let mut args_type = quote! { ::bgql_sdk::typed::NoArgs };
    let mut ctx_type = quote! { ::bgql_sdk::context::TypedContext };

    for (i, arg) in inputs.iter().enumerate() {
        if let FnArg::Typed(pat_type) = arg {
            let ty = &pat_type.ty;
            match i {
                0 => {
                    // First arg could be parent or context
                    if is_context_type(ty) {
                        ctx_type = quote! { #ty };
                    } else {
                        parent_type = quote! { #ty };
                    }
                }
                1 => args_type = quote! { #ty },
                2 => ctx_type = quote! { #ty },
                _ => {}
            }
        }
    }

    (parent_type, args_type, ctx_type)
}

fn is_context_type(ty: &Type) -> bool {
    // Simple heuristic - check if type name contains "Context"
    let ty_str = quote!(#ty).to_string();
    ty_str.contains("Context")
}

/// Derive macro for typed context keys.
///
/// # Example
///
/// ```ignore
/// #[derive(ContextKey)]
/// pub struct CurrentUserId(pub String);
/// ```
#[proc_macro_derive(ContextKey)]
pub fn derive_context_key(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;

    let expanded = quote! {
        impl ::bgql_sdk::context::FromContext for #name {
            fn from_context(ctx: &::bgql_sdk::context::TypedContext) -> Option<Self> {
                ctx.get::<Self>().cloned()
            }
        }
    };

    TokenStream::from(expanded)
}

/// Macro for defining GraphQL operations inline.
///
/// # Example
///
/// ```ignore
/// graphql! {
///     query GetUser($id: ID!) {
///         user(id: $id) {
///             id
///             name
///         }
///     }
/// }
/// ```
#[proc_macro]
pub fn graphql(input: TokenStream) -> TokenStream {
    parse_gql_impl(input)
}

/// Alias for `graphql!` macro.
///
/// # Example
///
/// ```ignore
/// // Define a typed operation
/// gql! {
///     query GetUser($id: ID!) {
///         user(id: $id) {
///             id
///             name
///             email
///         }
///     }
/// }
///
/// // Use with client
/// let result = client.execute_typed::<GetUser>(GetUserVariables { id: "1".into() }).await?;
/// ```
///
/// For schema-first workflow, codegen generates `GetUserVariables` and `GetUserData` types.
/// This macro creates the operation struct that binds them together.
#[proc_macro]
pub fn gql(input: TokenStream) -> TokenStream {
    parse_gql_impl(input)
}

fn parse_gql_impl(input: TokenStream) -> TokenStream {
    let input_str = input.to_string();

    // Parse the GraphQL operation
    let (op_kind, op_name, _variables, query) = parse_graphql_string(&input_str);

    let kind_ident = format_ident!("{}", op_kind);
    let name_ident = format_ident!("{}", op_name);
    let vars_ident = format_ident!("{}Variables", op_name);
    let data_ident = format_ident!("{}Data", op_name);

    let expanded = quote! {
        pub struct #name_ident;

        impl ::bgql_sdk::typed::TypedOperation for #name_ident {
            type Variables = #vars_ident;
            type Response = #data_ident;

            const OPERATION: &'static str = #query;
            const OPERATION_NAME: &'static str = stringify!(#name_ident);
            const KIND: ::bgql_sdk::typed::OperationKind = ::bgql_sdk::typed::OperationKind::#kind_ident;
        }
    };

    TokenStream::from(expanded)
}

fn parse_graphql_string(input: &str) -> (String, String, Vec<String>, String) {
    // Simple parser for GraphQL operation
    let input = input.trim();

    let op_kind = if input.starts_with("query") {
        "Query"
    } else if input.starts_with("mutation") {
        "Mutation"
    } else {
        "Query"
    };

    // Extract operation name (simplified)
    let name_start = input.find(char::is_alphabetic).unwrap_or(0);
    let rest = &input[name_start..];
    let name_end = rest
        .find(|c: char| !c.is_alphanumeric() && c != '_')
        .unwrap_or(rest.len());
    let op_name = &rest[..name_end];

    (
        op_kind.to_string(),
        op_name.to_string(),
        vec![],
        input.to_string(),
    )
}

/// Macro for defining field arguments.
///
/// # Example
///
/// ```ignore
/// args! {
///     pub struct GetUserArgs {
///         id: String,
///         #[optional]
///         include_posts: Option<bool>,
///     }
/// }
/// ```
#[proc_macro]
pub fn args(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as ItemStruct);
    let name = &input.ident;
    let vis = &input.vis;
    let fields = &input.fields;

    let field_impls = match fields {
        Fields::Named(named) => named
            .named
            .iter()
            .map(|f| {
                let fname = &f.ident;
                let fty = &f.ty;
                quote! {
                    pub #fname: #fty
                }
            })
            .collect::<Vec<_>>(),
        _ => vec![],
    };

    let expanded = quote! {
        #[derive(Debug, Clone, ::serde::Deserialize)]
        #vis struct #name {
            #(#field_impls),*
        }

        impl ::bgql_sdk::typed::GraphQLArgs for #name {}
    };

    TokenStream::from(expanded)
}

/// Macro for registering resolvers.
///
/// # Example
///
/// ```ignore
/// resolvers! {
///     Query {
///         user => get_user,
///         users => get_users,
///     }
///     Mutation {
///         create_user => create_user,
///     }
/// }
/// ```
#[proc_macro]
pub fn resolvers(input: TokenStream) -> TokenStream {
    let input_str = input.to_string();

    // Parse resolver registrations (simplified)
    let mut registrations = Vec::new();

    let lines: Vec<&str> = input_str.lines().collect();
    let mut current_type = "";

    for line in lines {
        let line = line.trim();
        if line.ends_with('{') {
            current_type = line.trim_end_matches('{').trim();
        } else if line.contains("=>") {
            let parts: Vec<&str> = line.split("=>").collect();
            if parts.len() == 2 {
                let field = parts[0].trim().trim_end_matches(',');
                let resolver = parts[1].trim().trim_end_matches(',');
                registrations.push((
                    current_type.to_string(),
                    field.to_string(),
                    resolver.to_string(),
                ));
            }
        }
    }

    let register_calls: Vec<TokenStream2> = registrations
        .iter()
        .map(|(type_name, field, resolver)| {
            let type_ident = format_ident!("{}", type_name);
            let resolver_ident = format_ident!("{}", resolver);
            quote! {
                builder.register(stringify!(#type_ident), #field, #resolver_ident);
            }
        })
        .collect();

    let expanded = quote! {
        {
            let mut builder = ::bgql_sdk::typed::ResolverBuilder::new();
            #(#register_calls)*
            builder.build()
        }
    };

    TokenStream::from(expanded)
}

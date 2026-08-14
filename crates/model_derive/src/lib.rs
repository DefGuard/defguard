#[cfg(test)]
mod tests;

use proc_macro::TokenStream;
use quote::quote;
use syn::{
    Attribute, Data, DataStruct, DeriveInput, Field, Fields, FieldsNamed, GenericArgument, Ident,
    Path, PathArguments, PathSegment, Type, TypePath, parse_macro_input,
};

#[derive(Clone, Copy)]
#[cfg_attr(test, derive(Debug, Eq, PartialEq))]
enum ModelType {
    Any,
    Enum,
    Ip,
    Option,
    OptionRef,
    Ref,
    Secret,
}

impl ModelType {
    fn is_any(self) -> bool {
        matches!(self, Self::Any)
    }
}

impl From<&Ident> for ModelType {
    fn from(value: &Ident) -> Self {
        if value == "enum" {
            Self::Enum
        } else if value == "ip" {
            Self::Ip
        } else if value == "option" {
            Self::Option
        } else if value == "option_ref" {
            Self::OptionRef
        } else if value == "ref" {
            Self::Ref
        } else if value == "secret" {
            Self::Secret
        } else {
            Self::Any
        }
    }
}

/// Try to find the value of `model` attribute, e.g. `#[model(type)]`.
fn model_attr(field: &Field) -> syn::Result<ModelType> {
    let mut model_type = ModelType::Any;

    if let Some(attr) = field
        .attrs
        .iter()
        .find(|attr| attr.path().is_ident("model"))
    {
        attr.parse_nested_meta(|meta| {
            if !model_type.is_any() {
                Err(meta.error("expected a single model property"))
            } else if let Some(ident) = meta.path.get_ident() {
                model_type = ident.into();
                if model_type.is_any() {
                    Err(meta.error("unsupported model property"))
                } else {
                    Ok(())
                }
            } else {
                Err(meta.error("unsupported model property"))
            }
        })?;
    }

    Ok(model_type)
}

/// Try to find the value of `table` attribute, e.g. `#[table(name)]`.
fn table_attr(attrs: &[Attribute], default_name: &Ident) -> syn::Result<String> {
    let mut table_name = default_name.to_string().to_ascii_lowercase();

    if let Some(attr) = attrs.iter().find(|attr| attr.path().is_ident("table")) {
        attr.parse_nested_meta(|meta| {
            if let Some(ident) = meta.path.get_ident() {
                table_name = ident.to_string();
                Ok(())
            } else {
                Err(meta.error("unsupported table property"))
            }
        })?;
    }

    Ok(table_name)
}

/// Last segment of a plain path type, e.g. `Bar` in `foo::Bar<T>`.
fn last_segment(ty: &Type) -> Option<&PathSegment> {
    if let Type::Path(TypePath {
        path: Path { segments, .. },
        ..
    }) = ty
    {
        segments.last()
    } else {
        None
    }
}

fn field_type(ty: &Type) -> Option<&Ident> {
    Some(&last_segment(ty)?.ident)
}

/// The `T` in `Option<T>`.
fn option_field_type(ty: &Type) -> Option<&Ident> {
    let segment = last_segment(ty)?;
    if segment.ident == "Option"
        && let PathArguments::AngleBracketed(args) = &segment.arguments
        && let Some(GenericArgument::Type(inner_ty)) = args.args.first()
    {
        return field_type(inner_ty);
    }
    None
}

#[proc_macro_derive(Model, attributes(table, model))]
pub fn derive(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    expand(&ast)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

fn expand(ast: &DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
    let name = &ast.ident;
    let table_name = table_attr(&ast.attrs, name)?;

    let Data::Struct(DataStruct {
        fields: Fields::Named(FieldsNamed { named, .. }),
        ..
    }) = &ast.data
    else {
        return Err(syn::Error::new_spanned(
            ast,
            "Model can only be derived for structs with named fields",
        ));
    };

    let non_id_fields = named
        .iter()
        .filter_map(|field| {
            let ident = field.ident.as_ref()?;
            (ident != "id").then_some((ident, field))
        })
        .collect::<Vec<_>>();

    // Quoted fields ("field1", "field2", ...)
    let mut cs_fields = Vec::with_capacity(non_id_fields.len());
    // Quoted fields with aliases ("field1", "field2" "field2: _", ...)
    let mut cs_aliased_fields = Vec::with_capacity(non_id_fields.len());
    // Values ($1, $2, ...)
    let mut cs_values = Vec::with_capacity(non_id_fields.len());
    // Setters ("field1" = $2, "field2" = $3, ...)
    let mut cs_setters = Vec::with_capacity(non_id_fields.len());

    // Interpolated into both the INSERT and the UPDATE query below.
    let mut query_args = Vec::with_capacity(non_id_fields.len());
    let mut struct_fields = Vec::with_capacity(non_id_fields.len());

    for (index, (name, field)) in non_id_fields.iter().enumerate() {
        let model_type = model_attr(field)?;
        let insert_value_number = index + 1;
        let update_value_number = index + 2;

        cs_fields.push(format!("\"{name}\""));
        cs_values.push(format!("${insert_value_number}"));
        cs_setters.push(format!("\"{name}\" = ${update_value_number}"));
        cs_aliased_fields.push(match model_type {
            ModelType::Any => format!("\"{name}\""),
            ModelType::Secret => format!("\"{name}\" \"{name}?: SecretString\""),
            ModelType::Ip => format!("\"{name}\" \"{name}: IpAddr\""),
            ModelType::Option | ModelType::OptionRef => format!("\"{name}\" \"{name}?: _\""),
            ModelType::Enum | ModelType::Ref => format!("\"{name}\" \"{name}: _\""),
        });

        query_args.push(match model_type {
            ModelType::Any => quote! { self.#name },
            ModelType::Enum => {
                if let Some(field_type) = field_type(&field.ty) {
                    quote! { &self.#name as &#field_type }
                } else {
                    quote! { &self.#name }
                }
            }
            ModelType::Option => {
                if let Some(field_type) = option_field_type(&field.ty) {
                    quote! { &self.#name as &Option<#field_type> }
                } else {
                    quote! { &self.#name }
                }
            }
            ModelType::OptionRef => quote! { self.#name.as_deref() },
            // FIXME: hard-coded struct name
            ModelType::Secret => quote! { &self.#name as &Option<SecretString> },
            // FIXME: hard-coded struct name
            ModelType::Ip => quote! { &self.#name as &IpAddr },
            ModelType::Ref => quote! { &self.#name },
        });
        struct_fields.push(quote! { #name: self.#name });
    }

    let cs_fields = cs_fields.join(",");
    let cs_aliased_fields = cs_aliased_fields.join(",");
    let cs_values = cs_values.join(",");
    let cs_setters = cs_setters.join(",");

    // Queries
    let all_query = format!("SELECT id, {cs_aliased_fields} FROM \"{table_name}\"");
    let all_query_limited = format!("{all_query} LIMIT $1 OFFSET $2");
    let find_by_id_query = format!("{all_query} WHERE id = $1");
    let delete_query = format!("DELETE FROM \"{table_name}\" WHERE id = $1");
    let insert_query =
        format!("INSERT INTO \"{table_name}\" ({cs_fields}) VALUES ({cs_values}) RETURNING id");
    let update_query = format!("UPDATE \"{table_name}\" SET {cs_setters} WHERE id = $1");
    let count_query = format!("SELECT count(*) FROM \"{table_name}\"");

    Ok(quote! {
        impl #name<NoId> {
            pub async fn save<'e, E>(self, executor: E) -> sqlx::Result<#name<Id>>
            where
                E: sqlx::PgExecutor<'e>
            {
                let id = sqlx::query_scalar!(#insert_query, #(#query_args,)*).fetch_one(executor).await?;
                Ok(#name { id, #(#struct_fields,)* })
            }

            pub fn with_id(self, id: Id) -> #name<Id> {
                #name { id, #(#struct_fields,)* }
            }
        }

        impl #name<Id> {
            pub async fn find_by_id<'e, E>(executor: E, id: Id) -> sqlx::Result<Option<Self>>
            where
                E: sqlx::PgExecutor<'e>
            {
                sqlx::query_as!(Self, #find_by_id_query, id).fetch_optional(executor).await
            }

            pub async fn all<'e, E>(executor: E) -> sqlx::Result<Vec<Self>>
            where
                E: sqlx::PgExecutor<'e>
            {
                sqlx::query_as!(Self, #all_query).fetch_all(executor).await
            }

            pub async fn all_paginated<'e, E>(executor: E, limit: i64, offset: i64) -> sqlx::Result<Vec<Self>>
            where
                E: sqlx::PgExecutor<'e>
            {
                sqlx::query_as!(Self, #all_query_limited, limit, offset).fetch_all(executor).await
            }

            pub async fn delete<'e, E>(self, executor: E) -> sqlx::Result<()>
            where
                E: sqlx::PgExecutor<'e>
            {
                sqlx::query!(#delete_query, self.id).execute(executor).await?;
                Ok(())
            }

            pub async fn save<'e, E>(&self, executor: E) -> sqlx::Result<()>
            where
                E: sqlx::PgExecutor<'e>
            {
                sqlx::query!(#update_query, self.id, #(#query_args,)*).execute(executor).await?;
                Ok(())
            }

            pub async fn count<'e, E>(executor: E) -> sqlx::Result<i64>
            where
                E: sqlx::PgExecutor<'e>,
            {
                let count = sqlx::query_scalar!(#count_query).fetch_one(executor).await?
                    .unwrap_or_default();
                Ok(count)
            }

            pub fn as_noid(self) -> #name {
                #name { id: NoId, #(#struct_fields,)* }
            }
        }
    })
}

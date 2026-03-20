use proc_macro::TokenStream;
use quote::quote;
use syn::{
    Attribute, Data, DataStruct, DeriveInput, Field, Fields, FieldsNamed, GenericArgument, Ident,
    Path, PathArguments, Type, TypePath, parse_macro_input,
};

#[derive(Clone, Copy, PartialEq, Eq)]
enum ModelType {
    Enum,
    Ip,
    Option,
    Ref,
    Secret,
}

/// Try to find the value of `model` attribute, e.g. `#[model(model_type)]`.
fn model_attr(field: &Field) -> syn::Result<Option<ModelType>> {
    for attr in &field.attrs {
        if attr.path().is_ident("model") {
            let mut model_type = None;

            attr.parse_nested_meta(|meta| {
                if model_type.is_some() {
                    return Err(meta.error("expected a single model property"));
                }

                model_type = Some(if meta.path.is_ident("enum") {
                    ModelType::Enum
                } else if meta.path.is_ident("ip") {
                    ModelType::Ip
                } else if meta.path.is_ident("option") {
                    ModelType::Option
                } else if meta.path.is_ident("ref") {
                    ModelType::Ref
                } else if meta.path.is_ident("secret") {
                    ModelType::Secret
                } else {
                    return Err(meta.error("unsupported model property"));
                });

                Ok(())
            })?;

            return Ok(model_type);
        }
    }

    Ok(None)
}

fn table_attr(attrs: &[Attribute], default_name: &Ident) -> syn::Result<String> {
    let mut table_name = default_name.to_string().to_ascii_lowercase();

    for attr in attrs {
        if attr.path().is_ident("table") {
            attr.parse_nested_meta(|meta| {
                if let Some(ident) = meta.path.get_ident() {
                    table_name = ident.to_string();
                    Ok(())
                } else {
                    Err(meta.error("unsupported table property"))
                }
            })?;

            break;
        }
    }

    Ok(table_name)
}

fn field_type(ty: &Type) -> Option<&Ident> {
    if let Type::Path(TypePath {
        path: Path { segments, .. },
        ..
    }) = ty
    {
        if let Some(segment) = segments.last() {
            return Some(&segment.ident);
        }
    }
    None
}

fn option_field_type(ty: &Type) -> Option<&Ident> {
    if let Type::Path(TypePath {
        path: Path { segments, .. },
        ..
    }) = ty
    {
        if let Some(segment) = segments.last() {
            if segment.ident == "Option" {
                // Extract the generic arguments
                if let PathArguments::AngleBracketed(args) = &segment.arguments {
                    // Get the first generic argument (the T in Option<T>)
                    if let Some(GenericArgument::Type(inner_ty)) = args.args.first() {
                        return field_type(inner_ty);
                    }
                }
            }
        }
    }
    None
}

#[proc_macro_derive(Model, attributes(table, model))]
pub fn derive(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    let name = &ast.ident;

    let table_name = match table_attr(&ast.attrs, name) {
        Ok(table_name) => table_name,
        Err(err) => return err.to_compile_error().into(),
    };

    let Data::Struct(DataStruct {
        fields: Fields::Named(FieldsNamed { named, .. }),
        ..
    }) = &ast.data
    else {
        return syn::Error::new_spanned(
            &ast,
            "Model can only be derived for structs with named fields",
        )
        .to_compile_error()
        .into();
    };

    let non_id_fields = named
        .iter()
        .filter(|field| field.ident.as_ref().is_some_and(|ident| ident != "id"))
        .collect::<Vec<_>>();

    // comma-separated fields ("field1", "field2", ...)
    let mut cs_fields = String::with_capacity(non_id_fields.len() * 16);
    // comma-separated fields with quotes and aliases ("field1", "field2" "field2: _", ...)
    let mut cs_aliased_fields = String::with_capacity(non_id_fields.len() * 24);
    // comma-separated values ($1, $2, ...)
    let mut cs_values = String::with_capacity(non_id_fields.len() * 4);
    // comma-separated setters ("field1" = $2, "field2" = $3, ...)
    let mut cs_setters = String::with_capacity(non_id_fields.len() * 20);

    let mut insert_args = Vec::with_capacity(non_id_fields.len());
    let mut struct_fields = Vec::with_capacity(non_id_fields.len());

    for (index, field) in non_id_fields.iter().enumerate() {
        let Some(name) = &field.ident else {
            continue;
        };

        let model_type = match model_attr(field) {
            Ok(model_type) => model_type,
            Err(err) => return err.to_compile_error().into(),
        };

        if index > 0 {
            cs_fields.push(',');
            cs_aliased_fields.push(',');
            cs_values.push(',');
            cs_setters.push(',');
        }

        let name_string = name.to_string();
        let insert_value_number = index + 1;
        let update_value_number = index + 2;

        cs_fields.push('"');
        cs_fields.push_str(&name_string);
        cs_fields.push('"');
        cs_aliased_fields.push('"');
        cs_aliased_fields.push_str(&name_string);
        cs_aliased_fields.push('"');
        if let Some(model_type) = model_type {
            cs_aliased_fields.push_str(" \"");
            cs_aliased_fields.push_str(&name_string);
            match model_type {
                ModelType::Secret => cs_aliased_fields.push_str("?: SecretString\""),
                ModelType::Ip => cs_aliased_fields.push_str(": IpAddr\""),
                ModelType::Option => cs_aliased_fields.push_str("?: _\""),
                ModelType::Enum | ModelType::Ref => cs_aliased_fields.push_str(": _\""),
            }
        }
        cs_values.push('$');
        cs_values.push_str(&insert_value_number.to_string());
        cs_setters.push('"');
        cs_setters.push_str(&name_string);
        cs_setters.push_str("\" = $");
        cs_setters.push_str(&update_value_number.to_string());

        let insert_arg = match model_type {
            Some(ModelType::Enum) => {
                if let Some(field_type) = field_type(&field.ty) {
                    quote! { &self.#name as &#field_type }
                } else {
                    quote! { &self.#name }
                }
            }
            Some(ModelType::Option) => {
                if let Some(field_type) = option_field_type(&field.ty) {
                    quote! { &self.#name as &Option<#field_type> }
                } else {
                    quote! { &self.#name }
                }
            }
            Some(ModelType::Secret) => {
                // FIXME: hard-coded struct name
                quote! { &self.#name as &Option<SecretString> }
            }
            Some(ModelType::Ip) => {
                // FIXME: hard-coded struct name
                quote! { &self.#name as &IpAddr }
            }
            Some(ModelType::Ref) => quote! { &self.#name },
            None => quote! { self.#name },
        };

        insert_args.push(insert_arg);
        struct_fields.push(quote! { #name: self.#name });
    }

    let update_args = insert_args.clone();

    // queries
    let all_query = format!("SELECT id, {cs_aliased_fields} FROM \"{table_name}\"");
    let all_query_limited = all_query.clone() + " LIMIT $1 OFFSET $2";
    let find_by_id_query = all_query.clone() + " WHERE id = $1";
    let delete_query = format!("DELETE FROM \"{table_name}\" WHERE id = $1");
    let insert_query =
        format!("INSERT INTO \"{table_name}\" ({cs_fields}) VALUES ({cs_values}) RETURNING id");
    let update_query = format!("UPDATE \"{table_name}\" SET {cs_setters} WHERE id = $1");
    let count_query = format!("SELECT count(*) FROM \"{table_name}\"");

    quote! {
        impl #name<NoId> {
            pub async fn save<'e, E>(self, executor: E) -> sqlx::Result<#name<Id>>
            where
                E: sqlx::PgExecutor<'e>
            {
                let id = sqlx::query_scalar!(#insert_query, #(#insert_args,)*).fetch_one(executor).await?;
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
                sqlx::query!(#update_query, self.id, #(#update_args,)*).execute(executor).await?;
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
    }
    .into()
}

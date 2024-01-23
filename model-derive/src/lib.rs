use proc_macro::TokenStream;
use quote::quote;
use syn::{
    meta::parser, parse::Parser, parse_macro_input, Data, DataStruct, DeriveInput, Field, Fields,
    FieldsNamed, Ident, Path, Type, TypePath,
};

/// Try to find the value of `model` attribute, e.g. `#[model(model_type)]`.
fn model_attr(f: &Field) -> Option<String> {
    // default
    let mut model_type = None;

    // Closure here, because `model_parser` must be dropped before `model_type` is returned.
    {
        let model_parser = parser(|meta| {
            if let Some(ident) = meta.path.get_ident() {
                model_type = Some(ident.to_string());
                Ok(())
            } else {
                Err(meta.error("unsupported model property"))
            }
        });

        for attr in &f.attrs {
            if attr.path().is_ident("model") {
                if let Ok(inner) = attr.meta.require_list() {
                    // `proc_macro2::TokenStream` to `proc_macro::TokenStream`
                    let tokens: TokenStream = inner.tokens.clone().into();
                    Parser::parse(model_parser, tokens).unwrap();
                    break;
                }
            }
        }
    }

    model_type
}

fn field_type(ty: &Type) -> Option<&Ident> {
    if let Type::Path(TypePath {
        path: Path { segments, .. },
        ..
    }) = ty
    {
        if let Some(segment) = segments.first() {
            return Some(&segment.ident);
        }
    }
    None
}

#[proc_macro_derive(Model, attributes(table, model))]
pub fn derive(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    let name = &ast.ident;

    // Find the "table" attribute, e.g. `#[table(mytable)]`
    let attribute = ast.attrs.iter().find(|a| a.path().is_ident("table"));

    // Default table name
    let mut table_name = name.to_string().to_ascii_lowercase();

    // Parse table name if attribute is present
    if let Some(attr) = attribute {
        let table_name_parser = parser(|meta| {
            if let Some(ident) = meta.path.get_ident() {
                table_name = ident.to_string();
                Ok(())
            } else {
                Err(meta.error("unsupported table property"))
            }
        });
        if let Ok(inner) = attr.meta.require_list() {
            // `proc_macro2::TokenStream` to `proc_macro::TokenStream`
            let tokens: TokenStream = inner.tokens.clone().into();
            parse_macro_input!(tokens with table_name_parser);
        }
    }

    let Data::Struct(DataStruct {
        fields: Fields::Named(FieldsNamed { named, .. }),
        ..
    }) = ast.data
    else {
        // fail for other but `struct`
        unimplemented!();
    };

    // comma-separated fields ("field1", "field2", ...)
    let mut cs_fields = String::new();
    // comma-separated fields with quotes and aliases ("field1", "field2" "field2: _", ...)
    let mut cs_aliased_fields = String::new();
    // comma-separated values ($1, $2, ...)
    let mut cs_values = String::new();
    // comma-separated setters ("field1" = $2, "field2" = $3, ...)
    let mut cs_setters = String::new();

    let mut add_comma = false;
    let mut value_number = 1;
    named.iter().for_each(|field| {
        if let Some(name) = &field.ident {
            if name != "id" {
                if add_comma {
                    cs_fields.push(',');
                    cs_aliased_fields.push(',');
                    cs_values.push(',');
                    cs_setters.push(',');
                } else {
                    add_comma = true;
                }

                let name_string = name.to_string();

                cs_fields.push('"');
                cs_fields.push_str(&name_string);
                cs_fields.push('"');
                cs_aliased_fields.push('"');
                cs_aliased_fields.push_str(&name_string);
                cs_aliased_fields.push('"');
                if let Some(field_type) = model_attr(field) {
                    cs_aliased_fields.push_str(" \"");
                    cs_aliased_fields.push_str(&name_string);
                    if field_type == "secret" {
                        // FIXME: don't hard-code struct name
                        cs_aliased_fields.push_str("?: SecretString\"");
                    } else {
                        cs_aliased_fields.push_str(": _\"");
                    }
                }
                cs_values.push('$');
                cs_values.push_str(&value_number.to_string());

                value_number += 1;

                cs_setters.push('"');
                cs_setters.push_str(&name.to_string());
                cs_setters.push_str("\" = $");
                cs_setters.push_str(&value_number.to_string());
            }
        }
    });

    // TODO: handle fields wrapped in Option
    // field arguments for queries
    let insert_args = named.iter().filter_map(|field| {
        if let Some(name) = &field.ident {
            if name != "id" {
                if let Some(tokens) = model_attr(field) {
                    if tokens == "enum" {
                        if let Some(field_type) = field_type(&field.ty) {
                            return Some(quote! { &self.#name as &#field_type });
                        }
                    } else if tokens == "secret" {
                        // FIXME: hard-coded struct name
                        return Some(quote! { &self.#name as &Option<SecretString> });
                    } else {
                        return Some(quote! { &self.#name });
                    }
                }
                return Some(quote! { self.#name });
            }
        }
        None
    });
    let update_args = insert_args.clone();

    // queries
    let all_query = format!("SELECT id \"id?\", {cs_aliased_fields} FROM \"{table_name}\"");
    let find_by_id_query =
        format!("SELECT id \"id?\", {cs_aliased_fields} FROM \"{table_name}\" WHERE id = $1");
    let delete_query = format!("DELETE FROM \"{table_name}\" WHERE id = $1");
    let insert_query =
        format!("INSERT INTO \"{table_name}\" ({cs_fields}) VALUES ({cs_values}) RETURNING id");
    let update_query = format!("UPDATE \"{table_name}\" SET {cs_setters} WHERE id = $1");

    quote! {
        impl #name {
            pub async fn find_by_id<'e, E>(executor: E, id: i64) -> Result<Option<Self>, sqlx::Error>
            where
                E: sqlx::PgExecutor<'e>
            {
                sqlx::query_as!(Self, #find_by_id_query, id).fetch_optional(executor).await
            }

            // TODO: add limit and offset
            pub async fn all<'e, E>(executor: E) -> Result<Vec<Self>, sqlx::Error>
            where
                E: sqlx::PgExecutor<'e>
            {
                sqlx::query_as!(Self, #all_query).fetch_all(executor).await
            }

            pub async fn delete<'e, E>(self, executor: E) -> Result<(), sqlx::Error>
            where
                E: sqlx::PgExecutor<'e>
            {
                if let Some(id) = self.id {
                    sqlx::query!(#delete_query, id).execute(executor).await?;
                }
                Ok(())
            }

            pub async fn save<'e, E>(&mut self, executor: E) -> Result<(), sqlx::Error>
            where
                E: sqlx::PgExecutor<'e>
            {
                match self.id {
                    None => {
                        let id = sqlx::query_scalar!(#insert_query, #(#insert_args,)*).fetch_one(executor).await?;
                        self.id = Some(id);
                    }
                    Some(id) => {
                        sqlx::query!(#update_query, id, #(#update_args,)*).execute(executor).await?;
                    }
                }
                Ok(())
            }
        }
    }
    .into()
}

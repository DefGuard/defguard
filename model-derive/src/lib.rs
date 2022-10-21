use proc_macro::TokenStream;
use quote::quote;
use syn::{
    parse::{Parse, ParseStream},
    parse_macro_input, Data, DataStruct, DeriveInput, Field, Fields, FieldsNamed, Ident, Path,
    Type, TypePath,
};

struct Attrs(Ident);

impl Parse for Attrs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let content;
        syn::parenthesized!(content in input);
        Ok(Self(content.parse()?))
    }
}

fn model_attr(f: &Field) -> Option<String> {
    for attr in &f.attrs {
        if let Some(path_seg) = attr.path.segments.first() {
            if path_seg.ident == "model" {
                // FIXME: this is a short-cut that returns tokens with parentheses, e.g. "(enum)".
                return Some(attr.tokens.to_string());
            }
        }
    }
    None
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
    // Find the "table" attribute
    let attribute = ast
        .attrs
        .iter()
        .find(|a| a.path.segments.len() == 1 && a.path.segments[0].ident == "table");
    // Parse table name if attribute is present
    let table_name = match attribute {
        Some(attribute) => {
            let attributes: Attrs =
                syn::parse2(attribute.tokens.clone()).expect("Invalid table attribute");
            attributes.0.to_string()
        }
        _ => name.to_string().to_ascii_lowercase(),
    };

    let fields = if let Data::Struct(DataStruct {
        fields: Fields::Named(FieldsNamed { named, .. }),
        ..
    }) = ast.data
    {
        named
    } else {
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
    fields.iter().for_each(|field| {
        if let Some(name) = &field.ident {
            if name != "id" {
                if add_comma {
                    cs_fields.push_str(", ");
                    cs_aliased_fields.push_str(", ");
                    cs_values.push_str(", ");
                    cs_setters.push_str(", ");
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
                if model_attr(field).is_some() {
                    cs_aliased_fields.push_str(" \"");
                    cs_aliased_fields.push_str(&name_string);
                    cs_aliased_fields.push_str(": _\"");
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
    let insert_args = fields.iter().filter_map(|field| {
        if let Some(name) = &field.ident {
            if name != "id" {
                if let Some(tokens) = model_attr(field) {
                    if tokens == "(enum)" {
                        if let Some(field_type) = field_type(&field.ty) {
                            return Some(quote! { &self.#name as &#field_type });
                        }
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
            pub async fn find_by_id(pool: &DbPool, id: i64) -> Result<Option<Self>, sqlx::Error> {
                sqlx::query_as!(Self, #find_by_id_query, id).fetch_optional(pool).await
            }

            // TODO: add limit and offset
            pub async fn all(pool: &DbPool) -> Result<Vec<Self>, sqlx::Error> {
                sqlx::query_as!(Self, #all_query).fetch_all(pool).await
            }

            pub async fn delete(self, pool: &DbPool) -> Result<(), sqlx::Error> {
                if let Some(id) = self.id {
                    sqlx::query!(#delete_query, id).execute(pool).await?;
                }
                Ok(())
            }

            pub async fn save(&mut self, pool: &DbPool) -> Result<(), sqlx::Error> {
                match self.id {
                    None => {
                        let id = sqlx::query_scalar!(#insert_query, #(#insert_args,)*).fetch_one(pool).await?;
                        self.id = Some(id);
                    }
                    Some(id) => {
                        sqlx::query!(#update_query, id, #(#update_args,)*).execute(pool).await?;
                    }
                }
                Ok(())
            }
        }
    }
    .into()
}

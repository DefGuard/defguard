use proc_macro2::{TokenStream as TokenStream2, TokenTree};
use quote::quote;

use super::*;

/// Parse a `DeriveInput` from source, e.g. `#[table(foo)] struct Foo { id: Id }`.
fn input(src: &str) -> DeriveInput {
    syn::parse_str(src).expect("test input should be a valid derive input")
}

/// Pull the first named field out of `struct S { <src> }`.
fn field(src: &str) -> Field {
    let ast = input(&format!("struct S {{ {src} }}"));
    let Data::Struct(DataStruct {
        fields: Fields::Named(FieldsNamed { named, .. }),
        ..
    }) = ast.data
    else {
        unreachable!("helper always builds a named-field struct");
    };
    named.into_iter().next().expect("one field")
}

fn ty(src: &str) -> Type {
    syn::parse_str(src).expect("test input should be a valid type")
}

fn ident(name: &str) -> Ident {
    syn::parse_str(name).expect("test input should be a valid ident")
}

/// Every string literal in the generated code, in emission order. Since the
/// generated code contains no strings other than the SQL, this recovers the
/// exact queries the macro built.
fn queries(src: &str) -> Vec<String> {
    fn walk(tokens: TokenStream2, out: &mut Vec<String>) {
        for token in tokens {
            match token {
                TokenTree::Group(group) => walk(group.stream(), out),
                TokenTree::Literal(literal) => {
                    let stream = TokenStream2::from(TokenTree::Literal(literal));
                    if let Ok(lit) = syn::parse2::<syn::LitStr>(stream) {
                        out.push(lit.value());
                    }
                }
                _ => {}
            }
        }
    }

    let mut out = Vec::new();
    walk(
        expand(&input(src)).expect("expansion should succeed"),
        &mut out,
    );
    out
}

// Indices into the `queries()` vector, following emission order in `expand`.
const INSERT: usize = 0;
const FIND_BY_ID: usize = 1;
const ALL: usize = 2;
const ALL_PAGINATED: usize = 3;
const DELETE: usize = 4;
const UPDATE: usize = 5;
const COUNT: usize = 6;

#[test]
fn table_name_defaults_to_lowercased_ident() {
    let ast = input("struct WireguardNetwork { id: Id }");
    assert_eq!(
        table_attr(&ast.attrs, &ast.ident).unwrap(),
        "wireguardnetwork"
    );
}

#[test]
fn table_attr_overrides_default_name() {
    let ast = input("#[table(wireguard_network)] struct WireguardNetwork { id: Id }");
    assert_eq!(
        table_attr(&ast.attrs, &ast.ident).unwrap(),
        "wireguard_network"
    );
}

#[test]
fn table_attr_ignores_unrelated_attributes() {
    let ast = input("#[derive(Debug)] #[serde(rename = \"x\")] struct Session { id: Id }");
    assert_eq!(table_attr(&ast.attrs, &ast.ident).unwrap(), "session");
}

#[test]
fn table_attr_rejects_non_ident_property() {
    let ast = input("#[table(schema::session)] struct Session { id: Id }");
    assert_eq!(
        table_attr(&ast.attrs, &ast.ident).unwrap_err().to_string(),
        "unsupported table property"
    );
}

#[test]
fn table_attr_reads_only_the_first_attribute() {
    let ast = input("#[table(first)] #[table(second)] struct Session { id: Id }");
    assert_eq!(table_attr(&ast.attrs, &ast.ident).unwrap(), "first");
}

#[test]
fn table_attr_without_properties_keeps_default() {
    let ast = input("#[table()] struct Session { id: Id }");
    assert_eq!(table_attr(&ast.attrs, &ast.ident).unwrap(), "session");
}

#[test]
fn model_attr_absent_yields_none() {
    assert_eq!(model_attr(&field("name: String")).unwrap(), ModelType::Any);
}

#[test]
fn model_attr_parses_every_supported_property() {
    let cases = [
        ("enum", ModelType::Enum),
        ("ip", ModelType::Ip),
        ("json", ModelType::Json),
        ("option", ModelType::Option),
        ("option_ref", ModelType::OptionRef),
        ("ref", ModelType::Ref),
        ("secret", ModelType::Secret),
    ];

    for (property, expected) in cases {
        let field = field(&format!("#[model({property})] value: T"));
        assert_eq!(
            model_attr(&field).unwrap(),
            expected,
            "#[model({property})]"
        );
    }
}

#[test]
fn model_attr_rejects_unknown_property() {
    let field = field("#[model(bogus)] value: T");
    assert_eq!(
        model_attr(&field).unwrap_err().to_string(),
        "unsupported model property"
    );
}

#[test]
fn model_attr_rejects_non_ident_property() {
    let field = field("#[model(a::b)] value: T");
    assert_eq!(
        model_attr(&field).unwrap_err().to_string(),
        "unsupported model property"
    );
}

#[test]
fn model_attr_rejects_multiple_properties() {
    let field = field("#[model(enum, ref)] value: T");
    assert_eq!(
        model_attr(&field).unwrap_err().to_string(),
        "expected a single model property"
    );
}

#[test]
fn model_attr_ignores_unrelated_attributes() {
    let field = field("#[serde(skip)] #[model(ref)] value: T");
    assert_eq!(model_attr(&field).unwrap(), ModelType::Ref);
}

#[test]
fn field_type_takes_the_last_path_segment() {
    assert_eq!(field_type(&ty("String")).unwrap(), &ident("String"));
    assert_eq!(
        field_type(&ty("std::net::IpAddr")).unwrap(),
        &ident("IpAddr")
    );
    assert_eq!(field_type(&ty("Vec<u8>")).unwrap(), &ident("Vec"));
}

#[test]
fn field_type_rejects_non_path_types() {
    assert!(field_type(&ty("&str")).is_none());
    assert!(field_type(&ty("(u8, u8)")).is_none());
    assert!(field_type(&ty("[u8; 4]")).is_none());
}

#[test]
fn option_field_type_extracts_the_inner_type() {
    assert_eq!(
        option_field_type(&ty("Option<String>")).unwrap(),
        &ident("String")
    );
    assert_eq!(
        option_field_type(&ty("std::option::Option<Foo>")).unwrap(),
        &ident("Foo")
    );
    // Only the outer layer is unwrapped.
    assert_eq!(
        option_field_type(&ty("Option<Vec<u8>>")).unwrap(),
        &ident("Vec")
    );
}

#[test]
fn option_field_type_rejects_non_options() {
    assert!(option_field_type(&ty("String")).is_none());
    assert!(option_field_type(&ty("Option")).is_none());
    assert!(option_field_type(&ty("Option<'a>")).is_none());
    assert!(option_field_type(&ty("&Option<String>")).is_none());
}

#[test]
fn queries_use_the_table_name_and_skip_the_id_field() {
    let queries = queries("#[table(session)] struct Session { id: Id, user_id: Id }");

    assert_eq!(
        queries,
        [
            "INSERT INTO \"session\" (\"user_id\") VALUES ($1) RETURNING id",
            "SELECT id, \"user_id\" FROM \"session\" WHERE id = $1",
            "SELECT id, \"user_id\" FROM \"session\"",
            "SELECT id, \"user_id\" FROM \"session\" LIMIT $1 OFFSET $2",
            "DELETE FROM \"session\" WHERE id = $1",
            "UPDATE \"session\" SET \"user_id\" = $2 WHERE id = $1",
            "SELECT count(*) FROM \"session\"",
        ]
    );
}

#[test]
fn id_is_skipped_wherever_it_is_declared() {
    // `id` need not be the first field.
    let queries = queries("struct Session { user_id: Id, id: Id, state: String }");
    assert_eq!(
        queries[INSERT],
        "INSERT INTO \"session\" (\"user_id\",\"state\") VALUES ($1,$2) RETURNING id"
    );
}

#[test]
fn insert_placeholders_start_at_one_and_update_placeholders_at_two() {
    // The update query binds `id` as $1, so its setters are offset by one.
    let queries = queries("struct T { id: Id, a: A, b: B, c: C }");

    assert_eq!(
        queries[INSERT],
        "INSERT INTO \"t\" (\"a\",\"b\",\"c\") VALUES ($1,$2,$3) RETURNING id"
    );
    assert_eq!(
        queries[UPDATE],
        "UPDATE \"t\" SET \"a\" = $2,\"b\" = $3,\"c\" = $4 WHERE id = $1"
    );
}

#[test]
fn placeholder_numbering_survives_two_digits() {
    let fields = (0..11)
        .map(|index| format!("f{index}: T"))
        .collect::<Vec<_>>()
        .join(", ");
    let queries = queries(&format!("struct T {{ id: Id, {fields} }}"));

    assert!(
        queries[INSERT].ends_with("VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11) RETURNING id"),
        "{}",
        queries[INSERT]
    );
    assert!(
        queries[UPDATE].ends_with("\"f10\" = $12 WHERE id = $1"),
        "{}",
        queries[UPDATE]
    );
}

#[test]
fn each_model_type_produces_its_own_column_alias() {
    let queries = queries(
        "struct T {
            id: Id,
            plain: String,
            #[model(enum)] kind: Kind,
            #[model(ref)] name: String,
            #[model(option)] count: Option<i64>,
            #[model(option_ref)] note: Option<String>,
            #[model(ip)] address: IpAddr,
            #[model(secret)] token: Option<SecretString>,
        }",
    );

    assert_eq!(
        queries[ALL],
        "SELECT id, \"plain\",\
            \"kind\" \"kind: _\",\
            \"name\" \"name: _\",\
            \"count\" \"count?: _\",\
            \"note\" \"note?: _\",\
            \"address\" \"address: IpAddr\",\
            \"token\" \"token?: SecretString\" \
            FROM \"t\""
    );
    assert_eq!(
        queries[INSERT],
        "INSERT INTO \"t\" (\"plain\",\"kind\",\"name\",\"count\",\"note\",\"address\",\"token\") \
             VALUES ($1,$2,$3,$4,$5,$6,$7) RETURNING id"
    );
}

#[test]
fn json_type_emits_wildcard_select_alias() {
    let queries = queries("struct T { id: Id, #[model(json)] data: Json<Foo> }");
    assert_eq!(queries[ALL], "SELECT id, \"data\" \"data: _\" FROM \"t\"");
    assert_eq!(
        queries[INSERT],
        "INSERT INTO \"t\" (\"data\") VALUES ($1) RETURNING id"
    );
}

#[test]
fn derived_queries_share_the_select_prefix() {
    let queries = queries("struct T { id: Id, a: A }");

    assert!(queries[FIND_BY_ID].starts_with(&queries[ALL]));
    assert!(queries[ALL_PAGINATED].starts_with(&queries[ALL]));
    assert_eq!(queries[DELETE], "DELETE FROM \"t\" WHERE id = $1");
    assert_eq!(queries[COUNT], "SELECT count(*) FROM \"t\"");
}

#[test]
fn a_struct_with_only_an_id_produces_empty_column_lists() {
    let queries = queries("struct T { id: Id }");

    assert_eq!(queries[ALL], "SELECT id,  FROM \"t\"");
    assert_eq!(
        queries[INSERT],
        "INSERT INTO \"t\" () VALUES () RETURNING id"
    );
    assert_eq!(queries[UPDATE], "UPDATE \"t\" SET  WHERE id = $1");
}

/// Isolates the bind-argument expression each `ModelType` produces, by
/// locating the `query_scalar!(<INSERT ...>, <args>,)` invocation in the
/// generated tokens and returning everything after the query literal.
fn bind_arg(field: &str) -> String {
    /// Tokens following the query literal of the `INSERT` macro invocation.
    fn insert_args(tokens: TokenStream2) -> Option<TokenStream2> {
        for token in tokens {
            let TokenTree::Group(group) = token else {
                continue;
            };

            let mut inner = group.stream().into_iter();
            if let Some(TokenTree::Literal(literal)) = inner.next() {
                let stream = TokenStream2::from(TokenTree::Literal(literal));
                if syn::parse2::<syn::LitStr>(stream)
                    .is_ok_and(|lit| lit.value().starts_with("INSERT INTO"))
                {
                    return Some(inner.skip(1).collect());
                }
            }

            if let Some(args) = insert_args(group.stream()) {
                return Some(args);
            }
        }
        None
    }

    let generated = expand(&input(&format!("struct T {{ id: Id, {field} }}"))).unwrap();
    let mut args = insert_args(generated)
        .expect("generated code contains an INSERT invocation")
        .into_iter()
        .collect::<Vec<_>>();

    // Bind args are emitted with a trailing comma.
    if matches!(args.last(), Some(TokenTree::Punct(punct)) if punct.as_char() == ',') {
        args.pop();
    }

    args.into_iter().collect::<TokenStream2>().to_string()
}

#[test]
fn bind_args_cast_according_to_model_type() {
    assert_eq!(bind_arg("value: String"), quote!(self.value).to_string());
    assert_eq!(
        bind_arg("#[model(ref)] value: String"),
        quote!(&self.value).to_string()
    );
    assert_eq!(
        bind_arg("#[model(enum)] value: Kind"),
        quote!(&self.value as &Kind).to_string()
    );
    assert_eq!(
        bind_arg("#[model(option)] value: Option<i64>"),
        quote!(&self.value as &Option<i64>).to_string()
    );
    assert_eq!(
        bind_arg("#[model(option_ref)] value: Option<String>"),
        quote!(self.value.as_deref()).to_string()
    );
    assert_eq!(
        bind_arg("#[model(ip)] value: IpAddr"),
        quote!(&self.value as &IpAddr).to_string()
    );
    assert_eq!(
        bind_arg("#[model(list)] value: Vec<Kind>"),
        quote!(&self.value as &Vec<Kind>).to_string()
    );
    assert_eq!(
        bind_arg("#[model(json)] value: Json<Foo>"),
        quote!(&self.value as &Json<Foo>).to_string()
    );
    assert_eq!(
        bind_arg("#[model(secret)] value: Option<SecretString>"),
        quote!(&self.value as &Option<SecretString>).to_string()
    );
}

#[test]
fn enum_and_option_casts_fall_back_when_the_type_is_not_a_path() {
    assert_eq!(
        bind_arg("#[model(enum)] value: (u8, u8)"),
        quote!(&self.value).to_string()
    );
    assert_eq!(
        bind_arg("#[model(option)] value: String"),
        quote!(&self.value).to_string()
    );
}

#[test]
fn enum_cast_uses_the_last_path_segment() {
    assert_eq!(
        bind_arg("#[model(enum)] value: crate::db::Kind"),
        quote!(&self.value as &Kind).to_string()
    );
}

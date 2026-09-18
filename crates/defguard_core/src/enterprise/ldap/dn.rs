//! Distinguished name string handling, as defined by RFC 4514.

/// Returns the byte index of the first unescaped separator.
#[must_use]
pub(crate) fn find_unescaped(input: &str, separator: u8) -> Option<usize> {
    debug_assert!(separator.is_ascii());

    let bytes = input.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        match bytes[index] {
            b'\\' => index = bytes.len().min(index.saturating_add(2)),
            byte if byte == separator => return Some(index),
            _ => index += 1,
        }
    }

    None
}

/// Resolves character and hex-pair escapes in one LDAP attribute value.
#[must_use]
pub(crate) fn unescape_value(value: &str) -> String {
    if !value.contains('\\') {
        return value.to_owned();
    }

    let bytes = value.as_bytes();
    let mut unescaped = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] != b'\\' {
            unescaped.push(bytes[index]);
            index += 1;
            continue;
        }

        if let Some(byte) = hexpair(bytes, index + 1) {
            unescaped.push(byte);
            index += 3;
        } else if let Some(&escaped) = bytes.get(index + 1) {
            unescaped.push(escaped);
            index += 2;
        } else {
            unescaped.push(b'\\');
            index += 1;
        }
    }

    String::from_utf8_lossy(&unescaped).into_owned()
}

fn hexpair(bytes: &[u8], index: usize) -> Option<u8> {
    let high = hex_value(*bytes.get(index)?)?;
    let low = hex_value(*bytes.get(index + 1)?)?;
    Some((high << 4) | low)
}

fn hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

/// Builds a comparison key for a DN without choosing an escape spelling.
#[must_use]
pub(crate) fn canonical_dn(dn: &str) -> String {
    if dn.is_empty() {
        return String::new();
    }

    let mut canonical = String::with_capacity(dn.len());
    let mut rest = dn;

    loop {
        match find_unescaped(rest, b',') {
            Some(comma) => {
                push_component(&mut canonical, &canonical_component(&rest[..comma]));
                rest = &rest[comma + 1..];
            }
            None => {
                push_component(&mut canonical, &canonical_component(rest));
                return canonical;
            }
        }
    }
}

fn push_component(canonical: &mut String, component: &str) {
    // Values are unescaped before components are joined, so a comma cannot delimit them safely.
    canonical.push_str(&component.len().to_string());
    canonical.push(':');
    canonical.push_str(component);
}

fn canonical_component(component: &str) -> String {
    let component = trim_unescaped(component);
    let Some(equals) = find_unescaped(component, b'=') else {
        return unescape_value(component).to_lowercase();
    };

    let attribute = trim_unescaped(&component[..equals]).to_lowercase();
    // Keep a multi-valued RDN opaque. LDAP user DNs do not use it as a structure we need to match.
    let value = unescape_value(trim_unescaped(&component[equals + 1..])).to_lowercase();
    format!("{attribute}={value}")
}

fn trim_unescaped(value: &str) -> &str {
    let bytes = value.as_bytes();
    let mut start = 0;
    while start < bytes.len() && bytes[start].is_ascii_whitespace() {
        start += 1;
    }

    let mut end = bytes.len();
    while end > start && bytes[end - 1].is_ascii_whitespace() && !is_escaped(bytes, end - 1) {
        end -= 1;
    }

    &value[start..end]
}

fn is_escaped(bytes: &[u8], index: usize) -> bool {
    let mut backslashes = 0;
    let mut index = index;
    while index > 0 && bytes[index - 1] == b'\\' {
        backslashes += 1;
        index -= 1;
    }

    backslashes % 2 == 1
}

#[cfg(test)]
mod tests {
    use super::{canonical_dn, find_unescaped, unescape_value};

    #[test]
    fn test_find_unescaped_skips_escaped_separators() {
        assert_eq!(find_unescaped("cn=user,dc=example", b','), Some(7));
        assert_eq!(find_unescaped("cn=user,dc=example", b'='), Some(2));
        assert_eq!(find_unescaped(r"cn=Doe\, John,ou=users", b','), Some(13));
        assert_eq!(find_unescaped(r"cn=Doe\2c John,ou=users", b','), Some(14));
        assert_eq!(find_unescaped(r"cn=Doe\5c,ou=users", b','), Some(9));
        assert_eq!(find_unescaped(r"cn=Doe\,John", b','), None);
        assert_eq!(find_unescaped("", b','), None);
        assert_eq!(find_unescaped("cn=Michał,dc=example", b','), Some(10));
    }

    #[test]
    fn test_unescape_value_resolves_both_escape_forms() {
        assert_eq!(unescape_value("plain value"), "plain value");
        assert_eq!(unescape_value(r"Doe\, John"), "Doe, John");
        assert_eq!(unescape_value(r"Doe\2c John"), "Doe, John");
        assert_eq!(unescape_value(r"Doe\2C John"), "Doe, John");
        assert_eq!(unescape_value(r#"a\+b\"c\\d\#e"#), "a+b\"c\\d#e");
        assert_eq!(unescape_value(r"edge\ "), "edge ");
        assert_eq!(unescape_value(r"Micha\c5\82"), "Michał");
    }

    #[test]
    fn test_canonical_dn_matches_equivalent_dns() {
        assert_eq!(
            canonical_dn(r"CN=Example\, Person - euser,OU=Members,DC=example,DC=com"),
            canonical_dn(r"cn=Example\2c Person - euser,ou=members,dc=example,dc=com")
        );
        assert_eq!(
            canonical_dn("cn=user, ou=users , dc=example"),
            canonical_dn("CN=user,ou=users,dc=example")
        );
        assert_ne!(
            canonical_dn(r"cn=foo\,bar\=qux,dc=x"),
            canonical_dn("cn=foo,bar=qux,dc=x")
        );
        assert_ne!(
            canonical_dn(r"cn=user\ ,ou=users"),
            canonical_dn("cn=user,ou=users")
        );
        assert_eq!(
            canonical_dn("CN=Smith+UID=123,DC=example"),
            canonical_dn("cn=smith+uid=123,dc=example")
        );
    }
}

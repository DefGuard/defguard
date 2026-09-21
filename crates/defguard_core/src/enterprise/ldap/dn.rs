//! Distinguished name string handling, as defined by RFC 4514.

/// Returns the byte index of the first `separator` that is not escaped.
///
/// `separator` must be ASCII, which every DN separator is. UTF-8 never uses a byte below 0x80
/// inside a multi-byte character, so a byte-wise hit is always a real separator.
#[must_use]
pub(crate) fn find_unescaped_separator(input: &str, separator: u8) -> Option<usize> {
    let bytes = input.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        match bytes[index] {
            // Skip the escape and whatever it protects, which may itself be a separator.
            b'\\' => index = bytes.len().min(index.saturating_add(2)),
            byte if byte == separator => return Some(index),
            _ => index += 1,
        }
    }

    None
}

/// Decodes the escapes in one RDN value: `Doe\, John` and `Doe\2c John` both give `Doe, John`.
///
/// Returns `None` for a value that is malformed or decodes to bytes that are not valid UTF-8. This
/// text is stored as a user's `ldap_rdn`, so a name Defguard cannot hold faithfully is refused.
#[must_use]
pub(crate) fn unescape_value(value: &str) -> Option<String> {
    String::from_utf8(unescape_value_bytes(value)?).ok()
}

/// Decodes to bytes, because a hex pair can carry one byte of a multi-byte character, as the
/// `\c5\82` of `Micha\c5\82` does.
fn unescape_value_bytes(value: &str) -> Option<Vec<u8>> {
    let bytes = value.as_bytes();
    let Some(first_escape) = bytes.iter().position(|&byte| byte == b'\\') else {
        return Some(bytes.to_vec());
    };

    let mut unescaped = Vec::with_capacity(bytes.len());
    unescaped.extend_from_slice(&bytes[..first_escape]);

    let mut index = first_escape;
    while index < bytes.len() {
        if bytes[index] != b'\\' {
            let run_end = bytes[index..]
                .iter()
                .position(|&byte| byte == b'\\')
                .map_or(bytes.len(), |offset| index + offset);
            unescaped.extend_from_slice(&bytes[index..run_end]);
            index = run_end;
            continue;
        }

        if let Some(byte) = hex_pair(bytes.get(index + 1..index + 3)) {
            unescaped.push(byte);
            index += 3;
        } else {
            // Every character RFC 4514 escapes this way is ASCII, so taking one byte cannot cut a
            // multi-byte character in half. Nothing left to protect means the value is malformed.
            unescaped.push(*bytes.get(index + 1)?);
            index += 2;
        }
    }

    Some(unescaped)
}

/// Reads the two hex digits of an escape such as the `c5` of `\c5`.
fn hex_pair(digits: Option<&[u8]>) -> Option<u8> {
    let digits = digits?;
    // `from_str_radix` would accept a leading sign, which would read `\+a` as a hex pair.
    if !digits.iter().all(u8::is_ascii_hexdigit) {
        return None;
    }

    u8::from_str_radix(str::from_utf8(digits).ok()?, 16).ok()
}

#[cfg(test)]
mod tests {
    use super::{find_unescaped_separator, unescape_value};

    #[test]
    fn test_find_unescaped_separator_skips_escaped_separators() {
        assert_eq!(
            find_unescaped_separator("cn=user,dc=example", b','),
            Some(7)
        );
        assert_eq!(
            find_unescaped_separator("cn=user,dc=example", b'='),
            Some(2)
        );
        assert_eq!(
            find_unescaped_separator(r"cn=Doe\, John,ou=users", b','),
            Some(13)
        );
        assert_eq!(
            find_unescaped_separator(r"cn=Doe\2c John,ou=users", b','),
            Some(14)
        );
        assert_eq!(
            find_unescaped_separator(r"cn=Doe\5c,ou=users", b','),
            Some(9)
        );
        assert_eq!(find_unescaped_separator(r"cn=Doe\,John", b','), None);
        assert_eq!(find_unescaped_separator("", b','), None);
        assert_eq!(
            find_unescaped_separator("cn=Michał,dc=example", b','),
            Some(10)
        );
    }

    #[test]
    fn test_unescape_value_resolves_both_escape_forms() {
        assert_eq!(
            unescape_value("plain value").as_deref(),
            Some("plain value")
        );
        assert_eq!(unescape_value(r"Doe\, John").as_deref(), Some("Doe, John"));
        assert_eq!(unescape_value(r"Doe\2c John").as_deref(), Some("Doe, John"));
        assert_eq!(unescape_value(r"Doe\2C John").as_deref(), Some("Doe, John"));
        assert_eq!(
            unescape_value(r#"a\+b\"c\\d\#e"#).as_deref(),
            Some("a+b\"c\\d#e")
        );
        assert_eq!(unescape_value(r"edge\ ").as_deref(), Some("edge "));
        assert_eq!(unescape_value(r"Micha\c5\82").as_deref(), Some("Michał"));
        // A sign is not a hex digit, so this is the character escape for a plus.
        assert_eq!(unescape_value(r"a\+5").as_deref(), Some("a+5"));
    }

    #[test]
    fn test_unescape_value_rejects_malformed_and_invalid_utf8() {
        assert_eq!(unescape_value(r"a\ff"), None);
        assert_eq!(unescape_value(r"Micha\c5"), None);
        assert_eq!(unescape_value(r"trailing\"), None);
    }
}

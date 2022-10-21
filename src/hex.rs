use std::{
    error::Error,
    fmt::{Display, Formatter, Result as FmtResult},
};

#[derive(Debug, PartialEq)]
pub enum HexError {
    InvalidCharacter(u8),
    InvalidStringLength(usize),
}

impl Error for HexError {}

impl Display for HexError {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        match self {
            Self::InvalidCharacter(char) => {
                write!(f, "Invalid character {char}")
            }
            Self::InvalidStringLength(length) => write!(f, "Invalid string length {length}"),
        }
    }
}

pub fn hex_decode<T: AsRef<[u8]>>(hex: T) -> Result<Vec<u8>, HexError> {
    let mut hex = hex.as_ref();
    let mut length = hex.len();
    if length == 0 || length % 2 != 0 {
        return Err(HexError::InvalidStringLength(length));
    }

    if length > 2 && hex[0] == b'0' && (hex[1] == b'x' || hex[1] == b'X') {
        length -= 2;
        hex = &hex[2..];
    }

    let hex_value = |char: u8| -> Result<u8, HexError> {
        match char {
            b'A'..=b'F' => Ok(char - b'A' + 10),
            b'a'..=b'f' => Ok(char - b'a' + 10),
            b'0'..=b'9' => Ok(char - b'0'),
            _ => Err(HexError::InvalidCharacter(char)),
        }
    };

    let mut bytes = Vec::with_capacity(length / 2);
    for chunk in hex.chunks(2) {
        let msd = hex_value(chunk[0])?;
        let lsd = hex_value(chunk[1])?;
        bytes.push(msd << 4 | lsd);
    }

    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[std::prelude::v1::test]
    fn test_hex_decode() {
        assert_eq!(hex_decode("deadf00d"), Ok(vec![0xde, 0xad, 0xf0, 0x0d]));
        assert_eq!(hex_decode("0Xdeadf00d"), Ok(vec![0xde, 0xad, 0xf0, 0x0d]));
        assert_eq!(hex_decode("0xdeadf00d"), Ok(vec![0xde, 0xad, 0xf0, 0x0d]));

        assert_eq!(hex_decode(""), Err(HexError::InvalidStringLength(0)));
        assert_eq!(hex_decode("f00"), Err(HexError::InvalidStringLength(3)));
        assert_eq!(hex_decode("0xf00"), Err(HexError::InvalidStringLength(5)));

        assert_eq!(hex_decode("0x"), Err(HexError::InvalidCharacter(120)));
        assert_eq!(hex_decode("0X"), Err(HexError::InvalidCharacter(88)));
    }
}

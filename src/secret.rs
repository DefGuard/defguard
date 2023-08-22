use std::{io::Write, str::FromStr};

use secrecy::{ExposeSecret, Secret};
use serde::{Deserialize, Serialize};
use sqlx::{
    database::{HasArguments, HasValueRef},
    encode::IsNull,
    postgres::PgTypeInfo,
    Decode, Encode, Postgres, Type,
};

/// Wrapper for secrecy Secret struct which implements sqlx Postgres
#[derive(Clone, Deserialize, Debug)]
pub struct SecretString(Secret<String>);

impl FromStr for SecretString {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(SecretString(Secret::from_str(s).unwrap()))
    }
}

impl SecretString {
    pub fn expose_secret(&self) -> &str {
        self.0.expose_secret()
    }
}

impl Serialize for SecretString {
    fn serialize<S>(&self, ser: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        ser.serialize_str(self.0.expose_secret())
    }
}

impl<'r> Decode<'r, Postgres> for SecretString {
    fn decode(
        value: <Postgres as HasValueRef<'r>>::ValueRef,
    ) -> std::result::Result<SecretString, Box<(dyn std::error::Error + Send + Sync + 'static)>>
    {
        let value = <&str as Decode<Postgres>>::decode(value)?;
        let secret = SecretString::from_str(value).unwrap();
        Ok(secret)
    }
}

impl<'r> Encode<'r, Postgres> for SecretString {
    fn encode_by_ref(&self, buf: &mut <Postgres as HasArguments<'r>>::ArgumentBuffer) -> IsNull {
        match buf.write(self.expose_secret().as_bytes()) {
            Ok(_) => IsNull::No,
            Err(_) => IsNull::Yes,
        }
    }
}

impl Type<Postgres> for SecretString {
    fn type_info() -> PgTypeInfo {
        PgTypeInfo::with_name("SecretString")
    }
}

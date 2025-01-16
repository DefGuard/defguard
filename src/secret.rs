use std::{convert::Infallible, error::Error, str::FromStr};

use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Serialize};
use sqlx::{
    encode::IsNull,
    postgres::{PgArgumentBuffer, PgTypeInfo, PgValueRef},
    Decode, Encode, Postgres, Type,
};

/// Wrapper for secrecy `SecretString` struct which implements sqlx traits.
#[derive(Clone, Debug, Deserialize)]
pub struct SecretStringWrapper(SecretString);

impl SecretStringWrapper {
    #[must_use]
    pub fn expose_secret(&self) -> &str {
        self.0.expose_secret()
    }
}

impl FromStr for SecretStringWrapper {
    type Err = Infallible;

    fn from_str(src: &str) -> Result<Self, Self::Err> {
        Ok(Self(SecretString::from(src)))
    }
}

impl Serialize for SecretStringWrapper {
    fn serialize<S>(&self, ser: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        ser.serialize_str(self.0.expose_secret())
    }
}

impl Decode<'_, Postgres> for SecretStringWrapper {
    fn decode(value: PgValueRef<'_>) -> Result<Self, Box<dyn Error + Send + Sync>> {
        <String as Decode<'_, Postgres>>::decode(value).map(|v| Self(SecretString::from(v)))
    }
}

impl Encode<'_, Postgres> for SecretStringWrapper {
    fn encode_by_ref(
        &self,
        buf: &mut PgArgumentBuffer,
    ) -> Result<IsNull, Box<dyn Error + Sync + Send>> {
        <&str as Encode<Postgres>>::encode_by_ref(&self.0.expose_secret(), buf)
    }
}

impl Type<Postgres> for SecretStringWrapper {
    fn type_info() -> PgTypeInfo {
        <String as ::sqlx::Type<Postgres>>::type_info()
    }

    fn compatible(ty: &PgTypeInfo) -> bool {
        <String as ::sqlx::Type<Postgres>>::compatible(ty)
    }
}

impl PartialEq for SecretStringWrapper {
    fn eq(&self, other: &Self) -> bool {
        self.0.expose_secret() == other.0.expose_secret()
    }
}

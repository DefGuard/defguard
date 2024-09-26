use std::{convert::Infallible, error::Error, str::FromStr};

use secrecy::{ExposeSecret, Secret};
use serde::{Deserialize, Serialize};
use sqlx::{encode::IsNull, Database, Decode, Encode, Type};

/// Wrapper for secrecy Secret struct which implements sqlx Postgres
#[derive(Clone, Deserialize, Debug)]
pub struct SecretString(Secret<String>);

impl SecretString {
    #[must_use]
    pub fn expose_secret(&self) -> &str {
        self.0.expose_secret()
    }
}

impl FromStr for SecretString {
    type Err = Infallible;

    fn from_str(src: &str) -> Result<Self, Self::Err> {
        Ok(Self(Secret::new(src.to_string())))
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

impl<'q, DB: Database> Decode<'q, DB> for SecretString
where
    String: Decode<'q, DB>,
{
    fn decode(value: <DB as Database>::ValueRef<'q>) -> Result<Self, Box<dyn Error + Send + Sync>> {
        <String as Decode<'q, DB>>::decode(value).map(|v| Self(Secret::from(v)))
    }
}

impl<'q, DB: Database> Encode<'q, DB> for SecretString
where
    String: Encode<'q, DB>,
{
    fn encode_by_ref(
        &self,
        buf: &mut <DB as Database>::ArgumentBuffer<'q>,
    ) -> Result<IsNull, Box<dyn Error + Sync + Send>> {
        <String as Encode<'q, DB>>::encode_by_ref(self.0.expose_secret(), buf)
    }

    fn produces(&self) -> Option<DB::TypeInfo> {
        <String as Encode<'q, DB>>::produces(self.0.expose_secret())
    }

    fn size_hint(&self) -> usize {
        <String as Encode<'q, DB>>::size_hint(self.0.expose_secret())
    }
}

impl<DB: Database> Type<DB> for SecretString
where
    String: Type<DB>,
{
    fn type_info() -> DB::TypeInfo {
        <String as ::sqlx::Type<DB>>::type_info()
    }

    fn compatible(ty: &DB::TypeInfo) -> bool {
        <String as ::sqlx::Type<DB>>::compatible(ty)
    }
}

impl PartialEq for SecretString {
    fn eq(&self, other: &Self) -> bool {
        self.0.expose_secret() == other.0.expose_secret()
    }
}

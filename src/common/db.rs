use sqlx;

use sqlx::Database;
use sqlx::Decode;
use sqlx::Type;

//static MIG: Migrator = sqlx::migrate!();

//static SQLITE: Migrator = sqlx::migrate!("./migrate/");

use super::ModType;
use super::Stability;
use sqlx::sqlite::SqliteArgumentValue;
use sqlx::Encode;
use sqlx::Sqlite;
use std::borrow::Cow;

impl<'q> Encode<'q, Sqlite> for Stability {
    fn encode_by_ref(
        &self,
        buf: &mut <Sqlite as sqlx::database::HasArguments<'q>>::ArgumentBuffer,
    ) -> sqlx::encode::IsNull {
        buf.push(SqliteArgumentValue::Text(Cow::Borrowed(
            match self {
                Self::Stable => "stable",
                Self::RC => "rc",
                Self::Nightly => "nightly",
            }
            .clone(),
        )));

        sqlx::encode::IsNull::No
    }

    fn encode(
        self,
        buf: &mut <Sqlite as sqlx::database::HasArguments<'q>>::ArgumentBuffer,
    ) -> sqlx::encode::IsNull {
        buf.push(SqliteArgumentValue::Text(Cow::Borrowed(
            match self {
                Self::Stable => "stable",
                Self::RC => "rc",
                Self::Nightly => "nightly",
            }
            .clone(),
        )));

        sqlx::encode::IsNull::No
    }
}

impl<'r, DB: Database> Decode<'r, DB> for Stability
where
    &'r str: Decode<'r, DB>,
{
    fn decode(
        value: <DB as sqlx::database::HasValueRef<'r>>::ValueRef,
    ) -> Result<Self, sqlx::error::BoxDynError> {
        let value = <&str as Decode<DB>>::decode(value)?;
        match value {
            "stable" => Ok(Self::Stable),
            "rc" => Ok(Self::RC),
            "nightly" => Ok(Self::Nightly),
            x => bail!(x),
        }
    }
}

impl Type<Sqlite> for Stability {
    fn type_info() -> <Sqlite as sqlx::Database>::TypeInfo {
        <&str as Type<Sqlite>>::type_info()
    }
}

impl<'q> Encode<'q, Sqlite> for ModType {
    fn encode_by_ref(
        &self,
        buf: &mut <Sqlite as sqlx::database::HasArguments<'q>>::ArgumentBuffer,
    ) -> sqlx::encode::IsNull {
        buf.push(SqliteArgumentValue::Text(Cow::Borrowed(
            match self {
                Self::Mod => "mod",
                Self::TC => "tc",
                Self::Engine => "engine",
            }
            .clone(),
        )));

        sqlx::encode::IsNull::No
    }

    fn encode(
        self,
        buf: &mut <Sqlite as sqlx::database::HasArguments<'q>>::ArgumentBuffer,
    ) -> sqlx::encode::IsNull {
        buf.push(SqliteArgumentValue::Text(Cow::Borrowed(
            match self {
                Self::Mod => "mod",
                Self::TC => "tc",
                Self::Engine => "engine",
            }
            .clone(),
        )));

        sqlx::encode::IsNull::No
    }
}

impl<'r, DB: Database> Decode<'r, DB> for ModType
where
    &'r str: Decode<'r, DB>,
{
    fn decode(
        value: <DB as sqlx::database::HasValueRef<'r>>::ValueRef,
    ) -> Result<Self, sqlx::error::BoxDynError> {
        let value = <&str as Decode<DB>>::decode(value)?;
        match value {
            "mod" => Ok(Self::Mod),
            "tc" => Ok(Self::TC),
            "engine" => Ok(Self::Engine),
            x => bail!(x),
        }
    }
}

impl Type<Sqlite> for ModType {
    fn type_info() -> <Sqlite as sqlx::Database>::TypeInfo {
        <&str as Type<Sqlite>>::type_info()
    }
}

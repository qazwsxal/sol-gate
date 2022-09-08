use std::{error::Error, path::PathBuf};

use super::structs::FSNMod;
use sqlx::{
    migrate::{MigrateError, Migrator},
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions},
    SqlitePool, Transaction,
};

static MIG: Migrator = sqlx::migrate!("src/fsnebula/migrations");

pub async fn init(sqlitepath: PathBuf) -> Result<SqlitePool, MigrateError> {
    let c_opts = SqliteConnectOptions::new()
        .filename(sqlitepath)
        .foreign_keys(true)
        .journal_mode(SqliteJournalMode::Wal)
        .create_if_missing(true);
    // Specifiy higher max connections, we're using Wal, so writes don't lock reads.
    let pool = SqlitePoolOptions::new()
        .max_connections(64)
        .connect_lazy_with(c_opts);

    MIG.run(&pool).await?;

    Ok(pool)
}
pub(crate) async fn update_mods(
    fsnmod: &FSNMod,
    tx: &mut Transaction<'_, sqlx::Sqlite>,
) -> Result<(), Box<dyn Error>> {
    sqlx::query_file!(
        "src/fsnebula/queries/update/mods.sql",
        fsnmod.id,
        fsnmod.title,
        fsnmod.version,
        fsnmod.private,
        fsnmod.parent,
        fsnmod.description,
        fsnmod.logo,
        fsnmod.tile,
        fsnmod.banner,
        fsnmod.notes,
        fsnmod.first_release,
        fsnmod.last_update,
        fsnmod.cmdline,
        fsnmod.mod_type,
    )
    .execute(tx)
    .await?;
    Ok(())
}

pub(crate) async fn update_link(
    fsnmod: &FSNMod,
    linktype: &str,
    link: &str,
    tx: &mut Transaction<'_, sqlx::Sqlite>,
) -> Result<(), Box<dyn Error>> {
    sqlx::query_file!(
        "src/fsnebula/queries/update/links.sql",
        fsnmod.id,
        fsnmod.version,
        linktype,
        link
    )
    .execute(tx)
    .await?;
    Ok(())
}

pub(crate) async fn update_mod_flags(
    fsnmod: &FSNMod,
    dep: &str,
    tx: &mut Transaction<'_, sqlx::Sqlite>,
) -> Result<(), Box<dyn Error>> {
    sqlx::query_file!(
        "src/fsnebula/queries/update/mod_flags.sql",
        fsnmod.id,
        fsnmod.version,
        dep,
    )
    .execute(tx)
    .await?;
    Ok(())
}

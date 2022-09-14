use std::{error::Error, path::PathBuf};

use super::structs::{FSNChecksum, FSNMod};
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

pub(crate) async fn commit_mod(
    fsn_pool: &sqlx::Pool<sqlx::Sqlite>,
    fsnmod: FSNMod,
) -> Result<(), sqlx::Error> {
    let mut tx = fsn_pool.begin().await?;
    update_mods(&fsnmod, &mut tx).await?;
    if let Some(stab) = &fsnmod.stability {
        sqlx::query!(
            "INSERT OR IGNORE INTO mods_stab (`stab`, `id`, `version`)
                    VALUES (?1, ?2, ?3)",
            stab,
            fsnmod.id,
            fsnmod.version
        )
        .execute(&mut tx)
        .await?;
    }
    for screen in fsnmod.screenshots.iter() {
        update_link(&fsnmod, "screenshot", screen, &mut tx).await?;
    }
    for attach in fsnmod.attachments.iter() {
        update_link(&fsnmod, "attachment", attach, &mut tx).await?;
    }
    if let Some(thread) = &fsnmod.release_thread {
        update_link(&fsnmod, "thread", thread, &mut tx).await?;
    }
    for vid in fsnmod.videos.iter() {
        update_link(&fsnmod, "videos", vid, &mut tx).await?;
    }
    for dep in fsnmod.mod_flag.iter() {
        update_mod_flags(&fsnmod, dep, &mut tx).await?;
    }
    for package in fsnmod.packages {
        let p_id = sqlx::query_file!(
            "src/fsnebula/queries/update/packages.sql",
            fsnmod.id,
            fsnmod.version,
            package.name,
            package.notes,
            package.status,
            package.environment,
            package.folder,
            package.is_vp
        )
        .fetch_one(&mut tx)
        .await?
        .p_id;

        for zipfile in &package.files {
            sqlx::query_file!(
                "src/fsnebula/queries/update/zipfiles.sql",
                p_id,
                zipfile.filename,
                zipfile.dest,
                zipfile.filesize,
            )
            .execute(&mut tx)
            .await?;
        }
        for dep in package.dependencies {
            let dep_id = sqlx::query!(
                "INSERT INTO pak_dep (p_id, version, dep_mod_id)
                        VALUES (?1, ?2, ?3)
                        RETURNING id
                        ",
                p_id,
                dep.version,
                dep.id
            )
            .fetch_one(&mut tx)
            .await?
            .id;

            for dep_pak in dep.packages {
                sqlx::query!(
                    "INSERT into dep_pak (dep_id, name) \
                            VALUES (?1, ?2);",
                    dep_id,
                    dep_pak
                )
                .execute(&mut tx)
                .await?;
            }
        }

        for modfile in package.filelist {
            let FSNChecksum::SHA256(hash) = modfile.checksum.clone();

            let zip_id: i64 = sqlx::query!(
                "SELECT id FROM zipfiles WHERE (p_id, filename) == (?1, ?2)",
                p_id,
                modfile.archive
            )
            .fetch_one(&mut tx)
            .await?
            .id;

            sqlx::query!(
                "INSERT OR IGNORE INTO files (f_path, zip_id, h_val) \
                        VALUES (?1, ?2, ?3)",
                modfile.filename,
                zip_id,
                hash,
            )
            .execute(&mut tx)
            .await?;
        }
    }
    tx.commit().await?;
    Ok(())
}

pub(crate) async fn update_mods(
    fsnmod: &FSNMod,
    tx: &mut Transaction<'_, sqlx::Sqlite>,
) -> Result<(), sqlx::Error> {
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
) -> Result<(), sqlx::Error> {
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
) -> Result<(), sqlx::Error> {
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

use super::structs::FSNRelType;
use crate::db;
use crate::fsnebula::structs::{FSNDependency, FSNMod, FSNPackage};
use hash_hasher::HashedSet;
use sqlx::Sqlite;
use sqlx::{query_builder::QueryBuilder, sqlite::SqliteQueryResult, Row, Transaction};
use std::{
    collections::{HashMap, HashSet},
    iter::{zip, FromIterator},
};

pub(crate) async fn commit_mods(
    fsn_pool: &sqlx::Pool<sqlx::Sqlite>,
    fsnmods: Vec<FSNMod>,
) -> Result<(), sqlx::Error> {
    let mut tx = fsn_pool.begin().await?;

    let mut name_set = HashSet::<String>::from_iter(fsnmods.iter().map(|m| m.id.clone()));

    name_set.extend(fsnmods.iter().filter_map(|m| m.parent.clone()));

    name_set.extend(fsnmods.iter().flat_map(|m| m.mod_flag.clone()));

    let names = name_set.into_iter().collect::<Vec<String>>();

    db::queries::add_release_names(&names, &mut tx).await?;

    let rel_ids: Vec<i64> = add_fsn_releases(&fsnmods, &mut tx).await?;

    let zipped_vals = zip(rel_ids, fsnmods)
        .into_iter()
        .collect::<Vec<(i64, FSNMod)>>();
    // We don't need any data from these, so we can run them as seperate tasks.
    for (rel_id, fsnmod) in zipped_vals.iter() {
        add_fsn_links(fsnmod, rel_id, &mut tx).await?;
    }

    let packages = zipped_vals
        .iter()
        .flat_map(|(i, m)| m.packages.iter().map(|p| (i.clone(), p.clone())))
        .collect::<Vec<(i64, FSNPackage)>>();
    let pak_ids: Vec<i64> = add_fsn_packages(&packages, &mut tx).await?;

    let dependencies = zip(pak_ids, &packages)
        .flat_map(|(i, (_, m))| m.dependencies.clone().into_iter().map(move |p| (i, p)))
        .collect::<Vec<(i64, FSNDependency)>>();
    let dep_ids: Vec<i64> = add_fsn_dependencies(&dependencies, &mut tx).await?;

    let dep_details = zip(dep_ids, dependencies)
        .flat_map(|(i, (_, m))| m.packages.clone().into_iter().map(move |p| (i, p)))
        .collect::<Vec<(i64, String)>>();
    add_fsn_dep_details(&dep_details, &mut tx).await?;

    // Handle files and hashes!
    // Generate list of hashes
    let mut hashes: HashedSet<db::SHA256Checksum> = HashedSet::from_iter(
        packages
            .iter()
            .flat_map(|(_, p)| p.filelist.iter().map(|f| f.checksum.clone())),
    );
    hashes.extend(
        packages
            .iter()
            .flat_map(|(_, p)| p.files.iter().map(|f| f.checksum.clone())),
    );

    let hashvec = hashes.into_iter().collect::<Vec<db::SHA256Checksum>>();
    let hmap = db::queries::add_hashes(&hashvec, &mut tx).await?;
    // Now we have ids for all the hashes we've inserted.
    // We've now got 2 tables to fill, the files table (what a package is made up of)
    // and the sources table (where to get each file)

    let mut files: Vec<db::File> = vec![];
    let mut sources: Vec<db::Source> = vec![];
    for (p_id, package) in packages {
        // first need to specify map of archives a file can be in.
        let mut archive_map = HashMap::<String, i64>::new();
        for archive in package.files {
            archive_map.insert(archive.filename, *hmap.get(&archive.checksum).unwrap());
            // While we're at it, add each archive as a source.
            for url in archive.urls {
                sources.push(db::Source {
                    h_id: hmap.get(&archive.checksum).unwrap().clone(),
                    par_id: None,
                    path: url,
                    location: db::SourceLocation::FSN,
                    s_type: db::SourceType::Raw,
                });
            }
        }
        // Now we add each file to our tables, we know parents too.
        for file in package.filelist {
            let h_id = hmap.get(&file.checksum).unwrap().clone();
            sources.push(db::Source {
                h_id,
                par_id: archive_map.get(&file.archive).copied(),
                path: file.orig_name, // Path inside archive.
                location: db::SourceLocation::FSN,
                s_type: db::SourceType::SevenZipEntry,
            });
            files.push(db::File {
                p_id,
                h_id,
                filepath: file.filename,
            })
        }
    }
    db::queries::add_sources(&sources, &mut tx).await?;
    db::queries::add_files(&files, &mut tx).await?;

    sqlx::query::<sqlx::Sqlite>("ANALYZE; PRAGMA analysis_limit=400;PRAGMA optimize;").execute(&mut tx)
    .await?;
    tx.commit().await?;
    Ok(())
}

async fn add_fsn_links(fsnmod: &FSNMod, rel_id: &i64, tx: &mut Transaction<'_, sqlx::Sqlite>) -> Result<(), sqlx::Error> {
    for screen in fsnmod.screenshots.iter() {
    db::queries::add_link(*rel_id, db::LinkType::Screenshot, screen, tx).await?;
            }
    for attach in fsnmod.attachments.iter() {
    db::queries::add_link(*rel_id, db::LinkType::Attachment, attach, tx).await?;
            }
    if let Some(thread) = &fsnmod.release_thread {
    db::queries::add_link(*rel_id, db::LinkType::ReleaseThread, thread, tx).await?;
            }
    for vid in fsnmod.videos.iter() {
    db::queries::add_link(*rel_id, db::LinkType::Video, vid, tx).await?;
            }
    Ok(for dep in fsnmod.mod_flag.iter() {
    add_mod_flags(*rel_id, dep, tx).await?;
            })
}

async fn add_fsn_releases(
    fsnmods: &Vec<FSNMod>,
    tx: &mut Transaction<'_, sqlx::Sqlite>,
) -> Result<Vec<i64>, sqlx::Error> {
    let rels_chunked = fsnmods.chunks(db::BIND_LIMIT / 5);
    let mut query_builder = QueryBuilder::new(
        "INSERT INTO releases (`name`, `version`, `rel_type`, `date`, `private`)",
    );
    let mut rel_ids: Vec<i64> = vec![];
    for relchunk in rels_chunked {
        query_builder.push_values(relchunk, |mut qb, m| {
            qb.push_bind(m.id.clone())
                .push_bind(m.version.clone())
                .push_bind(match m.mod_type {
                    FSNRelType::Engine => db::RelType::Build,
                    FSNRelType::Mod => db::RelType::Mod,
                    FSNRelType::TC => db::RelType::TC,
                })
                .push_bind(m.last_update.clone())
                .push_bind(m.private.clone());
        });
        query_builder.push("RETURNING rel_id");

        let query = query_builder.build();
        let mut res = query
            .fetch_all(&mut *tx)
            .await?
            .into_iter()
            .map(|x| x.get("rel_id"))
            .collect();
        rel_ids.append(&mut res);
        query_builder.reset();
    }
    let zipped_rels = zip(&rel_ids, fsnmods).into_iter();
    let mut builds: Vec<(i64, &FSNMod)> = vec![];
    let mut mods: Vec<(i64, &FSNMod)> = vec![];
    for (idx, fsnmod) in zipped_rels {
        match fsnmod.mod_type {
            FSNRelType::Engine => builds.push((idx.clone(), fsnmod)),
            FSNRelType::Mod => mods.push((idx.clone(), fsnmod)),
            FSNRelType::TC => mods.push((idx.clone(), fsnmod)),
        }
    }
    // These are slow for now.
    add_fsn_mods(mods, tx).await?;
    add_fsn_builds(builds, tx).await?;
    Ok(rel_ids.clone())
}

// Should optimise this so it's not a load of queries, but we're prototyping atm.
async fn add_fsn_mods(
    mods: Vec<(i64, &FSNMod)>,
    tx: &mut Transaction<'_, sqlx::Sqlite>,
) -> Result<(), sqlx::Error> {
    for (rel_id, fsnmod) in mods {
        add_fsn_mod(fsnmod, rel_id, tx).await?;
    }
    Ok(())
}

// Should optimise this so it's not a load of queries, but we're prototyping atm.
async fn add_fsn_builds(
    builds: Vec<(i64, &FSNMod)>,
    tx: &mut Transaction<'_, sqlx::Sqlite>,
) -> Result<(), sqlx::Error> {
    for (rel_id, fsnmod) in builds {
        add_fsn_build(fsnmod, rel_id, tx).await?;
    }
    Ok(())
}

async fn add_fsn_mod(
    fsnmod: &FSNMod,
    rel_id: i64,
    tx: &mut Transaction<'_, sqlx::Sqlite>,
) -> Result<SqliteQueryResult, sqlx::Error> {
    sqlx::query!(
        "INSERT INTO mods \
        (`rel_id`, `title`, `parent`, `description`, `logo`, `tile`, `banner`, `notes`, `cmdline`)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9) ",
        rel_id,
        fsnmod.title,
        fsnmod.parent,
        fsnmod.description,
        fsnmod.logo,
        fsnmod.tile,
        fsnmod.banner,
        fsnmod.notes,
        fsnmod.cmdline,
    )
    .execute(tx)
    .await
}

async fn add_fsn_build(
    fsnmod: &FSNMod,
    rel_id: i64,
    tx: &mut Transaction<'_, sqlx::Sqlite>,
) -> Result<SqliteQueryResult, sqlx::Error> {
    let stability = fsnmod.stability.expect("Builds require a set stability!");
    sqlx::query!(
        "INSERT INTO builds \
    (`rel_id`, `title`, `stability`, `description`, `notes`) \
    VALUES (?1, ?2, ?3, ?4, ?5)",
        rel_id,
        fsnmod.title,
        stability,
        fsnmod.description,
        fsnmod.notes,
    )
    .execute(tx)
    .await
}

pub(crate) async fn add_mod_flags(
    rel_id: i64,
    dep: &str,
    tx: &mut Transaction<'_, sqlx::Sqlite>,
) -> Result<SqliteQueryResult, sqlx::Error> {
    sqlx::query!(
        "INSERT OR IGNORE INTO `mod_flags` (rel_id, dep_name) \
        VALUES (?, ?);",
        rel_id,
        dep,
    )
    .execute(tx)
    .await
}

pub(crate) async fn add_fsn_packages(
    packages: &Vec<(i64, FSNPackage)>,
    tx: &mut Transaction<'_, sqlx::Sqlite>,
) -> Result<Vec<i64>, sqlx::Error> {
    let paks_chunked = packages.chunks(db::BIND_LIMIT / 7);
    let mut query_builder = QueryBuilder::new(
        "INSERT INTO packages (`rel_id`, `name`, `notes`, `status`, `environment`, `folder`, `is_vp`)"
    );
    let mut pak_ids: Vec<i64> = vec![];
    for relchunk in paks_chunked {
        query_builder.push_values(relchunk, |mut qb, (rel_id, p)| {
            qb.push_bind(rel_id)
                .push_bind(p.name.clone())
                .push_bind(p.notes.clone())
                .push_bind(p.status.clone())
                .push_bind(p.environment.clone())
                .push_bind(p.folder.clone())
                .push_bind(p.is_vp.clone());
        });
        query_builder.push("RETURNING p_id");

        let query = query_builder.build();
        let mut res = query
            .fetch_all(&mut *tx)
            .await?
            .into_iter()
            .map(|x| x.get("p_id"))
            .collect();
        pak_ids.append(&mut res);
        query_builder.reset();
    }
    Ok(pak_ids.clone())
}

pub(crate) async fn add_fsn_dependencies(
    dependencies: &Vec<(i64, FSNDependency)>,
    tx: &mut Transaction<'_, sqlx::Sqlite>,
) -> Result<Vec<i64>, sqlx::Error> {
    let deps_chunked = dependencies.chunks(db::BIND_LIMIT / 3);
    let mut query_builder =
        QueryBuilder::new("INSERT INTO package_deps (`p_id`, `modname`, `version`)");
    let mut dep_ids: Vec<i64> = vec![];
    for relchunk in deps_chunked {
        query_builder.push_values(relchunk, |mut qb, (pak_id, p)| {
            qb.push_bind(pak_id)
                .push_bind(p.id.clone())
                .push_bind(p.version.clone());
        });
        query_builder.push("RETURNING id");

        let query = query_builder.build();
        let mut res = query
            .fetch_all(&mut *tx)
            .await?
            .into_iter()
            .map(|x| x.get("id"))
            .collect();
        dep_ids.append(&mut res);
        query_builder.reset();
    }
    Ok(dep_ids.clone())
}

async fn add_fsn_dep_details(
    details: &Vec<(i64, String)>,
    tx: &mut Transaction<'_, sqlx::Sqlite>,
) -> Result<(), sqlx::Error> {
    let dets_chunked = details.chunks(db::BIND_LIMIT / 2);
    let mut query_builder = QueryBuilder::new("INSERT INTO dep_details (`dep_id`, `name`)");
    for relchunk in dets_chunked {
        query_builder.push_values(relchunk, |mut qb, (pak_id, p)| {
            qb.push_bind(pak_id).push_bind(p.clone());
        });

        let query = query_builder.build();
        query.execute(&mut *tx).await?;
        query_builder.reset();
    }
    Ok(())
}

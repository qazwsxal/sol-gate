use std::collections::HashSet;

use hash_hasher::HashedMap;
use sqlx::{query_builder::QueryBuilder, Transaction, sqlite::SqliteQueryResult};

use super::{File, Hash, SHA256Checksum, LinkType, Source, BIND_LIMIT};

pub(crate) async fn add_release_names(
    names: &Vec<String>,
    tx: &mut Transaction<'_, sqlx::Sqlite>,
) -> Result<(), sqlx::Error> {
    let names_chunked = names.chunks(BIND_LIMIT);
    let mut query_builder = QueryBuilder::new("INSERT INTO rel_names (`name`)");
    for relchunk in names_chunked {
        query_builder.push_values(relchunk, |mut qb, s| {
            qb.push_bind(s.clone());
        });

        let query = query_builder.build();
        query.execute(&mut *tx).await?;
        query_builder.reset();
    }
    Ok(())
}

pub(crate) async fn add_link(
    rel_id: i64,
    linktype: LinkType,
    link: &str,
    tx: &mut Transaction<'_, sqlx::Sqlite>,
) -> Result<SqliteQueryResult, sqlx::Error> {
    sqlx::query!(
        "INSERT OR IGNORE INTO `modlinks` (`rel_id`, `link_type`, `link`) \
        VALUES (?, ?, ?);",
        rel_id,
        linktype,
        link
    )
    .execute(tx)
    .await
}


pub(crate) async fn add_hashes(
    hashes: &Vec<SHA256Checksum>,
    tx: &mut Transaction<'_, sqlx::Sqlite>,
) -> Result<HashedMap<SHA256Checksum, i64>, sqlx::Error> {
    // We use a special hashmap with a very simple hash function
    // as sha256 hashes are already well distibuted
    let mut hmap: HashedMap<SHA256Checksum, i64> = HashedMap::default();

    let hashes_chunked = hashes.chunks(BIND_LIMIT);
    let mut existing_hashes_qb = QueryBuilder::new("SELECT id, val FROM hashes WHERE (val) in ");
    for hash_chunk in hashes_chunked {
        existing_hashes_qb.push_tuples(hash_chunk, |mut qb, hash| {
            qb.push_bind(hash);
        });
        let query = existing_hashes_qb.build_query_as::<Hash>();
        let rows = query.fetch_all(&mut *tx).await?;

        for res in rows {
            let id = res.id;
            let cs = res.val;
            hmap.insert(cs, id);
        }
        existing_hashes_qb.reset();
    }

    let new_hashes = hashes
        .iter()
        .filter(|&h| !hmap.contains_key(h))
        .map(|h| h.clone())
        .collect::<Vec<SHA256Checksum>>();
    let mut new_hashes_qb = QueryBuilder::new("INSERT INTO hashes (`val`) ");
    for hash_chunk in new_hashes.chunks(BIND_LIMIT / 2) {
        new_hashes_qb.push_values(hash_chunk, |mut qb, hash| {
            qb.push_bind(hash.clone());
        });
        new_hashes_qb.push(" RETURNING `id`, `val`");
        let query = new_hashes_qb.build_query_as::<Hash>();
        let rows = query.fetch_all(&mut *tx).await?;

        hmap.extend(rows.into_iter().map(|res| (res.val, res.id)));
        new_hashes_qb.reset();
    }
    Ok(hmap)
}

pub(crate) async fn add_files(
    files: &Vec<File>,
    tx: &mut Transaction<'_, sqlx::Sqlite>,
) -> Result<(), sqlx::Error> {
    let files_chunked = files.chunks(BIND_LIMIT / 3);
    let mut query_builder = QueryBuilder::new("INSERT INTO files (`p_id`, `h_id`, `filepath`)");
    for file_chunk in files_chunked {
        query_builder.push_values(file_chunk, |mut qb, f| {
            qb.push_bind(f.p_id.clone())
                .push_bind(f.h_id.clone())
                .push_bind(f.filepath.clone());
        });

        let query = query_builder.build();
        query.execute(&mut *tx).await?;
        query_builder.reset();
    }
    Ok(())
}

pub(crate) async fn add_sources(
    sources: &Vec<Source>,
    tx: &mut Transaction<'_, sqlx::Sqlite>,
) -> Result<(), sqlx::Error> {
    let dets_chunked = sources.chunks(BIND_LIMIT / 5);
    let mut query_builder =
        QueryBuilder::new("INSERT INTO sources (`h_id`, `par_id`, `path`, `location`, `s_type`)");
    for relchunk in dets_chunked {
        query_builder.push_values(relchunk, |mut qb, s| {
            qb.push_bind(s.h_id.clone())
                .push_bind(s.par_id.clone())
                .push_bind(s.path.clone())
                .push_bind(s.location.clone())
                .push_bind(s.s_type.clone());
        });

        let query = query_builder.build();
        query.execute(&mut *tx).await?;
        query_builder.reset();
    }
    Ok(())
}
use std::{
    collections::{HashMap, HashSet},
    ops::Deref,
};

use hash_hasher::HashedMap;
use sqlx::{query_builder::QueryBuilder, sqlite::SqliteQueryResult, Transaction};

use super::{EntryType, File, Hash, LinkType, Parent, Rel, SHA256Checksum, Source, BIND_LIMIT};

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
    let mut hmap = get_hash_ids(hashes, tx).await?;

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
    let mut query_builder = QueryBuilder::new("INSERT INTO sources (`h_id`, `path`, `location`,)");
    for relchunk in dets_chunked {
        query_builder.push_values(relchunk, |mut qb, s| {
            qb.push_bind(s.h_id.clone())
                .push_bind(s.path.clone())
                .push_bind(s.location.clone());
        });

        let query = query_builder.build();
        query.execute(&mut *tx).await?;
        query_builder.reset();
    }
    Ok(())
}

pub(crate) async fn add_parents(
    sources: &Vec<Parent>,
    tx: &mut Transaction<'_, sqlx::Sqlite>,
) -> Result<(), sqlx::Error> {
    let dets_chunked = sources.chunks(BIND_LIMIT / 4);
    let mut query_builder =
        QueryBuilder::new("INSERT INTO parents (`child`, `parent`, `child_path`, `par_type`)");
    for relchunk in dets_chunked {
        query_builder.push_values(relchunk, |mut qb, s| {
            qb.push_bind(s.child.clone())
                .push_bind(s.parent.clone())
                .push_bind(s.child_path.clone())
                .push_bind(s.par_type.clone());
        });

        let query = query_builder.build();
        query.execute(&mut *tx).await?;
        query_builder.reset();
    }
    Ok(())
}
pub async fn get_hash_ids(
    hashes: &Vec<SHA256Checksum>,
    tx: &mut Transaction<'_, sqlx::Sqlite>,
) -> Result<
    HashMap<SHA256Checksum, i64, std::hash::BuildHasherDefault<hash_hasher::HashHasher>>,
    sqlx::Error,
> {
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
    Ok(hmap)
}

pub async fn get_sources_from_hash(
    hash: &SHA256Checksum,
    tx: &mut Transaction<'_, sqlx::Sqlite>,
) -> Result<Vec<Source>, sqlx::Error> {
    let result = sqlx::query_as!(Source, r#"SELECT sources.h_id, sources.path, sources.location as "location: _", sources.size from sources, hashes WHERE sources.h_id = hashes.id AND hashes.val = ?"#, hash.0)
    .fetch_all(tx)
    .await;

    result
}

pub async fn get_sources_from_hashes(
    hashes: &Vec<SHA256Checksum>,
    tx: &mut Transaction<'_, sqlx::Sqlite>,
) -> Result<HashedMap<SHA256Checksum, Vec<Source>>, sqlx::Error> {
    let id_map = get_hash_ids(hashes, tx)
        .await?
        .into_iter()
        .map(|(hash, id)| (id, hash))
        .collect::<HashMap<i64, SHA256Checksum>>();

    let mut hmap: HashedMap<SHA256Checksum, Vec<Source>> = HashedMap::default();
    let id_vec = id_map.keys().map(|id| id.clone()).collect::<Vec<i64>>();
    let ids_chunked = id_vec.chunks(BIND_LIMIT);
    let mut existing_hashes_qb = QueryBuilder::new("SELECT * FROM sources WHERE (h_id) in ");
    for id_chunk in ids_chunked {
        existing_hashes_qb.push_tuples(id_chunk, |mut qb, hash| {
            qb.push_bind(hash);
        });
        let query = existing_hashes_qb.build_query_as::<Source>();
        let rows = query.fetch_all(&mut *tx).await?;

        for res in rows.iter() {
            // we've queried the DB using (a subset of) the contents of `ids` so this can't fail.
            let hash = id_map.get(&res.h_id).unwrap();
            hmap.entry(hash.clone())
                .or_insert(Vec::new())
                .push(res.clone());
        }
        existing_hashes_qb.reset();
    }
    Ok(hmap)
}

pub async fn get_sources_from_id(
    id: i64,
    tx: &mut Transaction<'_, sqlx::Sqlite>,
) -> Result<Vec<Source>, sqlx::Error> {
    let result = sqlx::query_as!(Source, r#"SELECT sources.h_id, sources.path, sources.location as "location: _", size from sources WHERE sources.h_id = ?"#, id)
    .fetch_all(tx)
    .await;

    result
}

pub async fn get_sources_from_ids(
    ids: &Vec<i64>,
    tx: &mut Transaction<'_, sqlx::Sqlite>,
) -> Result<Vec<Source>, sqlx::Error> {
    let mut sources = Vec::<Source>::new();
    let ids_chunked = ids.chunks(BIND_LIMIT);
    let mut query_builder = QueryBuilder::new("SELECT * FROM sources WHERE (h_id) in ");
    for id_chunk in ids_chunked {
        query_builder.push_tuples(id_chunk, |mut qb, hash| {
            qb.push_bind(hash);
        });
        let query = query_builder.build_query_as::<Source>();
        let mut rows = query.fetch_all(&mut *tx).await?;

        sources.append(&mut rows);

        query_builder.reset();
    }
    Ok(sources)
}

pub async fn get_parents_from_ids(
    ids: &Vec<i64>,
    tx: &mut Transaction<'_, sqlx::Sqlite>,
) -> Result<HashMap<i64, Vec<(i64, String, EntryType)>>, sqlx::Error> {
    let mut parents: HashMap<i64, Vec<(i64, String, EntryType)>> = HashMap::new();
    let ids_chunked = ids.chunks(BIND_LIMIT);
    let mut query_builder = QueryBuilder::new("SELECT * FROM parents WHERE (child) in ");
    for id_chunk in ids_chunked {
        query_builder.push_tuples(id_chunk, |mut qb, id| {
            qb.push_bind(id);
        });
        let query = query_builder.build_query_as::<Parent>();
        let rows = query.fetch_all(&mut *tx).await?;
        for parent in rows {
            parents
                .entry(parent.parent)
                .or_insert(Vec::new())
                .push((parent.child, parent.child_path, parent.par_type).clone())
        }
        query_builder.reset();
    }
    Ok(parents)
}

pub async fn get_hashes_from_ids(
    ids: &Vec<i64>,
    tx: &mut Transaction<'_, sqlx::Sqlite>,
) -> Result<Vec<Hash>, sqlx::Error> {
    let mut hashes = Vec::<Hash>::new();
    let ids_chunked = ids.chunks(BIND_LIMIT);
    let mut query_builder = QueryBuilder::new("SELECT id, val, size FROM hashes WHERE (id) in ");
    for hash_chunk in ids_chunked {
        query_builder.push_tuples(hash_chunk, |mut qb, hash| {
            qb.push_bind(hash);
        });
        let query = query_builder.build_query_as::<Hash>();
        let mut rows = query.fetch_all(&mut *tx).await?;
        hashes.append(&mut rows);
        query_builder.reset();
    }
    Ok(hashes)
}

pub async fn get_releases(
    tx: &mut Transaction<'_, sqlx::Sqlite>,
) -> Result<HashSet<Rel>, sqlx::Error> {
    let existing_releases = sqlx::query_as!(Rel, "SELECT `name`, `version` from releases;")
        .fetch_all(tx)
        .await?;

    Ok(HashSet::<Rel>::from_iter(existing_releases.into_iter()))
}

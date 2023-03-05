PRAGMA foreign_keys = ON;
CREATE TABLE IF NOT EXISTS rel_names (`name` TEXT PRIMARY KEY);
CREATE TABLE IF NOT EXISTS releases (
    `rel_id` INTEGER PRIMARY KEY AUTOINCREMENT,
    `name` TEXT NOT NULL REFERENCES rel_names(`name`),
    `version` TEXT NOT NULL,
    `rel_type` TEXT NOT NULL,
    `date` DATE NOT NULL,
    `private` INTEGER NOT NULL,
    UNIQUE(`name`, `version`)
);
-- We're often going to be sorting this by date, so good to have an index
CREATE INDEX IF NOT EXISTS release_date on releases(`date`);

-- We're going to be querying this for exiting releases too so good to have an index
CREATE UNIQUE INDEX IF NOT EXISTS release_vers on releases(`name`, `version`);

CREATE TABLE IF NOT EXISTS mods (
    `rel_id` INTEGER REFERENCES releases(rel_id),
    `title` TEXT NOT NULL,
    `parent` TEXT REFERENCES rel_names(`name`),
    `description` TEXT,
    `logo` TEXT,
    `tile` TEXT,
    `banner` TEXT,
    -- screenshots
    -- attachments
    -- release_thread
    -- videos
    `notes` TEXT,
    `cmdline` TEXT NOT NULL ,
    -- mod_flag
    -- packages
    `installed` INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS builds (
    `rel_id` INTEGER REFERENCES releases(rel_id),
    `title` TEXT NOT NULL,
    `stability` TEXT NOT NULL,
    `description` TEXT,
    `notes` TEXT
);

CREATE TABLE IF NOT EXISTS modlinks (
    `rel_id` INTEGER REFERENCES releases(rel_id),
    link_type TEXT,
    link TEXT
);


CREATE TABLE IF NOT EXISTS mod_flags (
    `key` INTEGER PRIMARY KEY AUTOINCREMENT,
    `rel_id` INTEGER REFERENCES releases(rel_id),
    dep_name TEXT NOT NULL REFERENCES rel_names(`name`)
);

CREATE TABLE IF NOT EXISTS packages (
    p_id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    `rel_id` INTEGER NOT NULL REFERENCES releases(rel_id),
    `name` TEXT NOT NULL,
    notes TEXT NOT NULL,
    `status` TEXT NOT NULL,
    environment TEXT,
    folder TEXT NOT NULL,
    is_vp INTEGER NOT NULL
);

-- Dependencies for each package (mod)
CREATE TABLE IF NOT EXISTS package_deps (
    `id` INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    p_id INTEGER NOT NULL REFERENCES packages(p_id),
    `modname` TEXT NOT NULL,
    `version` TEXT
);

-- details about what packages the above dependency needs
CREATE TABLE IF NOT EXISTS dep_details (
    `id` INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    dep_id INTEGER NOT NULL REFERENCES package_deps(`id`),
    `name` TEXT NOT NULL -- Name of optional and recommended packages that are are also required.
);

-- Big table of every hash ever seen, local or otherwise.
-- These are unique sha256 identifiers of a file's contents.
CREATE TABLE IF NOT EXISTS hashes (
    id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    val BLOB NOT NULL UNIQUE
);
CREATE UNIQUE INDEX IF NOT EXISTS hash_index ON hashes(val);

-- -- What files make up a package, 
CREATE TABLE IF NOT EXISTS files (
    `p_id` INTEGER NOT NULL REFERENCES packages(p_id),
    `h_id` INTEGER NOT NULL REFERENCES hashes(id),
    `filepath` TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS filehash_index ON files(`h_id`);
CREATE INDEX IF NOT EXISTS filepack_index ON files(`p_id`);


-- We need to know *where* a hash can be found. local or remote.
CREATE TABLE IF NOT EXISTS sources (
    `id` INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    `h_id` INTEGER NOT NULL REFERENCES hashes(id),
    `path` TEXT NOT NULL, -- path to source of file
    `location` TEXT NOT NULL,
    `format` TEXT NOT NULL,
    `size` INTEGER NOT NULL
);

-- We're likely to query this table often to know if we can find a hash anywhere or 
CREATE INDEX IF NOT EXISTS source_index ON sources(`h_id`);

-- Sometimes files are inside other ones, and we can extract them instead of re-downloading.
CREATE TABLE IF NOT EXISTS archive_entries(
    `file_id` INTEGER NOT NULL REFERENCES hashes(id),
    `archive_id` INTEGER NOT NULL REFERENCES hashes(id),
    `file_path` TEXT NOT NULL, -- Where to look in archive to get our file.
    `archive_type` TEXT NOT NULL -- what sort of archive (usually 7z or vp)
);
CREATE INDEX IF NOT EXISTS file_index ON archive_entries(`file_id`);
CREATE INDEX IF NOT EXISTS archive_index ON archive_entries(`archive_id`);
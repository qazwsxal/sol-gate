PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS mods (
    `id` TEXT NOT NULL,
    `title` TEXT NOT NULL,
    `version` TEXT NOT NULL,
    `private` INTEGER NOT NULL,
    `parent` TEXT,
    `description` TEXT,
    `logo` TEXT,
    `tile` TEXT,
    `banner` TEXT,
    `notes` TEXT,
    `first_release` TEXT,
    `last_update` TEXT,
    `cmdline` TEXT,
    `mod_type` INTEGER,
    PRIMARY KEY(`id`, `version`)
);

CREATE TABLE IF NOT EXISTS linktypes (
    `id` INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    `name` TEXT UNIQUE, -- screenshot, attachment, 
    UNIQUE (`name`)
);

INSERT OR IGNORE INTO `linktypes` (`name`) VALUES
    ("screenshot"),
    ("attachment"),
    ("thread"),
    ("videos");

CREATE TABLE IF NOT EXISTS modtypes (
    `id` INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    `name` TEXT, -- mod, tc, engine, 
    UNIQUE (`name`)
);

INSERT OR IGNORE INTO `modtypes` (`name`) VALUES
    ("mod"),
    ("tc"),
    ("engine");

CREATE TABLE IF NOT EXISTS mod_flags (
    `id` INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    mod_id INTEGER NOT NULL REFERENCES mods(id) ON DELETE CASCADE ON UPDATE CASCADE,
    dep_id INTEGER NOT NULL REFERENCES mods(id)
);


CREATE TABLE IF NOT EXISTS packages (
    p_id INTEGER PRIMARY KEY AUTOINCREMENT,
    mod_id INTEGER NOT NULL REFERENCES mods(m_id) ON DELETE CASCADE ON UPDATE CASCADE,
    p_name TEXT
);

CREATE TABLE IF NOT EXISTS modlinks (
    mod_id INTEGER REFERENCES mods(id) ON DELETE CASCADE ON UPDATE CASCADE,
    linktype INTEGER REFERENCES linktypes(id) ON DELETE CASCADE ON UPDATE CASCADE,
    link TEXT
);

CREATE TABLE IF NOT EXISTS filelinks (
    package_id INTEGER REFERENCES packages(p_id) ON DELETE CASCADE ON UPDATE CASCADE,
    link TEXT
);

CREATE TABLE IF NOT EXISTS mod_dep (
    pak_id INTEGER NOT NULL REFERENCES packages(p_id),
    dep_id INTEGER NOT NULL REFERENCES mods(id),
    dep_ver TEXT NOT NULL REFERENCES mods(`version`),
    `status` INTEGER,
    PRIMARY KEY(pak_id, dep_id, dep_ver)
);


CREATE TABLE IF NOT EXISTS pak_dep (
    pak_id INTEGER NOT NULL REFERENCES packages(p_id),
    dep_id INTEGER NOT NULL REFERENCES packages(p_id),
    `status` INTEGER,
    PRIMARY KEY(pak_id, dep_id)
);


CREATE TABLE IF NOT EXISTS dep_status (
    id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    `status` TEXT
);

INSERT OR IGNORE INTO `dep_status` (`status`) VALUES
    ("required"),
    ("recommended"),
    ("optional");

CREATE TABLE IF NOT EXISTS files (
    f_id INTEGER PRIMARY KEY AUTOINCREMENT,
    f_path TEXT,
    p_id INTEGER NOT NULL REFERENCES packages(p_id) ON DELETE CASCADE ON UPDATE CASCADE,
    hash_id INTEGER NOT NULL REFERENCES file_hashes(hash_id) ON DELETE CASCADE ON UPDATE CASCADE
);

CREATE TABLE IF NOT EXISTS file_hashes (
    hash_id INTEGER PRIMARY KEY AUTOINCREMENT,
    hash_val TEXT NOT NULL
);


PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS mods (
    `id` TEXT NOT NULL,
    `title` TEXT NOT NULL,
    `version` TEXT NOT NULL,
    `private` INTEGER NOT NULL,
    -- stability
    `parent` TEXT,
    `description` TEXT,
    `logo` TEXT,
    `tile` TEXT,
    `banner` TEXT,
    -- screenshots
    -- attachments
    -- release_thread
    -- videos
    `notes` TEXT,
    `first_release` TEXT,
    `last_update` TEXT,
    `cmdline` TEXT,
    -- mod_flag
    `mod_type` INTEGER,
    -- packages
    PRIMARY KEY(`id`, `version`)
);

-- stability
CREATE TABLE IF NOT EXISTS stability (
    `id` INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    `name` TEXT UNIQUE, -- screenshot, attachment, 
    UNIQUE (`name`)
);

INSERT OR IGNORE INTO `stability` (`name`) VALUES
    ("stable"),
    ("rc"),
    ("nightly"),

CREATE TABLE IF NOT EXISTS mod_stability (
    mod_id INTEGER REFERENCES mods(id) ON DELETE CASCADE ON UPDATE CASCADE,
    stability INTEGER REFERENCES stability(id),
    link TEXT
);


--screenshots, attachments, release_thread, videos
CREATE TABLE IF NOT EXISTS link_types (
    `id` INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    `name` TEXT UNIQUE, -- screenshot, attachment, 
    UNIQUE (`name`)
);

INSERT OR IGNORE INTO `link_types` (`name`) VALUES
    ("screenshot"),
    ("attachment"),
    ("thread"),
    ("videos");

CREATE TABLE IF NOT EXISTS modlinks (
    mod_id INTEGER REFERENCES mods(id) ON DELETE CASCADE ON UPDATE CASCADE,
    linktype INTEGER REFERENCES linktypes(id) ON DELETE CASCADE ON UPDATE CASCADE,
    link TEXT
);

-- mod_flags
CREATE TABLE IF NOT EXISTS mod_flags (
    `id` INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    mod_id INTEGER NOT NULL REFERENCES mods(id) ON DELETE CASCADE ON UPDATE CASCADE,
    dep_id INTEGER NOT NULL REFERENCES mods(id)
);

-- mod_types
CREATE TABLE IF NOT EXISTS mod_types (
    `id` INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    `name` TEXT, -- mod, tc, engine, 
    UNIQUE (`name`)
);

INSERT OR IGNORE INTO `mod_types` (`name`) VALUES
    ("mod"),
    ("tc"),
    ("engine");

-- packages
CREATE TABLE IF NOT EXISTS packages (
    p_id INTEGER PRIMARY KEY AUTOINCREMENT,
    mod_id INTEGER NOT NULL REFERENCES mods(m_id) ON DELETE CASCADE ON UPDATE CASCADE,
    p_name TEXT
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


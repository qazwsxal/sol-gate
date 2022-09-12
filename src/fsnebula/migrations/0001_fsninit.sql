PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS mod_ids (
    `id` TEXT,
    PRIMARY KEY(`id`)
);

CREATE TABLE IF NOT EXISTS stability (
    `stab` TEXT,
    PRIMARY KEY(`stab`)
);


CREATE TABLE IF NOT EXISTS mod_type (
    `type` TEXT,
    PRIMARY KEY(`type`)
);


CREATE TABLE IF NOT EXISTS dep_type (
    `type` TEXT NOT NULL,
    PRIMARY KEY(`type`)
);



CREATE TABLE IF NOT EXISTS mods (
    `id` TEXT NOT NULL REFERENCES mod_ids(`id`),
    `title` TEXT NOT NULL,
    `version` TEXT NOT NULL,
    `private` INTEGER NOT NULL,
    `parent` TEXT REFERENCES mod_ids(`id`),
    `description` TEXT,
    `logo` TEXT,
    `tile` TEXT,
    `banner` TEXT,
    -- screenshots
    -- attachments
    -- release_thread
    -- videos
    `notes` TEXT,
    `first_release` TEXT NOT NULL,
    `last_update` TEXT NOT NULL,
    `cmdline` TEXT NOT NULL,
    -- mod_flag
    `mod_type` TEXT NOT NULL REFERENCES mod_type(`type`),
    -- packages
    PRIMARY KEY(`id`, `version`)
);

CREATE INDEX IF NOT EXISTS mod_update_date on mods(`last_update`);

CREATE TRIGGER IF NOT EXISTS mod_id_insert
BEFORE INSERT ON mods
FOR EACH ROW 
    WHEN NOT EXISTS(SELECT 1 FROM mod_ids WHERE mod_ids.id == NEW.id)
    BEGIN INSERT INTO mod_ids (id) VALUES (NEW.id);
END;

CREATE TRIGGER IF NOT EXISTS mod_id_insert_parent
BEFORE INSERT ON mods
FOR EACH ROW 
    WHEN NOT EXISTS(SELECT 1 FROM mod_ids WHERE mod_ids.id == NEW.parent)
    BEGIN INSERT INTO mod_ids (id) VALUES (NEW.parent);
END;


CREATE TRIGGER IF NOT EXISTS modtype_insert
BEFORE INSERT ON mods
FOR EACH ROW 
    WHEN NOT EXISTS(SELECT 1 FROM mod_type WHERE mod_type.type == NEW.mod_type)
    BEGIN INSERT INTO mod_type (`type`) VALUES (NEW.mod_type);
END;



CREATE TABLE IF NOT EXISTS mods_stab (
    `stab` TEXT REFERENCES stability(stab),
    `id` TEXT NOT NULL,
    `version` TEXT NOT NULL,
    FOREIGN KEY(id, `version`) REFERENCES mods(id, `version`) ON DELETE CASCADE ON UPDATE CASCADE
);

CREATE TRIGGER IF NOT EXISTS stability_insert
BEFORE INSERT ON mods_stab
FOR EACH ROW 
    WHEN NOT EXISTS(SELECT 1 FROM stability WHERE stability.stab == NEW.stab)
    BEGIN INSERT INTO stability (stab) VALUES (NEW.stab);
END;


--screenshots, attachments, release_thread, videos
CREATE TABLE IF NOT EXISTS link_types (
    `type` TEXT, -- screenshot, attachment, etc.
    PRIMARY KEY(`type`)
);

CREATE TABLE IF NOT EXISTS modlinks (
    mod_id TEXT NOT NULL,
    mod_ver TEXT NOT NULL,
    link_type TEXT REFERENCES link_types(`type`),
    link TEXT,
    FOREIGN KEY(mod_id, mod_ver) REFERENCES mods(id, `version`)
);

CREATE TRIGGER IF NOT EXISTS linktype_insert
BEFORE INSERT ON modlinks
FOR EACH ROW
    WHEN NOT EXISTS(SELECT 1 FROM link_types WHERE link_types.type == NEW.link_type)
    BEGIN INSERT INTO link_types (`type`) VALUES (NEW.link_type);
END;

-- mod_flags
CREATE TABLE IF NOT EXISTS mod_flags (
    mod_id TEXT NOT NULL,
    mod_ver TEXT NOT NULL,
    dep_id TEXT NOT NULL REFERENCES mod_ids(id),
    FOREIGN KEY(mod_id, mod_ver) REFERENCES mods(id, `version`)
);

CREATE TRIGGER IF NOT EXISTS mod_flags_check
BEFORE INSERT ON mod_flags
FOR EACH ROW
    WHEN NOT EXISTS(SELECT 1 FROM mod_ids WHERE mod_ids.id == NEW.dep_id)
    BEGIN INSERT INTO mod_ids (`id`) VALUES (NEW.dep_id);
END;

-- packages
CREATE TABLE IF NOT EXISTS p_names (
    `name` TEXT PRIMARY KEY
);

CREATE TABLE IF NOT EXISTS packages (
    p_id INTEGER PRIMARY KEY AUTOINCREMENT,
    mod_id TEXT NOT NULL,
    mod_ver TEXT NOT NULL,
    p_name TEXT NOT NULL REFERENCES p_names(`name`),
    notes TEXT NOT NULL,
    `status` TEXT NOT NULL REFERENCES dep_type(`type`),
    environment TEXT,
    folder TEXT,
    is_vp INTEGER,
    FOREIGN KEY(mod_id, mod_ver) REFERENCES mods(`id`, `version`) ON DELETE CASCADE ON UPDATE CASCADE
);

CREATE TRIGGER IF NOT EXISTS p_name_insert
BEFORE INSERT ON packages
FOR EACH ROW
    WHEN NOT EXISTS(SELECT 1 FROM p_names WHERE p_names.name == NEW.p_name)
    BEGIN INSERT INTO p_names (`name`) VALUES (NEW.p_name);
END;

CREATE TRIGGER IF NOT EXISTS status_insert
BEFORE INSERT ON packages
FOR EACH ROW
    WHEN NOT EXISTS(SELECT 1 FROM `dep_type` WHERE dep_type.type == NEW.status)
    BEGIN INSERT INTO dep_type (`type`) VALUES (NEW.status);
END;

-- Dependencies for each package (mod)
CREATE TABLE IF NOT EXISTS pak_dep (
    `id` INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    p_id INTEGER NOT NULL REFERENCES packages(p_id),
    `version` TEXT,
    `dep_mod_id` TEXT NOT NULL
);

-- what packages the above dependency needs
CREATE TABLE IF NOT EXISTS dep_pak (
    `id` INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    dep_id INTEGER NOT NULL REFERENCES pak_dep(id),
    `name` REFERENCES p_names(`name`)
);

CREATE TRIGGER IF NOT EXISTS dep_name_insert
BEFORE INSERT ON dep_pak
FOR EACH ROW
    WHEN NOT EXISTS(SELECT 1 FROM p_names WHERE p_names.name == NEW.name)
    BEGIN INSERT INTO p_names (`name`) VALUES (NEW.name);
END;


CREATE TABLE IF NOT EXISTS zipfiles (
    `id` INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    p_id INTEGER REFERENCES packages(p_id) ON DELETE CASCADE ON UPDATE CASCADE,
    `filename` TEXT,
    `dest` TEXT,
    `filesize` INTEGER
);

CREATE TABLE IF NOT EXISTS files (
    f_id INTEGER PRIMARY KEY AUTOINCREMENT,
    f_path TEXT,
    zip_id INTEGER NOT NULL REFERENCES zipfiles(id) ON DELETE CASCADE ON UPDATE CASCADE,
    h_val TEXT NOT NULL REFERENCES hashes(val) ON DELETE CASCADE ON UPDATE CASCADE
);

CREATE TABLE IF NOT EXISTS hashes (
    id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    val BLOB NOT NULL UNIQUE
);
CREATE UNIQUE INDEX IF NOT EXISTS 
hash_index ON hashes (val);

CREATE TRIGGER IF NOT EXISTS files_hash
BEFORE INSERT ON files
FOR EACH ROW
    WHEN NOT EXISTS(SELECT 1 FROM hashes WHERE hashes.val == NEW.h_val)
    BEGIN INSERT INTO hashes (`val`) VALUES (NEW.h_val);
END;

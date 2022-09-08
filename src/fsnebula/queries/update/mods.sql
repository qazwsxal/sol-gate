INSERT OR IGNORE INTO mods (
                    `id`,
                    `title`,
                    `version`,
                    `private`,
                    `parent`,
                    `description`,
                    `logo`,
                    `tile`, 
                    `banner`,
                    `notes`,
                    `first_release`,
                    `last_update`,
                    `cmdline`,
                    `mod_type`)
VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14);



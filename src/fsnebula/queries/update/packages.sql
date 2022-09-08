INSERT OR IGNORE INTO `packages` (mod_id, mod_ver, p_name, notes, `status`, environment, folder, is_vp) 
VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
RETURNING p_id;
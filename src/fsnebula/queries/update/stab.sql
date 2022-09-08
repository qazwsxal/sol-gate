INSERT OR IGNORE INTO `mod_stability` (mod_id, mod_ver, stability) 
SELECT ?, ?, stability.id FROM stability WHERE stability.name = ?;
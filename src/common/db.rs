use sqlx::migrate::Migrator;

static SQLITE: Migrator = sqlx::migrate!("./migrate/");
use sqlx::migrate::Migrator;

static MIG: Migrator = sqlx::migrate!("./migrate/");
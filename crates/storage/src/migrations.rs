#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DatabaseKind {
    Sqlite,
    Postgres,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MigrationSet {
    pub kind: DatabaseKind,
    pub path: &'static str,
}

impl MigrationSet {
    pub fn sqlite() -> Self {
        Self {
            kind: DatabaseKind::Sqlite,
            path: "crates/storage/migrations/sqlite",
        }
    }

    pub fn postgres() -> Self {
        Self {
            kind: DatabaseKind::Postgres,
            path: "crates/storage/migrations/postgres",
        }
    }
}

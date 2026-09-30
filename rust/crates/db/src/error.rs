use std::fmt;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Sqlite(#[from] rusqlite::Error),

    /// `ActiveRecord::RecordNotFound`
    #[error("Couldn't find {0}")]
    RecordNotFound(&'static str),

    /// `ActiveRecord::RecordInvalid`
    #[error("Validation failed: {0}")]
    RecordInvalid(Errors),

    #[error("database writer is gone")]
    WriterGone,

    /// The database's `schema_migrations` isn't exactly the set this build's schema was generated
    /// from (see `schema::prepare`).
    #[error("{0}")]
    SchemaMismatch(crate::schema::SchemaMismatch),

    #[error("{0}")]
    Other(String),
}

pub type Result<T, E = Error> = std::result::Result<T, E>;

impl Error {
    /// `ActiveRecord::RecordNotUnique`: a unique index (or primary key) rejected the write.
    pub fn is_record_not_unique(&self) -> bool {
        matches!(
            self,
            Error::Sqlite(rusqlite::Error::SqliteFailure(e, _))
                if e.extended_code == rusqlite::ffi::SQLITE_CONSTRAINT_UNIQUE
                    || e.extended_code == rusqlite::ffi::SQLITE_CONSTRAINT_PRIMARYKEY
        )
    }
}

/// `ActiveModel::Errors`: attribute/message pairs in the order they were added.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Errors(pub Vec<(&'static str, String)>);

impl Errors {
    pub fn add(&mut self, attribute: &'static str, message: impl Into<String>) {
        self.0.push((attribute, message.into()));
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn on(&self, attribute: &str) -> Vec<&str> {
        self.0
            .iter()
            .filter(|(a, _)| *a == attribute)
            .map(|(_, m)| m.as_str())
            .collect()
    }

    /// `errors.full_messages`: "Endpoint must use HTTPS"
    pub fn full_messages(&self) -> Vec<String> {
        self.0
            .iter()
            .map(|(attribute, message)| {
                if *attribute == "base" {
                    message.clone()
                } else {
                    format!("{} {message}", humanize(attribute))
                }
            })
            .collect()
    }

    pub fn into_result(self) -> Result<()> {
        if self.is_empty() {
            Ok(())
        } else {
            Err(Error::RecordInvalid(self))
        }
    }
}

impl fmt::Display for Errors {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.full_messages().join(", "))
    }
}

fn humanize(attribute: &str) -> String {
    let words = attribute.trim_end_matches("_id").replace(['_', '.'], " ");
    let mut chars = words.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}

pub(crate) trait OptionalExt<T> {
    fn or_not_found(self, model: &'static str) -> Result<T>;
}

impl<T> OptionalExt<T> for Option<T> {
    fn or_not_found(self, model: &'static str) -> Result<T> {
        self.ok_or(Error::RecordNotFound(model))
    }
}

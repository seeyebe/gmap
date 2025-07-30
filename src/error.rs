use thiserror::Error;

pub type Result<T> = std::result::Result<T, GmapError>;

#[derive(Error, Debug)]
pub enum GmapError {
    #[error("Git error: {0}")]
    Git(#[from] Box<gix::open::Error>),
    #[error("Git repository error: {0}")]
    GitRepo(String),
    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),
    #[error("Cache error: {0}")]
    Cache(String),
    #[error("Serialization error: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Parse error: {0}")]
    Parse(String),
    #[error("Invalid date: {0}")]
    InvalidDate(String),
    #[error("Other: {0}")]
    Other(String),
    #[error("Object find error: {0}")]
    ObjectFind(#[from] Box<gix::object::find::existing::Error>),
    #[error("Commit error: {0}")]
    Commit(#[from] Box<gix::object::commit::Error>),
    #[error("Reference find error: {0}")]
    RefFind(#[from] Box<gix::reference::find::existing::Error>),
    #[error("Head peel error: {0}")]
    HeadPeel(#[from] Box<gix::head::peel::to_commit::Error>),
    #[error("Object find with conversion error: {0}")]
    ObjectFindConv(#[from] Box<gix::object::find::existing::with_conversion::Error>),
    #[error("Object decode error: {0}")]
    ObjectDecode(#[from] Box<gix::objs::decode::Error>),
    #[error("Diff tree to tree error: {0}")]
    DiffTreeToTree(#[from] Box<gix::repository::diff_tree_to_tree::Error>),
    #[error("Git discover error: {0}")]
    GitDiscover(#[from] Box<gix::discover::Error>),
}

// Manual From implementations for unboxed to boxed conversions
impl From<gix::open::Error> for GmapError {
    fn from(err: gix::open::Error) -> Self {
        GmapError::Git(Box::new(err))
    }
}

impl From<gix::object::find::existing::Error> for GmapError {
    fn from(err: gix::object::find::existing::Error) -> Self {
        GmapError::ObjectFind(Box::new(err))
    }
}

impl From<gix::object::commit::Error> for GmapError {
    fn from(err: gix::object::commit::Error) -> Self {
        GmapError::Commit(Box::new(err))
    }
}

impl From<gix::reference::find::existing::Error> for GmapError {
    fn from(err: gix::reference::find::existing::Error) -> Self {
        GmapError::RefFind(Box::new(err))
    }
}

impl From<gix::head::peel::to_commit::Error> for GmapError {
    fn from(err: gix::head::peel::to_commit::Error) -> Self {
        GmapError::HeadPeel(Box::new(err))
    }
}

impl From<gix::object::find::existing::with_conversion::Error> for GmapError {
    fn from(err: gix::object::find::existing::with_conversion::Error) -> Self {
        GmapError::ObjectFindConv(Box::new(err))
    }
}

impl From<gix::objs::decode::Error> for GmapError {
    fn from(err: gix::objs::decode::Error) -> Self {
        GmapError::ObjectDecode(Box::new(err))
    }
}

impl From<gix::repository::diff_tree_to_tree::Error> for GmapError {
    fn from(err: gix::repository::diff_tree_to_tree::Error) -> Self {
        GmapError::DiffTreeToTree(Box::new(err))
    }
}

impl From<gix::discover::Error> for GmapError {
    fn from(err: gix::discover::Error) -> Self {
        GmapError::GitDiscover(Box::new(err))
    }
}

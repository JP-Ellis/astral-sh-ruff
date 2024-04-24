use crate::cache::KeyValueCache;
use crate::db::{HasJar, SourceDb, SourceJar};
use ruff_notebook::Notebook;
use ruff_python_ast::PySourceType;
use std::ops::{Deref, DerefMut};
use std::sync::Arc;

use crate::files::FileId;

pub(crate) fn source_text<Db>(db: &Db, file_id: FileId) -> Source
where
    Db: SourceDb + HasJar<SourceJar>,
{
    let sources = &db.jar().sources;

    sources.get(&file_id, |file_id| {
        tracing::trace!("Reading source text for file_id={:?}.", file_id);

        let path = db.file_path(*file_id);

        let source_text = std::fs::read_to_string(&path).unwrap_or_else(|err| {
            tracing::error!("Failed to read file '{path:?}: {err}'. Falling back to empty text");
            String::new()
        });

        let python_ty = PySourceType::from(&path);

        let kind = match python_ty {
            PySourceType::Python => {
                SourceKind::Python(Arc::from(source_text))
            }
            PySourceType::Stub => SourceKind::Stub(Arc::from(source_text)),
            PySourceType::Ipynb => {
                let notebook = Notebook::from_source_code(&source_text).unwrap_or_else(|err| {
                    // TODO should this be changed to never fail?
                    // or should we instead add a diagnostic somewhere? But what would we return in this case?
                    tracing::error!(
                        "Failed to parse notebook '{path:?}: {err}'. Falling back to an empty notebook"
                    );
                    Notebook::from_source_code("").unwrap()
                });

                SourceKind::IpyNotebook(Arc::new(notebook))
            }
        };

        Source { kind }
    })
}

#[derive(Debug, Clone, PartialEq)]
pub enum SourceKind {
    Python(Arc<str>),
    Stub(Arc<str>),
    IpyNotebook(Arc<Notebook>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Source {
    kind: SourceKind,
}

impl Source {
    pub fn python<T: Into<Arc<str>>>(source: T) -> Self {
        Self {
            kind: SourceKind::Python(source.into()),
        }
    }
    pub fn kind(&self) -> &SourceKind {
        &self.kind
    }

    pub fn text(&self) -> &str {
        match &self.kind {
            SourceKind::Python(text) => text,
            SourceKind::Stub(text) => text,
            SourceKind::IpyNotebook(notebook) => notebook.source_code(),
        }
    }
}

#[derive(Debug, Default)]
pub struct SourceStorage(pub(crate) KeyValueCache<FileId, Source>);

impl Deref for SourceStorage {
    type Target = KeyValueCache<FileId, Source>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for SourceStorage {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

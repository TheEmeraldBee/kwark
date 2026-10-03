use std::{collections::HashMap, io, path::PathBuf, rc::Rc};

use tree_sitter::{LanguageError, Parser, Query};

use crate::error::QueryLoadError;

pub use tree_sitter::Language;

/// A global registry for tree-sitter language definitions
pub struct LanguageStore {
    map: HashMap<String, StoredLanguage>,

    queries: HashMap<(String, String), Rc<Query>>,

    query_path: PathBuf,
}

struct StoredLanguage {
    language: Language,
    parser: Parser,
}

impl LanguageStore {
    /// Creates a new empty language store
    ///
    /// # Errors
    /// The function returns an IO error if the `query_folder` path cannot be cannonicalized.
    pub fn new(query_folder: PathBuf) -> Result<Self, io::Error> {
        let query_path = query_folder.canonicalize()?;

        Ok(Self {
            map: HashMap::new(),
            queries: HashMap::new(),
            query_path,
        })
    }

    /// Loads and retrieves a query from the store, compiling it if needed
    ///
    /// # Errors
    /// The function returns an error when:
    /// - The language doesn't exist
    /// - The query path is missing
    /// - The file fails to read
    /// - The query contents fail to load
    pub fn get_query(&mut self, lang: &str, query: &str) -> Result<Rc<Query>, QueryLoadError> {
        let key = (lang.to_string(), query.to_string());

        if let Some(cached) = self.queries.get(&key) {
            return Ok(cached.clone());
        }

        let Some(language) = self.get_language(lang) else {
            return Err(QueryLoadError::NoLanguage(lang.to_string()));
        };

        let query_path = self.query_path.join(lang).join(format!("{}.scm", query));

        if !query_path.exists() {
            return Err(QueryLoadError::MissingPath(query_path));
        }

        let file_contents = std::fs::read_to_string(query_path)?;

        let compiled = Rc::new(Query::new(language, &file_contents)?);

        self.queries.insert(key, compiled.clone());

        Ok(compiled)
    }

    /// Returns whether `lang` has a `{query}.scm` file on disk, without compiling it
    pub fn has_query(&self, lang: &str, query: &str) -> bool {
        self.query_path
            .join(lang)
            .join(format!("{}.scm", query))
            .exists()
    }

    /// Returns the names of every language currently registered
    pub fn languages(&self) -> impl Iterator<Item = &str> {
        self.map.keys().map(String::as_str)
    }

    /// Registers a new language into the store
    ///
    /// # Errors
    /// Returns an error when creating the parser fails with the given language
    pub fn register(
        &mut self,
        name: impl Into<String>,
        lang: Language,
    ) -> Result<&mut Self, LanguageError> {
        let mut parser = Parser::new();
        parser.set_language(&lang)?;

        let stored = StoredLanguage {
            language: lang,
            parser,
        };

        self.map.insert(name.into(), stored);
        Ok(self)
    }

    /// Retrieves the parser for the given language from the store
    pub fn get_parser(&mut self, lang: &str) -> Option<&mut Parser> {
        self.map.get_mut(lang).map(|x| &mut x.parser)
    }

    /// Retrieves a language definition from the store
    pub fn get_language(&self, lang: &str) -> Option<&Language> {
        self.map.get(lang).map(|x| &x.language)
    }
}

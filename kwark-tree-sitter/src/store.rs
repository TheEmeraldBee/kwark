use std::{
    collections::HashMap,
    io,
    path::{Path, PathBuf},
    sync::Arc,
};

use tree_sitter::{LanguageError, Parser, Query};

use crate::detect::parse_shebang;
use crate::error::QueryLoadError;

pub use tree_sitter::Language;

/// A global registry for tree-sitter language definitions
pub struct LanguageStore {
    map: HashMap<String, StoredLanguage>,

    queries: HashMap<(String, String), Arc<Query>>,

    query_path: PathBuf,
}

struct StoredLanguage {
    language: Language,
    parser: Parser,

    extensions: Vec<String>,
    filenames: Vec<String>,
    shebangs: Vec<String>,
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
    pub fn get_query(&mut self, lang: &str, query: &str) -> Result<Arc<Query>, QueryLoadError> {
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

        let compiled = Arc::new(Query::new(language, &file_contents)?);

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
    /// Returns a [`LanguageBuilder`] for attaching detection rules to it
    ///
    /// # Errors
    /// Returns an error when creating the parser fails with the given language
    pub fn register(
        &mut self,
        name: impl Into<String>,
        lang: Language,
    ) -> Result<LanguageBuilder<'_>, LanguageError> {
        let mut parser = Parser::new();
        parser.set_language(&lang)?;

        let stored = StoredLanguage {
            language: lang,
            parser,
            extensions: Vec::new(),
            filenames: Vec::new(),
            shebangs: Vec::new(),
        };

        let name = name.into();
        self.map.insert(name.clone(), stored);

        Ok(LanguageBuilder { store: self, name })
    }

    /// Returns a copy of the store with its own parsers
    ///
    /// # Errors
    /// Returns an error when creating a parser fails
    pub fn fork(&self) -> Result<Self, crate::error::Error> {
        let mut map = HashMap::with_capacity(self.map.len());

        for (name, stored) in &self.map {
            let mut parser = Parser::new();
            parser.set_language(&stored.language)?;

            map.insert(
                name.clone(),
                StoredLanguage {
                    language: stored.language.clone(),
                    parser,
                    extensions: stored.extensions.clone(),
                    filenames: stored.filenames.clone(),
                    shebangs: stored.shebangs.clone(),
                },
            );
        }

        Ok(Self {
            map,
            queries: self.queries.clone(),
            query_path: self.query_path.clone(),
        })
    }

    /// Keeps the queries another store compiled that this store lacks
    pub fn absorb_queries(&mut self, other: LanguageStore) {
        for (key, query) in other.queries {
            self.queries.entry(key).or_insert(query);
        }
    }

    /// Retrieves the parser for the given language from the store
    pub fn get_parser(&mut self, lang: &str) -> Option<&mut Parser> {
        self.map.get_mut(lang).map(|x| &mut x.parser)
    }

    /// Retrieves a language definition from the store
    pub fn get_language(&self, lang: &str) -> Option<&Language> {
        self.map.get(lang).map(|x| &x.language)
    }

    /// Resolves a buffer to a registered language name
    ///
    /// Tries, in order, the file name, the file extension, and the shebang
    /// line, returning the first language whose detection rules match
    pub fn detect(&self, path: Option<&Path>, first_line: Option<&str>) -> Option<&str> {
        let file_name = path.and_then(|p| p.file_name()).and_then(|n| n.to_str());
        let extension = path
            .and_then(|p| p.extension())
            .and_then(|e| e.to_str())
            .map(str::to_lowercase);
        let interpreter = first_line.and_then(parse_shebang);

        if let Some(file_name) = file_name
            && let Some(name) = self.find(|lang| lang.filenames.iter().any(|f| f == file_name))
        {
            return Some(name);
        }

        if let Some(extension) = &extension
            && let Some(name) = self.find(|lang| lang.extensions.iter().any(|e| e == extension))
        {
            return Some(name);
        }

        if let Some(interpreter) = interpreter
            && let Some(name) = self.find(|lang| lang.shebangs.iter().any(|s| s == interpreter))
        {
            return Some(name);
        }

        None
    }

    fn find(&self, matches: impl Fn(&StoredLanguage) -> bool) -> Option<&str> {
        self.map
            .iter()
            .find(|(_, lang)| matches(lang))
            .map(|(name, _)| name.as_str())
    }
}

/// A handle for attaching detection rules to a just-registered language
pub struct LanguageBuilder<'a> {
    store: &'a mut LanguageStore,
    name: String,
}

impl LanguageBuilder<'_> {
    /// Adds file extensions (without the leading `.`) that detect this language
    pub fn with_extensions(mut self, exts: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.stored()
            .extensions
            .extend(exts.into_iter().map(Into::into));
        self
    }

    /// Adds exact file names (e.g. `Dockerfile`) that detect this language
    pub fn with_filenames(mut self, names: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.stored()
            .filenames
            .extend(names.into_iter().map(Into::into));
        self
    }

    /// Adds shebang interpreter names (e.g. `bash`) that detect this language
    pub fn with_shebangs(
        mut self,
        interpreters: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        self.stored()
            .shebangs
            .extend(interpreters.into_iter().map(Into::into));
        self
    }

    fn stored(&mut self) -> &mut StoredLanguage {
        self.store
            .map
            .get_mut(&self.name)
            .expect("LanguageBuilder always holds a name that was just registered")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn store_with_rust() -> LanguageStore {
        let mut store = LanguageStore::new(PathBuf::from(env!("CARGO_MANIFEST_DIR"))).unwrap();

        store
            .register("rust", Language::new(tree_sitter_rust::LANGUAGE))
            .unwrap()
            .with_extensions(["rs"])
            .with_filenames(["build.rs"])
            .with_shebangs(["rustrun"]);

        store
    }

    #[test]
    fn detects_by_extension() {
        let store = store_with_rust();
        assert_eq!(store.detect(Some(Path::new("main.rs")), None), Some("rust"));
    }

    #[test]
    fn detects_by_filename_even_without_extension_match() {
        let store = store_with_rust();
        assert_eq!(
            store.detect(Some(Path::new("build.rs")), None),
            Some("rust")
        );
    }

    #[test]
    fn detects_by_shebang_when_no_extension_matches() {
        let store = store_with_rust();
        assert_eq!(
            store.detect(Some(Path::new("script")), Some("#!/usr/bin/env rustrun")),
            Some("rust")
        );
    }

    #[test]
    fn no_match_returns_none() {
        let store = store_with_rust();
        assert_eq!(store.detect(Some(Path::new("notes.txt")), None), None);
    }

    #[test]
    fn fork_keeps_languages_and_detection() {
        let store = store_with_rust();
        let mut fork = store.fork().unwrap();

        assert_eq!(fork.detect(Some(Path::new("main.rs")), None), Some("rust"));
        assert!(fork.get_parser("rust").is_some());
    }

    #[test]
    fn fork_can_move_to_another_thread() {
        let fork = store_with_rust().fork().unwrap();

        let parsed = std::thread::spawn(move || {
            let mut fork = fork;
            let rope = ropey::Rope::from_str("fn main() {}");
            crate::tree::SyntaxTree::parse(&mut fork, "rust", &rope).is_ok()
        })
        .join()
        .unwrap();

        assert!(parsed);
    }
}

use std::collections::{HashMap, HashSet};

use jsonfilter::FilterError;

/// Extracts the front matter from a Markdown document.
///
/// This function scans the input `markdown` string for a YAML front matter block,
/// which is expected to be enclosed within triple dashes (`---`). The front matter
/// is typically used in Markdown files to specify metadata such as title, date, tags,
/// etc.
///
/// # Arguments
///
/// * `markdown` - A string slice that holds the content of the Markdown document.
///
/// # Returns
///
/// An `Option<String>` which contains the front matter if found, or `None` if the
/// front matter block is not present in the provided `markdown`.
///
/// # Examples
///
/// ```
/// use mdq::get_frontmatter;
///
/// let markdown = r#"---
/// title: "Sample Document"
/// date: "2024-06-06"
/// tags: ["rust", "markdown"]
/// ---
///
/// # Introduction
///
/// This is the content of the Markdown document.
/// "#;
///
/// let frontmatter = get_frontmatter(markdown);
/// assert_eq!(frontmatter, Some(String::from(r#"title: "Sample Document"
/// date: "2024-06-06"
/// tags: ["rust", "markdown"]"#)));
/// ```
#[must_use]
pub fn get_frontmatter(markdown: &str) -> Option<String> {
    let frontmatter_regex = regex::Regex::new(r"(?s)^---\s*\n(.*?)\n---").unwrap();

    frontmatter_regex.captures(markdown).and_then(|captures| {
        let frontmatter = captures.get(1).map(|m| m.as_str().to_string());

        frontmatter
    })
}

/// Extracts inline `#tags` from a Markdown document.
///
/// This function scans the input `markdown` string for inline tags prefixed with a
/// hash (`#`) symbol. Inline tags are commonly used in Markdown documents to
/// categorize content or add metadata within the text body.
///
/// # Arguments
///
/// * `markdown` - A string slice that holds the content of the Markdown document.
///
/// # Returns
///
/// A `Vec<String>` containing all the tags found in the Markdown document. Each tag
/// is represented without the leading `#` symbol. If no tags are found, an empty
/// vector is returned.
///
/// # Examples
///
/// ```
/// use mdq::get_inline_tags;
///
/// let markdown = r#"
/// # Introduction
/// This document is a sample for #Rust and #Markdown.
///
/// # Content
/// Here we have some #examples and #code snippets.
/// "#;
///
/// let tags = get_inline_tags(markdown);
/// assert_eq!(tags, vec!["Rust", "Markdown", "examples", "code"]);
/// ```
#[must_use]
pub fn get_inline_tags(markdown: &str) -> Vec<String> {
    let tag_regex = regex::Regex::new(r"#(\w+)").unwrap();
    let mut tags: Vec<String> = vec![];

    for captures in tag_regex.captures_iter(markdown) {
        if let Some(tag) = captures.get(1) {
            tags.push(tag.as_str().to_string());
        }
    }

    tags
}

fn system_time_to_date_time(t: std::time::SystemTime) -> chrono::DateTime<chrono::Utc> {
    let (sec, nsec) = match t.duration_since(std::time::UNIX_EPOCH) {
        Ok(dur) => (dur.as_secs() as i64, dur.subsec_nanos()),
        Err(e) => {
            // unlikely but should be handled
            let dur = e.duration();
            let (sec, nsec) = (dur.as_secs() as i64, dur.subsec_nanos());
            if nsec == 0 {
                (-sec, 0)
            } else {
                (-sec - 1, 1_000_000_000 - nsec)
            }
        }
    };
    chrono::TimeZone::timestamp_opt(&chrono::Utc, sec, nsec).unwrap()
}

/// Represents a Markdown document with a file path and front matter metadata.
///
/// The `Document` struct encapsulates the essential properties of a Markdown document,
/// including its file path and the parsed front matter. The front matter is typically
/// represented in YAML format and stored as a `serde_yaml::Value`.
#[derive(Debug, Clone)]
pub struct Document {
    /// The file path of the Markdown document.
    pub path: String,
    /// The parsed front matter metadata in YAML format.
    pub frontmatter: serde_yaml::Value,
}

#[derive(Debug)]
pub struct Index {
    pub documents: Vec<Document>,
}

type Table = Vec<Vec<String>>;

/// Markdown Document Index
impl Index {
    /// Creates a new markdown document index for a given directory.
    ///
    /// This method scans the specified directory recursively for Markdown files
    /// (`.md` extension) and constructs an index of `Document` instances. Optionally,
    /// it can also extract inline tags from the document content and add them to the
    /// front matter metadata.
    ///
    /// # Arguments
    ///
    /// * `dir` - A string slice representing the directory to scan for Markdown files.
    /// * `inline_tags` - A boolean indicating whether to extract inline tags from the
    ///   document content and add them to the front matter.
    ///
    /// # Returns
    ///
    /// An `Index` instance containing the indexed documents.
    ///
    /// # Panics
    ///
    /// This method will panic if it fails to read a file or if the front matter
    /// cannot be parsed as valid YAML.
    pub fn new(dir: &str, inline_tags: bool) -> Self {
        let mut i = Self { documents: vec![] };

        for e in walkdir::WalkDir::new(dir)
            .into_iter()
            .filter_map(std::result::Result::ok)
        {
            if e.path().is_dir() {
                continue;
            }
            if e.path().extension().is_none() {
                continue;
            }

            if e.path().extension().unwrap().to_str().unwrap() == "md" {
                let path = e.path().to_str().unwrap().to_owned();
                let content = std::fs::read_to_string(&path).unwrap();
                let frontmatter = get_frontmatter(&content).unwrap_or_else(|| "{}".to_owned());
                let mut frontmatter: serde_yaml::Value =
                    serde_yaml::from_str(&frontmatter).unwrap();

                if inline_tags {
                    let mut tags = frontmatter
                        .as_mapping()
                        .unwrap()
                        .get("tags")
                        .map(|x| x.as_sequence().unwrap_or(&Vec::new()).clone())
                        .unwrap_or_default();
                    let inline_tags = get_inline_tags(&content);

                    tags.extend(inline_tags.iter().map(|x| x.clone().into()));

                    let mut unique_tags = HashSet::new();
                    for tag in tags {
                        unique_tags.insert(tag);
                    }

                    frontmatter
                        .as_mapping_mut()
                        .unwrap()
                        .insert("tags".into(), unique_tags.into_iter().collect());
                }

                log::trace!("Adding {path} to Index");
                let doc = Document { path, frontmatter };
                i.documents.push(doc);
            }
        }

        i
    }

    /// Builds a table with specified columns from the index within a specified scope.
    ///
    /// This method allows you to apply a limit, offset, and sorting to the documents
    /// in the index, returning a new `Index` with the resulting documents.
    ///
    /// # Arguments
    ///
    /// * `limit` - The maximum number of documents to include in the resulting index.
    /// * `offset` - The number of documents to skip before starting to include documents in the resulting index.
    /// * `sort` - An optional string specifying the key to sort the documents by.
    /// * `reverse` - A boolean indicating whether to reverse the sort order.
    ///
    /// # Returns
    ///
    /// A new `Index` containing the documents within the specified scope.
    #[must_use]
    pub fn apply(&self, limit: usize, offset: usize, sort: Option<String>, reverse: bool) -> Self {
        let mut scope = self.documents.clone();

        if let Some(sort) = sort {
            scope.sort_by(|a, b| {
                let a_str: serde_json::Value = a.get_key(&sort);
                let b_str: serde_json::Value = b.get_key(&sort);

                jsonfilter::order(&a_str, &b_str)
            });
        }

        if reverse {
            scope.reverse();
        }

        let scope = scope.into_iter().skip(offset);

        if limit == 0 {
            Self {
                documents: scope.collect(),
            }
        } else {
            Self {
                documents: scope.take(limit).collect(),
            }
        }
    }

    /// Groups the documents in the index by a specified key.
    ///
    /// This method groups the documents based on the value of a specified key in the
    /// front matter, returning a `HashMap` where the keys are the unique values of the
    /// specified key, and the values are new `Index` instances containing the grouped documents.
    ///
    /// # Arguments
    ///
    /// * `key` - A string slice representing the key to group the documents by.
    ///
    /// # Returns
    ///
    /// A `HashMap` where each key is a unique value of the specified key in the front matter,
    /// and each value is an `Index` containing the documents that share that key.
    #[must_use]
    pub fn group_by(&self, key: &str) -> HashMap<String, Self> {
        let mut grouped_items: HashMap<String, Vec<Document>> = HashMap::new();

        for doc in self.documents.clone() {
            grouped_items
                .entry(stringify(&serde_yaml::to_value(doc.get_key(key)).unwrap()))
                .or_default()
                .push(doc);
        }

        grouped_items
            .into_iter()
            .map(|(key, item)| (key, Self { documents: item }))
            .collect()
    }

    /// Creates a table data representation of the documents with specified columns.
    ///
    /// This method constructs a table where each row represents a document and each
    /// column corresponds to a specified key in the front matter. The resulting table
    /// can be used for display or further processing.
    ///
    /// # Arguments
    ///
    /// * `col` - A slice of strings representing the keys to include as columns in the table.
    ///
    /// # Returns
    ///
    /// A `Table` (vector of vectors of strings) where each inner vector represents a row of
    /// the table, and each string represents a cell in the row.
    #[must_use]
    pub fn create_table_data(&self, col: &[String]) -> Table {
        let mut rows = vec![];

        for doc in &self.documents {
            let mut rcol = vec![];
            for c in col {
                rcol.push(stringify(&serde_yaml::to_value(doc.get_key(c)).unwrap()));
            }
            rows.push(rcol);
        }

        rows
    }

    /// Applies filters to the documents of the index, returning a new filtered index.
    ///
    /// This method filters the documents based on the specified [JSON filter](https://crates.io/crates/jsonfilter) criteria,
    /// returning a new `Index` instance containing only the documents that match the filter.
    ///
    /// # Arguments
    ///
    /// * `filters` - A `serde_json::Value` representing the filter criteria.
    ///
    /// # Returns
    ///
    /// A new `Index` containing the filtered documents.
    #[must_use]
    pub fn filter_documents<F: FnMut(&Document) -> bool>(&self, mut filter: F) -> Self {
        let docs: Vec<_> = self
            .documents
            .iter()
            .filter(|x| filter(*x))
            .cloned()
            .collect();

        Self { documents: docs }
    }
}

pub fn filter_jsonfilter(filters: &serde_json::Value, doc: &serde_json::Value) -> bool {
    let res = jsonfilter::try_matches(filters, doc);
    match res {
        Ok(valid) => Ok(valid),
        Err(e) => match e {
            jsonfilter::FilterError::InvalidFilter | jsonfilter::FilterError::UnknownOperator => {
                Err(e)
            }
            jsonfilter::FilterError::KeyNotFound => Ok(false),
        },
    }
    .unwrap()
}

impl Document {
    /// Retrieves the value of a specified key from the document.
    ///
    /// This method first checks internal properties such as file metadata and path information.
    /// If the key does not match any internal properties, it searches the document's front matter.
    /// If the key is not found, it returns a JSON null value.
    ///
    /// # Arguments
    ///
    /// * `key` - A string slice representing the key to retrieve the value for. The key can be
    ///   either an internal property or a front matter field. Nested front matter fields can be
    ///   accessed using dot notation.
    ///
    /// # Returns
    ///
    /// A `serde_json::Value` representing the value associated with the specified key. If the key
    /// is not found, it returns `serde_json::Value::Null`.
    fn get_key(&self, key: &str) -> serde_json::Value {
        match key {
            "file.title" => {
                let path = std::path::Path::new(&self.path);
                return serde_json::Value::String(
                    path.file_stem().unwrap().to_str().unwrap().to_string(),
                );
            }
            "file.name" => {
                let path = std::path::Path::new(&self.path);
                return serde_json::Value::String(
                    path.file_name().unwrap().to_str().unwrap().to_string(),
                );
            }
            "file.parent" => {
                let path = std::path::Path::new(&self.path);
                return serde_json::Value::String(
                    path.parent()
                        .unwrap()
                        .file_name()
                        .unwrap()
                        .to_str()
                        .unwrap()
                        .to_string(),
                );
            }
            "file.folder" => {
                let path = std::path::Path::new(&self.path);
                return serde_json::Value::String(
                    path.parent().unwrap().to_str().unwrap().to_string(),
                );
            }
            "file.ext" => {
                let path = std::path::Path::new(&self.path);
                return serde_json::Value::String(
                    path.extension().unwrap().to_str().unwrap().to_string(),
                );
            }
            "file.size" => {
                let path = std::path::Path::new(&self.path);
                return serde_json::Value::String(path.metadata().unwrap().len().to_string());
            }
            "file.ctime" => {
                let path = std::path::Path::new(&self.path);
                return serde_json::Value::String(
                    system_time_to_date_time(path.metadata().unwrap().created().unwrap())
                        .to_rfc3339(),
                );
            }
            "file.cday" => {
                let path = std::path::Path::new(&self.path);
                return serde_json::Value::String(
                    system_time_to_date_time(path.metadata().unwrap().created().unwrap())
                        .format("%Y-%m-%d")
                        .to_string(),
                );
            }
            "file.mtime" => {
                let path = std::path::Path::new(&self.path);
                return serde_json::Value::String(
                    system_time_to_date_time(path.metadata().unwrap().modified().unwrap())
                        .to_rfc3339(),
                );
            }
            "file.mday" => {
                let path = std::path::Path::new(&self.path);
                return serde_json::Value::String(
                    system_time_to_date_time(path.metadata().unwrap().modified().unwrap())
                        .format("%Y-%m-%d")
                        .to_string(),
                );
            }
            "file.path" => {
                return serde_json::Value::String(self.path.clone());
            }
            _ => {}
        }

        let split_path: Vec<_> = key.split('.').collect();

        if split_path.len() > 1 {
            let data = self
                .frontmatter
                .as_mapping()
                .unwrap()
                .get(split_path.first().unwrap());
            if data.is_none() {
                return serde_json::Value::Null;
            }
            let mut data = data.unwrap();

            for path in &split_path[1..] {
                let data_opt = data.as_mapping().unwrap().get(path);
                if data_opt.is_none() {
                    return serde_json::Value::Null;
                }
                data = data_opt.unwrap();
            }

            serde_json::to_value(data).unwrap()
        } else {
            self.frontmatter.as_mapping().unwrap().get(key).map_or_else(
                || serde_json::Value::Null,
                |x| serde_json::to_value(x).unwrap(),
            )
        }
    }

    /// Retrieves the complete front matter of the document, including additional file metadata.
    ///
    /// This method returns the full front matter of the document as a JSON object, with added
    /// metadata fields such as file name, title, parent directory, folder, extension, size,
    /// creation time, modification time, and path.
    ///
    /// # Returns
    ///
    /// A `serde_json::Value` representing the full front matter of the document, enriched with
    /// additional file metadata.
    #[must_use]
    pub fn get_full_frontmatter(&self) -> serde_json::Value {
        let mut frontmatter = serde_json::to_value(&self.frontmatter).unwrap();
        let frontmatter_obj = frontmatter.as_object_mut().unwrap();
        frontmatter_obj.insert("file.title".into(), self.get_key("file.title"));
        frontmatter_obj.insert("file.name".into(), self.get_key("file.name"));
        frontmatter_obj.insert("file.parent".into(), self.get_key("file.parent"));
        frontmatter_obj.insert("file.folder".into(), self.get_key("file.folder"));
        frontmatter_obj.insert("file.ext".into(), self.get_key("file.ext"));
        frontmatter_obj.insert("file.size".into(), self.get_key("file.size"));
        frontmatter_obj.insert("file.ctime".into(), self.get_key("file.ctime"));
        frontmatter_obj.insert("file.cday".into(), self.get_key("file.cday"));
        frontmatter_obj.insert("file.mtime".into(), self.get_key("file.mtime"));
        frontmatter_obj.insert("file.mday".into(), self.get_key("file.mday"));
        frontmatter_obj.insert("file.path".into(), self.get_key("file.path"));
        frontmatter
    }
}

fn stringify(val: &serde_yaml::Value) -> String {
    match val {
        serde_yaml::Value::Null => String::new(),
        serde_yaml::Value::Bool(b) => b.to_string(),
        serde_yaml::Value::Number(n) => n.to_string(),
        serde_yaml::Value::String(s) => s.to_owned(),
        serde_yaml::Value::Sequence(_) | serde_yaml::Value::Mapping(_) => {
            serde_json::to_string(&val).unwrap()
        }
        serde_yaml::Value::Tagged(_) => unimplemented!(),
    }
}

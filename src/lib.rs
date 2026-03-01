use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use jsaw_core::{
    filter::filter_mongo,
    provider::{Format, Provider},
    record::{value_cmp, FileRecord},
};

// ── Inline tag extraction ─────────────────────────────────────────────────────
// This is the only piece that stays in mdq: jsaw-core's Provider parses
// frontmatter, but inline #hashtags embedded in the document body are a
// markdown-specific concept.

/// Extracts inline `#tags` from a Markdown document body.
///
/// ```
/// use mdq::get_inline_tags;
/// let md = "This document is about #Rust and #Markdown.";
/// assert_eq!(get_inline_tags(md), vec!["Rust", "Markdown"]);
/// ```
#[must_use]
pub fn get_inline_tags(markdown: &str) -> Vec<String> {
    let tag_regex = regex::Regex::new(r"#(\w+)").unwrap();
    tag_regex
        .captures_iter(markdown)
        .filter_map(|c| c.get(1).map(|m| m.as_str().to_string()))
        .collect()
}

// ── Data structures ───────────────────────────────────────────────────────────

/// A single markdown document: its path and parsed frontmatter as JSON.
#[derive(Debug, Clone)]
pub struct Document {
    pub path: String,
    pub frontmatter: serde_json::Value,
}

/// An indexed collection of markdown documents.
#[derive(Debug)]
pub struct Index {
    pub documents: Vec<Document>,
}

type Table = Vec<Vec<String>>;

// ── Index ─────────────────────────────────────────────────────────────────────

impl Index {
    /// Recursively scan `dir` for `.md` files and build an index.
    ///
    /// Frontmatter is parsed by `jsaw-core`'s `Provider`. When `inline_tags`
    /// is true, `#hashtags` from the document body are merged into the
    /// frontmatter `tags` field (deduplicating with any existing tags).
    pub fn new(dir: &str, inline_tags: bool) -> Self {
        let mut i = Self { documents: vec![] };

        for e in walkdir::WalkDir::new(dir)
            .into_iter()
            .filter_map(std::result::Result::ok)
        {
            if !e.path().is_file() {
                continue;
            }
            if e.path().extension().and_then(|s| s.to_str()) != Some("md") {
                continue;
            }

            let path = e.path().to_str().unwrap().to_owned();

            let frontmatter = if inline_tags {
                // Need the raw content for tag extraction.
                let content = match std::fs::read_to_string(&path) {
                    Ok(c) => c,
                    Err(_) => continue,
                };

                // jsaw-core parses the frontmatter — no duplication needed.
                let mut fm = Provider::parse_content(&content, Format::MarkdownFrontmatter)
                    .unwrap_or_else(|| serde_json::json!({}));

                // Merge inline #tags with existing frontmatter tags, preserving order.
                let mut tags: Vec<String> = fm
                    .get("tags")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str().map(String::from))
                            .collect()
                    })
                    .unwrap_or_default();
                tags.extend(get_inline_tags(&content));

                let mut seen = HashSet::new();
                let unique: Vec<serde_json::Value> = tags
                    .into_iter()
                    .filter(|t| seen.insert(t.clone()))
                    .map(serde_json::Value::String)
                    .collect();

                fm.as_object_mut()
                    .unwrap()
                    .insert("tags".into(), serde_json::Value::Array(unique));
                fm
            } else {
                Provider::File(PathBuf::from(&path))
                    .load()
                    .unwrap_or_else(|| serde_json::json!({}))
            };

            log::trace!("Adding {path} to Index");
            i.documents.push(Document { path, frontmatter });
        }

        i
    }

    /// Filter documents using MongoDB-style JSON filter criteria.
    /// Uses `jsaw_core::filter::filter_mongo` — no direct `jsonfilter` dep needed.
    #[must_use]
    pub fn filter_documents(&self, filters: &serde_json::Value) -> Self {
        let docs: Vec<_> = self
            .documents
            .iter()
            .filter(|doc| {
                filter_mongo(&doc.get_full_frontmatter(), filters).expect("invalid filter")
            })
            .cloned()
            .collect();
        Self { documents: docs }
    }

    /// Sort, reverse, and paginate the index.
    #[must_use]
    pub fn apply(&self, limit: usize, offset: usize, sort: Option<String>, reverse: bool) -> Self {
        let mut scope = self.documents.clone();

        if let Some(key) = sort {
            // value_cmp from jsaw-core handles numeric, string, null ordering.
            scope.sort_by(|a, b| value_cmp(&a.get_key(&key), &b.get_key(&key)));
        }
        if reverse {
            scope.reverse();
        }

        let scope = scope.into_iter().skip(offset);
        Self {
            documents: if limit == 0 {
                scope.collect()
            } else {
                scope.take(limit).collect()
            },
        }
    }

    /// Group documents by a frontmatter key, returning one sub-index per value.
    #[must_use]
    pub fn group_by(&self, key: &str) -> HashMap<String, Self> {
        let mut grouped: HashMap<String, Vec<Document>> = HashMap::new();
        for doc in &self.documents {
            grouped
                .entry(stringify(&doc.get_key(key)))
                .or_default()
                .push(doc.clone());
        }
        grouped
            .into_iter()
            .map(|(k, docs)| (k, Self { documents: docs }))
            .collect()
    }

    /// Project the specified columns into a 2-D table of strings for display.
    #[must_use]
    pub fn create_table_data(&self, col: &[String]) -> Table {
        self.documents
            .iter()
            .map(|doc| col.iter().map(|c| stringify(&doc.get_key(c))).collect())
            .collect()
    }
}

// ── Document ──────────────────────────────────────────────────────────────────

impl Document {
    /// Resolve a key. Delegates computed `file.*` keys and dot-notation to
    /// `jsaw_core::record::FileRecord::get()` — no duplication of that logic here.
    pub fn get_key(&self, key: &str) -> serde_json::Value {
        FileRecord::new(PathBuf::from(&self.path), self.frontmatter.clone()).get(key)
    }

    /// Returns the frontmatter enriched with all `file.*` fields as flat keys,
    /// ready for use with MongoDB-style filters via `filter_mongo`.
    #[must_use]
    pub fn get_full_frontmatter(&self) -> serde_json::Value {
        FileRecord::new(PathBuf::from(&self.path), self.frontmatter.clone()).mongo_context()
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn stringify(val: &serde_json::Value) -> String {
    match val {
        serde_json::Value::Null => String::new(),
        serde_json::Value::Bool(b) => b.to_string(),
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::String(s) => s.clone(),
        other => serde_json::to_string(other).unwrap_or_default(),
    }
}

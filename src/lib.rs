use std::collections::HashMap;

use txd::DataType;

/// get frontmatter from markdown document
#[must_use]
pub fn get_frontmatter(markdown: &str) -> Option<String> {
    let frontmatter_regex = regex::Regex::new(r"(?s)^---\s*\n(.*?)\n---").unwrap();

    frontmatter_regex.captures(markdown).and_then(|captures| {
        let frontmatter = captures.get(1).map(|m| m.as_str().to_string());

        frontmatter
    })
}

/// get inline #tags from markdown file
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

#[derive(Debug, Clone)]
pub struct Document {
    pub path: String,
    pub frontmatter: serde_yaml::Value,
}

#[derive(Debug)]
pub struct Index {
    pub documents: Vec<Document>,
}

type Table = Vec<Vec<String>>;

impl Index {
    /// Create a markdown document index over `dir`
    pub fn new(dir: &str, ignore_inline_tags: bool) -> Self {
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

                if !ignore_inline_tags {
                    let tags = get_inline_tags(&content);

                    frontmatter
                        .as_mapping_mut()
                        .unwrap()
                        .entry("tags".into())
                        .or_insert(tags.clone().into())
                        .as_sequence_mut()
                        .unwrap()
                        .extend::<Vec<serde_yaml::Value>>(
                            tags.iter().map(|x| x.clone().into()).collect(),
                        );
                }

                let doc = Document { path, frontmatter };
                i.documents.push(doc);
            }
        }

        i
    }

    /// Build a table with specified columns from index within specified scope
    #[must_use]
    pub fn apply(&self, limit: usize, offset: usize, sort: Option<String>, reverse: bool) -> Self {
        let mut scope = self.documents.clone();

        if let Some(sort) = sort {
            scope.sort_by(|a, b| {
                let a_str = a.get_key(&sort);
                let b_str = b.get_key(&sort);
                let mut a = txd::parse(&a_str);
                let mut b = txd::parse(&b_str);

                log::debug!("Trying to order {a:?} and {b:?}",);

                if !a.same_as(&b) {
                    log::debug!("trying to cast a to string because of different types");
                    a = txd::DataType::String(a_str);
                    b = txd::DataType::String(b_str);
                }

                a.order_with(&b).unwrap()
            });
        }

        if reverse {
            scope.reverse();
        }

        let scope: Vec<_> = scope.into_iter().skip(offset).collect();

        let scope = if limit == 0 {
            scope
        } else {
            scope.into_iter().take(limit).collect()
        };

        Self { documents: scope }
    }

    #[must_use]
    pub fn group_by(&self, key: &str) -> HashMap<String, Self> {
        let mut grouped_items: HashMap<String, Vec<Document>> = HashMap::new();

        for doc in self.documents.clone() {
            grouped_items.entry(doc.get_key(key)).or_default().push(doc);
        }

        grouped_items
            .into_iter()
            .map(|(key, item)| (key, Index { documents: item }))
            .collect()
    }

    #[must_use]
    pub fn create_table_data(&self, col: &[String]) -> Table {
        let mut rows = vec![];

        for doc in &self.documents {
            let mut rcol = vec![];
            for c in col {
                rcol.push(doc.get_key(c));
            }
            rows.push(rcol);
        }

        rows
    }

    /// Apply filters to the documents of the index returning a new filtered index
    #[must_use]
    pub fn filter_documents(&self, filters: &[txd::filter::Filter]) -> Self {
        // TODO : Implement option for chaining filters with AND OR
        let docs: Vec<_> = self
            .documents
            .iter()
            .filter(|x| {
                let mut is_included = true;

                for f in filters {
                    let a_str = x.get_key(&f.0);
                    let mut a = txd::parse(&a_str);
                    let b = txd::parse(&f.2);

                    log::debug!(
                        "Trying to compare '{}' = {a:?} and {b:?} with {:?}",
                        f.0,
                        f.1
                    );

                    if a_str.is_empty() {
                        // TODO : Maybe add explicit null instead of empty string
                        is_included = false;
                        break;
                    }

                    if !a.same_as(&b) && !matches!(a, DataType::List(_)) {
                        log::debug!("trying to cast a to string because of different types");
                        a = txd::DataType::String(a_str);
                    }

                    if !a.compare(f.1, &b).unwrap() {
                        is_included = false;
                    }
                }

                is_included
            })
            .cloned()
            .collect();

        Self { documents: docs }
    }
}

impl Document {
    /// Get a key from document.
    /// This will return internal properties first, then it will search the document frontmatter for the key and return it. If nothing was found an empty string is returned.
    fn get_key(&self, key: &str) -> String {
        match key {
            "file.title" => {
                let path = std::path::Path::new(&self.path);
                return path.file_stem().unwrap().to_str().unwrap().to_string();
            }
            "file.name" => {
                let path = std::path::Path::new(&self.path);
                return path.file_name().unwrap().to_str().unwrap().to_string();
            }
            "file.parent" => {
                let path = std::path::Path::new(&self.path);
                return path
                    .parent()
                    .unwrap()
                    .file_name()
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .to_string();
            }
            "file.folder" => {
                let path = std::path::Path::new(&self.path);
                return path.parent().unwrap().to_str().unwrap().to_string();
            }
            "file.ext" => {
                let path = std::path::Path::new(&self.path);
                return path.extension().unwrap().to_str().unwrap().to_string();
            }
            "file.size" => {
                let path = std::path::Path::new(&self.path);
                return path.metadata().unwrap().len().to_string();
            }
            "file.ctime" => {
                let path = std::path::Path::new(&self.path);
                return system_time_to_date_time(path.metadata().unwrap().created().unwrap())
                    .to_rfc3339();
            }
            "file.cday" => {
                let path = std::path::Path::new(&self.path);
                return system_time_to_date_time(path.metadata().unwrap().created().unwrap())
                    .format("%Y-%m-%d")
                    .to_string();
            }
            "file.mtime" => {
                let path = std::path::Path::new(&self.path);
                return system_time_to_date_time(path.metadata().unwrap().modified().unwrap())
                    .to_rfc3339();
            }
            "file.mday" => {
                let path = std::path::Path::new(&self.path);
                return system_time_to_date_time(path.metadata().unwrap().modified().unwrap())
                    .format("%Y-%m-%d")
                    .to_string();
            }
            "file.path" => {
                return self.path.clone();
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
                return String::new();
            }
            let mut data = data.unwrap();

            for path in &split_path[1..] {
                let data_opt = data.as_mapping().unwrap().get(path);
                if data_opt.is_none() {
                    return String::new();
                }
                data = data_opt.unwrap();
            }

            stringify(data)
        } else {
            self.frontmatter
                .as_mapping()
                .unwrap()
                .get(key)
                .map_or_else(String::new, stringify)
        }
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

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
    pub fn new(dir: &str) -> Self {
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
                let frontmatter = get_frontmatter(&content);
                if let Some(frontmatter) = frontmatter {
                    let frontmatter = serde_yaml::from_str(&frontmatter).unwrap();
                    let doc = Document { path, frontmatter };
                    i.documents.push(doc);
                } else {
                    i.documents.push(Document {
                        path,
                        frontmatter: serde_yaml::to_value(&serde_yaml::Mapping::new()).unwrap(),
                    });
                }
            }
        }

        i
    }

    /// Build a table with specified columns from index within specified scope
    #[must_use]
    pub fn select_columns(&self, col: &[String], limit: usize, offset: usize) -> Table {
        let mut rows = vec![];

        let scope: Vec<_> = self.documents.clone().into_iter().skip(offset).collect();

        let scope = if limit == 0 {
            scope
        } else {
            scope.into_iter().take(limit).collect()
        };

        for doc in scope {
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

                    log::debug!("Trying to compare {a:?} and {b:?} with {:?}", f.1);

                    if a_str.is_empty() {
                        // TODO : Maybe add explicit null instead of empty string
                        is_included = false;
                        break;
                    }

                    if !a.same_as(&b) && !matches!(a, DataType::List(_)) {
                        log::debug!("trying to cast a to string because of different types");
                        a = txd::DataType::String(a_str);
                    }

                    if !a.compare(f.1, b) {
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
        self.frontmatter
            .as_mapping()
            .unwrap()
            .get(key)
            .map_or_else(String::new, stringify)
    }
}

fn stringify(val: &serde_yaml::Value) -> String {
    match val {
        serde_yaml::Value::Null => String::new(),
        serde_yaml::Value::Bool(b) => b.to_string(),
        serde_yaml::Value::Number(n) => n.to_string(),
        serde_yaml::Value::String(s) => s.to_owned(),
        serde_yaml::Value::Sequence(_) => serde_json::to_string(&val).unwrap(),
        serde_yaml::Value::Mapping(_o) => todo!(),
        serde_yaml::Value::Tagged(_) => unimplemented!(),
    }
}

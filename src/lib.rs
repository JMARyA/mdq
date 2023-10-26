/// get frontmatter from markdown document
#[must_use]
pub fn get_frontmatter(markdown: &str) -> Option<String> {
    let frontmatter_regex = regex::Regex::new(r"(?s)^---\s*\n(.*?)\n---").unwrap();

    if let Some(captures) = frontmatter_regex.captures(markdown) {
        let frontmatter = captures.get(1).map(|m| m.as_str().to_string());

        frontmatter
    } else {
        None
    }
}

#[derive(Debug)]
pub struct Document {
    pub path: String,
    pub frontmatter: serde_yaml::Value,
}

#[derive(Debug)]
pub struct Index {
    pub documents: Vec<Document>,
}

/// Create a markdown document index over `dir`
pub fn scan_dir(dir: &str) -> Index {
    let mut i = Index { documents: vec![] };

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

/// Get a key from document.
/// This will return internal properties first, then it will search the document frontmatter for the key and return it. If nothing was found an empty string is returned.
fn get_key(d: &Document, key: &str) -> String {
    if key == "file.title" {
        let path = std::path::Path::new(&d.path);
        return path.file_stem().unwrap().to_str().unwrap().to_string();
    }
    if let Some(val) = d.frontmatter.as_mapping().unwrap().get(key) {
        // TODO : Fix format
        match val {
            serde_yaml::Value::Null => String::new(),
            serde_yaml::Value::Bool(b) => b.to_string(),
            serde_yaml::Value::Number(n) => n.to_string(),
            serde_yaml::Value::String(s) => s.to_owned(),
            serde_yaml::Value::Sequence(_v) => todo!(),
            serde_yaml::Value::Mapping(_o) => todo!(),
            serde_yaml::Value::Tagged(_) => unimplemented!(),
        }
    } else {
        String::new()
    }
}

type Table = Vec<Vec<String>>;

/// Build a table with specified columns from index
#[must_use]
pub fn select_columns(i: &Index, col: &[String]) -> Table {
    let mut rows = vec![];

    for doc in &i.documents {
        let mut rcol = vec![];
        for c in col {
            rcol.push(get_key(doc, c));
        }
        rows.push(rcol);
    }

    rows
}

/// Apply filters to the documents of the index returning a new filtered index
#[must_use]
pub fn filter_documents(i: Index, filters: &[txd::filter::Filter]) -> Index {
    // TODO : Implement option for chaining filters with AND OR
    let docs: Vec<_> = i
        .documents
        .into_iter()
        .filter_map(|x| {
            let mut is_included = true;

            for f in filters {
                let a_str = get_key(&x, &f.0);
                let mut a = txd::parse(&a_str);
                let b = txd::parse(&f.2);

                log::debug!("Trying to compare {a:?} and {b:?} with {:?}", f.1);

                if a_str.is_empty() {
                    // TODO : Maybe add explicit null instead of empty string
                    is_included = false;
                    break;
                }

                if !a.same_as(&b) {
                    log::debug!("trying to cast a to string because of different types");
                    a = txd::DataType::String(a_str);
                }

                if !a.compare(f.1, b) {
                    is_included = false
                }
            }
            if is_included {
                Some(x)
            } else {
                None
            }
        })
        .collect();

    Index { documents: docs }
}

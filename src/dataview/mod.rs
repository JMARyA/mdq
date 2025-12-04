pub mod query;
pub mod transform;

pub fn preprocess(file: String, root: Option<String>) -> String {
    let f = std::fs::read_to_string(&file).unwrap();

    let root = match root {
        Some(r) => r,
        None => {
            // normalize path so parent() works reliably
            let path = std::path::Path::new(&file)
                .canonicalize()
                .unwrap_or_else(|_| file.clone().into());

            let parent = path.parent().unwrap_or_else(|| std::path::Path::new("."));
            parent.to_string_lossy().into_owned()
        }
    };

    transform::eval_dataview_blocks(&f, root)
}

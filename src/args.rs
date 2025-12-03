use crate::quit_err;
use clap::{Parser, Subcommand};
use serde_json::Value;

/// Command-line arguments
#[derive(Parser, Debug)]
#[command(name = "Markdown Query", about = "Query markdown files", version)]
pub struct Args {
    #[command(subcommand)]
    pub command: Option<Commands>,

    /// Directory to scan
    #[arg(default_value = ".")]
    pub root_dir: String,

    /// Output result as JSON
    #[arg(short, long)]
    pub output_json: bool,

    /// Don't print header in CSV mode. Useful for scripting
    #[arg(long)]
    pub no_header: bool,

    /// Limit number of results returned
    #[arg(short, long, default_value = "0")]
    pub limit: usize,

    /// Offset results by a factor. Useful when used with --limit
    #[arg(long, default_value = "0")]
    pub offset: usize,

    /// Filter to apply to the documents (JSON format)
    #[arg(short, long)]
    pub filter: Vec<String>,

    /// Specify output columns. You can rename headers using `:` like `VariableName:OutputName`
    #[arg(short, long, default_value = "file.title:Title")]
    pub column: Vec<String>,

    /// Sort results based on specified key
    #[arg(short, long)]
    pub sort_by: Option<String>,

    /// Group results based on specified key
    #[arg(short, long)]
    pub group_by: Option<String>,

    /// Reverse the results
    #[arg(short, long)]
    pub reversed: bool,

    /// Include inline #tags in tags frontmatter
    #[arg(short = 't', long)]
    pub use_inline_tags: bool,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Preprocess a single markdown file for dataview blocks
    Preprocess {
        /// File to preprocess
        file: String,

        /// Optional root directory
        #[arg(long)]
        root: Option<String>,
    },
}

impl Args {
    /// Parses filters and returns a single `serde_json::Value`
    pub fn parsed_filters(&self) -> Value {
        if self.filter.is_empty() {
            return serde_json::json!({});
        }

        if self.filter.len() == 1 {
            serde_json::from_str(&self.filter[0]).unwrap_or_else(|e| {
                quit_err(
                    e,
                    &format!("filter '{}' could not be parsed", &self.filter[0]),
                )
            })
        } else {
            let filters: Vec<_> = self
                .filter
                .iter()
                .map(|f| {
                    serde_json::from_str::<Value>(f).unwrap_or_else(|e| {
                        quit_err(e, &format!("filter '{}' could not be parsed", f))
                    })
                })
                .collect();
            serde_json::json!({ "$and": filters })
        }
    }

    /// Returns column names and optional renamed headers
    pub fn columns_and_headers(&self) -> (Vec<String>, Vec<String>) {
        let (columns, headers): (Vec<_>, Vec<_>) = self
            .column
            .iter()
            .map(|x| {
                let (col, header) = x.split_once(':').unwrap_or((&x, &x));
                (col.to_string(), header.to_string())
            })
            .unzip();
        (columns, headers)
    }
}

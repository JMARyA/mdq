use clap::{arg, command, ArgMatches};

use crate::quit_err;

pub struct Args {
    pub root_dir: String,
    pub output_json: bool,
    pub no_header: bool,
    pub limit: usize,
    pub offset: usize,
    pub use_inline_tags: bool,
    pub extract_tasks: bool,
    pub sort_by: Option<String>,
    pub group_by: Option<String>,
    pub reversed: bool,
    pub columns: Vec<String>,
    pub headers: Vec<String>,
    pub filters: serde_json::Value,
}

pub fn get_args() -> Args {
    let args = get_args_match();

    let root_dir = args.get_one::<String>("dir").unwrap();

    let output_json = args.get_flag("json");

    let no_header = args.get_flag("noheader");

    let limit: usize = args
        .get_one::<String>("limit")
        .unwrap()
        .parse()
        .unwrap_or_else(|e| quit_err(e, "Limit is not a number"));

    let offset: usize = args
        .get_one::<String>("offset")
        .unwrap()
        .parse()
        .unwrap_or_else(|e| quit_err(e, "Offset is not a number"));

    let use_inline_tags: bool = args.get_flag("inline-tags");
    let extract_tasks: bool = args.get_flag("tasks");

    let sort_by = args
        .get_one::<String>("sortby")
        .map(std::borrow::ToOwned::to_owned);

    let group_by = args
        .get_one::<String>("groupby")
        .map(std::borrow::ToOwned::to_owned);

    let reversed = args.get_flag("reverse");

    let columns: Vec<_> = args
        .get_many::<String>("column")
        .unwrap()
        .cloned()
        .collect();
    log::debug!("columns: {columns:?}");

    let (columns, headers): (Vec<_>, Vec<_>) = columns
        .into_iter()
        .map(|x| {
            let (column, header_rename) = x.split_once(':').unwrap_or((&x, &x));

            (column.to_owned(), header_rename.to_owned())
        })
        .unzip();

    if columns != headers {
        log::debug!("renamed headers: {headers:?}");
    }

    let filters = args
        .get_many::<String>("filter")
        .map_or_else(std::vec::Vec::new, std::iter::Iterator::collect);

    log::debug!("raw filters: {filters:?}");

    let filters = if filters.len() == 1 {
        let filter = filters.first().unwrap();
        serde_json::from_str(filter)
            .unwrap_or_else(|e| quit_err(e, &format!("filter '{filter}' could not be parsed")))
    } else {
        let filters: Vec<_> = filters
            .iter()
            .map(|x| {
                serde_json::from_str::<serde_json::Value>(x)
                    .unwrap_or_else(|e| quit_err(e, &format!("filter '{x}' could not be parsed")))
            })
            .collect();
        serde_json::json!({
            "$and": filters
        })
    };

    log::debug!("parsed filters: {filters:?}");

    Args {
        root_dir: root_dir.to_string(),
        output_json,
        no_header,
        limit,
        offset,
        use_inline_tags,
        extract_tasks,
        sort_by,
        group_by,
        reversed,
        columns,
        headers,
        filters,
    }
}

fn get_args_match() -> ArgMatches {
    command!()
        .about("Query markdown files")
        .arg(arg!([dir] "Directory to scan").required(false).default_value("."))
        .arg(arg!(-j --json "Output result as JSON").required(false))
        .arg(
            arg!(-l --limit <LIMIT> "Limit number of results returned")
                .required(false)
                .default_value("0")
                .allow_negative_numbers(false),
        )
        .arg(
            arg!(--offset <OFFSET> "Offset results by a factor. Useful when used with --limit")
                .required(false)
                .allow_negative_numbers(false)
                .default_value("0"),
        )
        .arg(arg!(-f --filter <FILTER>... "Filter to apply to the documents").required(false))
        .arg(
            arg!(-c --column <COLUMN>... "Specify output columns. You can rename the text displayed in the header using the `:` character like this: VariableName:OutputName")
                .required(false)
                .default_value("file.title:Title"),
        )
        .arg(arg!(-s --sortby <KEY> "Sort results based on specified key").required(false))
        .arg(arg!(-g --groupby <KEY> "Group results based on specified key").required(false))
        .arg(arg!(-r --reverse "Reverse the results").required(false))
        .arg(arg!(--noheader "Dont print header in CSV mode. Useful for scripting").required(false))
        .arg(clap::Arg::new("inline-tags").short('t').long("inline-tags").help("Include inline #tags in tags frontmatter").required(false).num_args(0))
        .arg(clap::Arg::new("tasks").long("tasks").help("Extract GFM task list items into tasks, tasks_done and tasks_open fields").required(false).num_args(0))
        .get_matches()
}

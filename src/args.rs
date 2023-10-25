use clap::{arg, command, ArgMatches};

pub fn get_args() -> ArgMatches {
    command!()
        .about("Query markdown files")
        .arg(arg!([dir] "Directory to scan").required(true))
        .arg(arg!(-j --json "Output result as JSON").required(false))
        .arg(arg!(-l --limit <LIMIT> "Limit number of results returned").required(false))
        .arg(arg!(-f --filter <FILTER>... "Filter to apply to the documents").required(false))
        .arg(
            arg!(-c --column <COLUMN>... "Specify output columns")
                .required(false)
                .default_value("path"),
        )
        .get_matches()
}

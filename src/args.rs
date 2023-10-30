use clap::{arg, command, ArgMatches};

pub fn get_args() -> ArgMatches {
    command!()
        .about("Query markdown files")
        .arg(arg!([dir] "Directory to scan").required(true))
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
        .get_matches()
}

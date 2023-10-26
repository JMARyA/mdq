use std::io::IsTerminal;

use mdq::{filter_documents, scan_dir, select_columns};

mod args;

// TODO : Add debug logging
// TODO : Add documentation comments
// TODO : Add tests

fn main() {
    env_logger::init();
    let args = args::get_args();

    let root_dir = args.get_one::<String>("dir").unwrap();

    let output_json = args.get_flag("json");

    let limit: Option<usize> = if let Some(limit_arg) = args.get_one::<String>("limit") {
        limit_arg.parse().ok()
    } else {
        None
    };

    let columns: Vec<_> = args
        .get_many::<String>("column")
        .unwrap()
        .cloned()
        .collect();
    log::info!("selected columns: {columns:?}");

    let columns: Vec<(_, _)> = columns
        .into_iter()
        .map(|x| {
            let (column, header_rename) = x.split_once(':').unwrap_or((&x, &x));

            (column.to_owned(), header_rename.to_owned())
        })
        .collect();

    let (columns, headers): (Vec<_>, Vec<_>) = columns.into_iter().unzip();

    let filters: Vec<_> = if let Some(filters) = args.get_many::<String>("filter") {
        filters.collect()
    } else {
        vec![]
    };

    let filters: Vec<_> = filters
        .into_iter()
        .map(|x| txd::filter::parse_condition(x).expect("failed to parse filter"))
        .collect();

    let mut i = scan_dir(root_dir);
    if !filters.is_empty() {
        i = filter_documents(i, &filters);
    }

    let data = if let Some(limit) = limit {
        select_columns(&i, &columns.clone())
            .into_iter()
            .take(limit)
            .collect::<Vec<_>>()
    } else {
        select_columns(&i, &columns.clone())
    };

    if output_json {
        let mut data = serde_json::json!(
            {
                "columns": columns,
                "results": data
            }
        );
        if columns != headers {
            data.as_object_mut()
                .unwrap()
                .insert("headers".into(), headers.into());
        }
        println!("{}", serde_json::to_string(&data).unwrap());
        return;
    }

    if data.is_empty() {
        return;
    }

    let mut table = comfy_table::Table::new();

    table.set_header(headers);
    table.load_preset(comfy_table::presets::UTF8_FULL_CONDENSED);
    if !std::io::stdout().is_terminal() {
        // TODO : Output as CSV?
        table.load_preset(comfy_table::presets::NOTHING);
    }
    table.add_rows(data);

    println!("{table}");
}

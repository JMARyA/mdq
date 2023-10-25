use mdq::{filter_documents, scan_dir, select_columns};
use prettytable::{Cell, Row};

mod args;

fn main() {
    let args = args::get_args();
    //println!("{args:?}");

    let root_dir = args.get_one::<String>("dir").unwrap();

    let output_json = args.get_flag("json");

    let limit: Option<usize> = if let Some(limit_arg) = args.get_one::<String>("limit") {
        limit_arg.parse().ok()
    } else {
        None
    };

    let columns: Vec<_> = if let Some(columns) = args.get_many::<String>("column") {
        columns.collect()
    } else {
        vec![]
    };
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
        let data = serde_json::json!(
            {
                "columns": columns,
                "results": data
            }
        );
        println!("{}", serde_json::to_string(&data).unwrap());
        return;
    }

    let mut table = prettytable::Table::new();

    let headers = columns
        .iter()
        .map(|cell| Cell::new(cell))
        .collect::<Vec<Cell>>();
    table.set_titles(Row::new(headers));

    // Add rows to the table
    for row in data {
        let cells: Vec<Cell> = row.iter().map(|cell| Cell::new(cell)).collect();
        table.add_row(Row::new(cells));
    }

    // Print the table to the console
    table.printstd();
}

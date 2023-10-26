use std::io::IsTerminal;

use mdq::Index;

mod args;

// TODO : Add debug logging
// TODO : Add documentation comments
// TODO : Add tests
// TODO : Add GROUP BY Function

fn main() {
    env_logger::init();
    let args = args::get_args();

    let root_dir = args.get_one::<String>("dir").unwrap();

    let output_json = args.get_flag("json");

    let limit: usize = args.get_one::<String>("limit").unwrap().parse().unwrap();

    let offset: usize = args.get_one::<String>("offset").unwrap().parse().unwrap();

    let sort_by = args.get_one::<String>("sortby").map(|x| x.to_owned());

    let reversed = args.get_flag("reverse");

    let columns: Vec<_> = args
        .get_many::<String>("column")
        .unwrap()
        .cloned()
        .collect();
    log::info!("selected columns: {columns:?}");

    let (columns, headers): (Vec<_>, Vec<_>) = columns
        .into_iter()
        .map(|x| {
            let (column, header_rename) = x.split_once(':').unwrap_or((&x, &x));

            (column.to_owned(), header_rename.to_owned())
        })
        .unzip();

    let filters = args
        .get_many::<String>("filter")
        .map_or_else(std::vec::Vec::new, std::iter::Iterator::collect);

    let filters: Vec<_> = filters
        .into_iter()
        .map(|x| txd::filter::parse_condition(x).expect("failed to parse filter"))
        .collect();

    let mut i = Index::new(root_dir);
    if !filters.is_empty() {
        i = i.filter_documents(&filters);
    }

    let data = i.select_columns(&columns, limit, offset, sort_by, reversed);

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

    if !std::io::stdout().is_terminal() {
        let mut writer = csv::WriterBuilder::new().from_writer(vec![]);
        writer.write_record(headers).unwrap();
        for e in data {
            writer.write_record(e).unwrap();
        }
        print!(
            "{}",
            String::from_utf8(writer.into_inner().unwrap()).unwrap()
        );
        return;
    }

    let mut table = comfy_table::Table::new();

    table.set_header(headers);
    table.load_preset(comfy_table::presets::UTF8_FULL_CONDENSED);
    table.add_rows(data);

    println!("{table}");
}

use std::{collections::HashMap, io::IsTerminal};

use mdq::Index;

mod args;

fn main() {
    env_logger::init();
    let args = args::get_args();

    let root_dir = args.get_one::<String>("dir").unwrap();

    let output_json = args.get_flag("json");

    let no_header = args.get_flag("noheader");

    let limit: usize = args.get_one::<String>("limit").unwrap().parse().unwrap();

    let offset: usize = args.get_one::<String>("offset").unwrap().parse().unwrap();

    let ignoretags: bool = args.get_flag("ignoretags");

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
        serde_json::from_str(filters.first().unwrap()).unwrap()
    } else {
        let filters: Vec<_> = filters
            .iter()
            .map(|x| serde_json::from_str::<serde_json::Value>(x).unwrap())
            .collect();
        serde_json::json!({
            "$and": filters
        })
    };

    log::debug!("parsed filters: {filters:?}");

    let mut i = Index::new(root_dir, ignoretags);
    if !filters.is_null() {
        i = i.filter_documents(&filters);
    }

    i = i.apply(limit, offset, sort_by, reversed);

    if group_by.is_some() {
        let grouped = i.group_by(&group_by.clone().unwrap());
        let grouped: HashMap<_, _> = grouped
            .into_iter()
            .map(|(key, val)| (key, val.create_table_data(&columns)))
            .collect();

        if output_json {
            let mut data = serde_json::json!(
                {
                    "columns": columns,
                    "groupby": group_by.unwrap(),
                    "results": grouped
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

        if std::io::stdout().is_terminal() {
            let mut grouped_keys = grouped.iter().map(|(key, _)| key).collect::<Vec<_>>();
            grouped_keys.sort_by(|a_str, b_str| {
                let a: serde_json::Value = serde_json::from_str(a_str).unwrap();
                let b: serde_json::Value = serde_json::from_str(b_str).unwrap();

                jsonfilter::order(&a, &b)
            });
            for group in grouped_keys {
                println!("# {group}");
                print_result(grouped.get(group).unwrap().clone(), &headers);
            }
        } else {
            let mut first = true;
            for (_, val) in grouped {
                if first {
                    print_csv(val, if no_header { None } else { Some(&headers) });
                    first = false;
                    continue;
                }
                print_csv(val, None);
            }
        }
        return;
    }

    let data = i.create_table_data(&columns);

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
    if std::io::stdout().is_terminal() {
        print_result(data, &headers);
    } else {
        print_csv(data, if no_header { None } else { Some(&headers) });
    }
}

fn print_result(data: Vec<Vec<String>>, headers: &[String]) {
    if data.is_empty() {
        return;
    }

    let mut table = comfy_table::Table::new();

    table.set_header(headers);
    table.load_preset(comfy_table::presets::UTF8_FULL_CONDENSED);
    table.add_rows(data);

    println!("{table}");
}

fn print_csv(data: Vec<Vec<String>>, headers: Option<&[String]>) {
    let mut writer = csv::WriterBuilder::new().from_writer(vec![]);
    if let Some(headers) = headers {
        writer.write_record(headers).unwrap();
    }
    for e in data {
        writer.write_record(e).unwrap();
    }
    print!(
        "{}",
        String::from_utf8(writer.into_inner().unwrap()).unwrap()
    );
}

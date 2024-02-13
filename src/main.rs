use std::{collections::HashMap, io::IsTerminal};

use mdq::Index;

mod args;

pub fn quit_err(e: impl std::error::Error, msg: &str) -> ! {
    eprintln!("Error: {msg}. {e}");
    std::process::exit(1);
}

fn main() {
    env_logger::init();
    let args = args::get_args();

    let mut i = Index::new(&args.root_dir, args.ignoretags);
    if !args.filters.is_null() {
        i = i.filter_documents(&args.filters);
    }

    i = i.apply(args.limit, args.offset, args.sort_by, args.reversed);

    if args.group_by.is_some() {
        let grouped = i.group_by(&args.group_by.clone().unwrap());
        let grouped: HashMap<_, _> = grouped
            .into_iter()
            .map(|(key, val)| (key, val.create_table_data(&args.columns)))
            .collect();

        if args.output_json {
            let mut data = serde_json::json!(
                {
                    "columns": args.columns,
                    "groupby": args.group_by.unwrap(),
                    "results": grouped
                }
            );
            if args.columns != args.headers {
                data.as_object_mut()
                    .unwrap()
                    .insert("headers".into(), args.headers.into());
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
                print_result(grouped.get(group).unwrap().clone(), &args.headers);
            }
        } else {
            let mut first = true;
            for (_, val) in grouped {
                if first {
                    print_csv(
                        val,
                        if args.no_header {
                            None
                        } else {
                            Some(&args.headers)
                        },
                    );
                    first = false;
                    continue;
                }
                print_csv(val, None);
            }
        }
        return;
    }

    let data = i.create_table_data(&args.columns);

    if args.output_json {
        let mut data = serde_json::json!(
            {
                "columns": args.columns,
                "results": data
            }
        );
        if args.columns != args.headers {
            data.as_object_mut()
                .unwrap()
                .insert("headers".into(), args.headers.into());
        }
        println!("{}", serde_json::to_string(&data).unwrap());
        return;
    }
    if std::io::stdout().is_terminal() {
        print_result(data, &args.headers);
    } else {
        print_csv(
            data,
            if args.no_header {
                None
            } else {
                Some(&args.headers)
            },
        );
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

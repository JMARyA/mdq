use std::{collections::HashMap, io::IsTerminal};

use clap::Parser;
use mdq::Index;

mod args;
mod dataview;

pub fn quit_err(e: impl std::error::Error, msg: &str) -> ! {
    eprintln!("Error: {msg}. {e}");
    std::process::exit(1);
}

fn main() {
    env_logger::init();
    let args = args::Args::parse();

    if let Some(args::Commands::Preprocess { file, root }) = args.command {
        let processed = dataview::preprocess(file, root);
        println!("{processed}");
        return;
    }

    let root_dir = if args.root_dir == "." {
        let cwd = std::env::current_dir().unwrap();
        cwd.to_str().unwrap().to_string()
    } else {
        args.root_dir.clone()
    };

    let mut i = Index::new(&root_dir, args.use_inline_tags);
    if !args.parsed_filters().is_null() {
        i = i.filter_documents(&args.parsed_filters());
    }

    i = i.apply(args.limit, args.offset, args.sort_by.clone(), args.reversed);

    let (columns, headers) = args.columns_and_headers();

    if args.group_by.is_some() {
        let grouped = i.group_by(&args.group_by.clone().unwrap());
        let grouped: HashMap<_, _> = grouped
            .into_iter()
            .map(|(key, val)| (key, val.create_table_data(&columns)))
            .collect();

        if args.output_json {
            let mut data = serde_json::json!(
                {
                    "columns": columns,
                    "groupby": args.group_by.unwrap(),
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
                    print_csv(val, if args.no_header { None } else { Some(&headers) });
                    first = false;
                    continue;
                }
                print_csv(val, None);
            }
        }
        return;
    }

    let data = i.create_table_data(&columns);

    if args.output_json {
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
        print_csv(data, if args.no_header { None } else { Some(&headers) });
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

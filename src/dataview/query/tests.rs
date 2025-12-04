use super::*;

/// Macro to generate DataviewQuery parse tests
macro_rules! test_query_parse {
    ($name:ident, $input:expr, $expected:expr) => {
        #[test]
        fn $name() {
            let (_, parsed) = DataviewQuery::parse($input).unwrap();
            assert_eq!(parsed, $expected);
        }
    };
}

/// Macro to generate WhereFilter parse tests
macro_rules! test_wherefilter_parse {
    ($name:ident, $input:expr, $expected:expr) => {
        #[test]
        fn $name() {
            let (_, parsed) = WhereFilter::parse($input).unwrap();
            assert_eq!(parsed, $expected);
        }
    };
}

test_query_parse!(
    basic_list,
    r#"list"#,
    DataviewQuery {
        kind: QueryKind::List,
        selection: Selection::default(),
        from_clause: FromSource::Folder("/".to_string()),
        sort_clause: None,
        limit: None,
        where_clause: None,
    }
);

test_query_parse!(
    basic_table,
    r#"table"#,
    DataviewQuery {
        kind: QueryKind::Table,
        selection: Selection::default(),
        from_clause: FromSource::Folder("/".to_string()),
        sort_clause: None,
        limit: None,
        where_clause: None,
    }
);

test_query_parse!(
    task_query,
    r#"TASK"#,
    DataviewQuery {
        kind: QueryKind::Task,
        selection: Selection::default(),
        from_clause: FromSource::Folder("/".to_string()),
        sort_clause: None,
        limit: None,
        where_clause: None,
    }
);

test_query_parse!(
    table_recipes,
    r#"TABLE recipe-type AS "type", portions, length FROM #recipes"#,
    DataviewQuery {
        kind: QueryKind::Table,
        selection: Selection {
            expr: r#"recipe-type AS "type", portions, length"#.to_string()
        },
        from_clause: FromSource::Tag("recipes".to_string()),
        sort_clause: None,
        limit: None,
        where_clause: None,
    }
);

test_query_parse!(
    list_open_assignments,
    r#"LIST FROM #assignments WHERE status = "open""#,
    DataviewQuery {
        kind: QueryKind::List,
        selection: Selection::default(),
        from_clause: FromSource::Tag("assignments".to_string()),
        sort_clause: None,
        limit: None,
        where_clause: Some(WhereClause::new("status = \"open\"")),
    }
);

test_query_parse!(
    table_appointments,
    r#"TABLE file.ctime, appointment.type, appointment.time, follow-ups FROM "30 Protocols/32 Management" WHERE follow-ups SORT appointment.time"#,
    DataviewQuery {
        kind: QueryKind::Table,
        selection: Selection::new("file.ctime, appointment.type, appointment.time, follow-ups"),
        from_clause: FromSource::Folder("30 Protocols/32 Management".to_string()),
        sort_clause: Some(SortClause {
            expr: "appointment.time".to_string(),
            dir: SortDirection::Asc,
        }),
        limit: None,
        where_clause: Some(WhereClause::new("follow-ups")),
    }
);

test_query_parse!(
    list_sort,
    r#"list sort file.ctime desc"#,
    DataviewQuery {
        kind: QueryKind::List,
        selection: Selection::default(),
        from_clause: FromSource::Folder("/".to_string()),
        sort_clause: Some(SortClause {
            expr: "file.ctime".to_string(),
            dir: SortDirection::Desc
        }),
        limit: None,
        where_clause: None
    }
);

// LIST with file.mtime filter
test_query_parse!(
    list_recent,
    r#"LIST WHERE file.mtime >= date(today) - dur(1 day)"#,
    DataviewQuery {
        kind: QueryKind::List,
        selection: Selection::default(),
        from_clause: FromSource::Folder("/".to_string()),
        sort_clause: None,
        limit: None,
        where_clause: Some(WhereClause::new("file.mtime >= date(today) - dur(1 day)")),
    }
);

// LIST projects not completed and older than 1 month
test_query_parse!(
    list_old_projects,
    r#"LIST FROM #projects WHERE !completed AND file.ctime <= date(today) - dur(1 month)"#,
    DataviewQuery {
        kind: QueryKind::List,
        selection: Selection::default(),
        from_clause: FromSource::Tag("projects".to_string()),
        sort_clause: None,
        limit: None,
        where_clause: Some(WhereClause::new(
            "!completed AND file.ctime <= date(today) - dur(1 month)"
        )),
    }
);

// LIST games with price filter
test_query_parse!(
    list_expensive_games,
    r#"LIST FROM "Games" WHERE price > 10"#,
    DataviewQuery {
        kind: QueryKind::List,
        selection: Selection::default(),
        from_clause: FromSource::Folder("Games".to_string()),
        sort_clause: None,
        limit: None,
        where_clause: Some(WhereClause::new("price > 10")),
    }
);

// TASK due today or earlier
test_query_parse!(
    task_due,
    r#"TASK WHERE due <= date(today)"#,
    DataviewQuery {
        kind: QueryKind::Task,
        selection: Selection::default(),
        from_clause: FromSource::Folder("/".to_string()),
        sort_clause: None,
        limit: None,
        where_clause: Some(WhereClause::new("due <= date(today)")),
    }
);

// LIST homework not done
test_query_parse!(
    list_homework_pending,
    r#"LIST FROM #homework WHERE status != "done""#,
    DataviewQuery {
        kind: QueryKind::List,
        selection: Selection::default(),
        from_clause: FromSource::Tag("homework".to_string()),
        sort_clause: None,
        limit: None,
        where_clause: Some(WhereClause::new("status != \"done\"")),
    }
);

// LIST by file.day sorted descending
test_query_parse!(
    list_by_day,
    r#"LIST file.day
WHERE file.day
SORT file.day DESC"#,
    DataviewQuery {
        kind: QueryKind::List,
        selection: Selection::new("file.day"),
        from_clause: FromSource::Folder("/".to_string()),
        sort_clause: Some(SortClause {
            expr: "file.day".to_string(),
            dir: SortDirection::Desc,
        }),
        limit: None,
        where_clause: Some(WhereClause::new("file.day")),
    }
);

// TABLE books last modified
test_query_parse!(
    table_books,
    r#"TABLE file.mtime AS "Last Modified"
FROM "books"
SORT file.mtime DESC"#,
    DataviewQuery {
        kind: QueryKind::Table,
        selection: Selection::new(r#"file.mtime AS "Last Modified""#),
        from_clause: FromSource::Folder("books".to_string()),
        sort_clause: Some(SortClause {
            expr: "file.mtime".to_string(),
            dir: SortDirection::Desc,
        }),
        limit: None,
        where_clause: None,
    }
);

// TABLE games with multiple columns sorted by rating
test_query_parse!(
    table_games,
    r#"TABLE time-played AS "Time Played", length AS "Length", rating AS "Rating" FROM "games" SORT rating DESC"#,
    DataviewQuery {
        kind: QueryKind::Table,
        selection: Selection::new(
            r#"time-played AS "Time Played", length AS "Length", rating AS "Rating""#
        ),
        from_clause: FromSource::Folder("games".to_string()),
        sort_clause: Some(SortClause {
            expr: "rating".to_string(),
            dir: SortDirection::Desc,
        }),
        limit: None,
        where_clause: None,
    }
);

// Where Filter

mod wherefilter {
    use serde_json::json;

    use crate::dataview::query::where_filter::*;

    // --- Literal tests ---
    test_wherefilter_parse!(
        literal_number,
        "42",
        WhereFilter::new(Expr::Literal(json!(42)))
    );

    test_wherefilter_parse!(
        literal_float,
        "3.14",
        WhereFilter::new(Expr::Literal(json!(3.14)))
    );

    test_wherefilter_parse!(
        literal_string,
        "\"hello\"",
        WhereFilter::new(Expr::Literal(json!("hello")))
    );

    test_wherefilter_parse!(
        literal_bool_true,
        "true",
        WhereFilter::new(Expr::Literal(json!(true)))
    );

    test_wherefilter_parse!(
        literal_bool_false,
        "false",
        WhereFilter::new(Expr::Literal(json!(false)))
    );

    test_wherefilter_parse!(
        literal_null,
        "null",
        WhereFilter::new(Expr::Literal(json!(null)))
    );

    // --- Identifier ---
    test_wherefilter_parse!(
        identifier,
        "file.ctime",
        WhereFilter::new(Expr::Identifier("file.ctime".to_string()))
    );

    // --- Unary operations ---
    test_wherefilter_parse!(
        unary_not,
        "!completed",
        WhereFilter::new(Expr::UnaryOp {
            op: UnaryOp::Not,
            expr: Box::new(Expr::Identifier("completed".to_string()))
        })
    );

    // --- Binary operations ---
    test_wherefilter_parse!(
        simple_binary,
        "1 = 1",
        WhereFilter::new(Expr::BinaryOp {
            left: Box::new(Expr::Literal(json!(1))),
            op: BinaryOp::Eq,
            right: Box::new(Expr::Literal(json!(1))),
        })
    );

    test_wherefilter_parse!(
        basic_ident,
        "data = 1",
        WhereFilter::new(Expr::BinaryOp {
            left: Box::new(Expr::Identifier("data".to_string())),
            op: BinaryOp::Eq,
            right: Box::new(Expr::Literal(json!(1)))
        })
    );

    test_wherefilter_parse!(
        arithmetic_binary,
        "3 + 5",
        WhereFilter::new(Expr::BinaryOp {
            left: Box::new(Expr::Literal(json!(3))),
            op: BinaryOp::Add,
            right: Box::new(Expr::Literal(json!(5))),
        })
    );

    // --- Functions ---
    test_wherefilter_parse!(
        simple_function,
        "date(\"today\")",
        WhereFilter::new(Expr::FunctionCall {
            name: "date".to_string(),
            args: vec![Expr::Literal(json!("today"))],
        })
    );

    test_wherefilter_parse!(
        nested_functions,
        "f(g(1, 2), h(3))",
        WhereFilter::new(Expr::FunctionCall {
            name: "f".to_string(),
            args: vec![
                Expr::FunctionCall {
                    name: "g".to_string(),
                    args: vec![Expr::Literal(json!(1)), Expr::Literal(json!(2))],
                },
                Expr::FunctionCall {
                    name: "h".to_string(),
                    args: vec![Expr::Literal(json!(3))],
                },
            ],
        })
    );

    // --- Complex expression ---
    test_wherefilter_parse!(
        complex_example,
        "!completed AND file.ctime <= (date(\"today\") - dur(\"1month\"))",
        WhereFilter::new(Expr::BinaryOp {
            left: Box::new(Expr::UnaryOp {
                op: UnaryOp::Not,
                expr: Box::new(Expr::Identifier("completed".to_string())),
            }),
            op: BinaryOp::And,
            right: Box::new(Expr::BinaryOp {
                left: Box::new(Expr::Identifier("file.ctime".to_string())),
                op: BinaryOp::Lte,
                right: Box::new(Expr::BinaryOp {
                    left: Box::new(Expr::FunctionCall {
                        name: "date".to_string(),
                        args: vec![Expr::Literal(json!("today"))],
                    }),
                    op: BinaryOp::Sub,
                    right: Box::new(Expr::FunctionCall {
                        name: "dur".to_string(),
                        args: vec![Expr::Literal(json!("1month"))],
                    }),
                }),
            }),
        })
    );

    // Negative numbers
    test_wherefilter_parse!(
        negative_number,
        "-42",
        WhereFilter::new(Expr::Literal(json!(-42)))
    );

    // Decimal arithmetic
    test_wherefilter_parse!(
        decimal_add,
        "1.5 + 2.25",
        WhereFilter::new(Expr::BinaryOp {
            left: Box::new(Expr::Literal(json!(1.5))),
            op: BinaryOp::Add,
            right: Box::new(Expr::Literal(json!(2.25))),
        })
    );

    // Boolean logic with unary
    test_wherefilter_parse!(
        boolean_chain,
        "true OR false AND !flag",
        WhereFilter::new(Expr::BinaryOp {
            left: Box::new(Expr::Literal(json!(true))),
            op: BinaryOp::Or,
            right: Box::new(Expr::BinaryOp {
                left: Box::new(Expr::Literal(json!(false))),
                op: BinaryOp::And,
                right: Box::new(Expr::UnaryOp {
                    op: UnaryOp::Not,
                    expr: Box::new(Expr::Identifier("flag".to_string())),
                }),
            }),
        })
    );

    // Nested parentheses and function calls
    test_wherefilter_parse!(
        nested_parens,
        "(1 + 2) * (3 + 4)",
        WhereFilter::new(Expr::BinaryOp {
            left: Box::new(Expr::BinaryOp {
                left: Box::new(Expr::Literal(json!(1))),
                op: BinaryOp::Add,
                right: Box::new(Expr::Literal(json!(2))),
            }),
            op: BinaryOp::Mul,
            right: Box::new(Expr::BinaryOp {
                left: Box::new(Expr::Literal(json!(3))),
                op: BinaryOp::Add,
                right: Box::new(Expr::Literal(json!(4))),
            }),
        })
    );

    // Function with multiple args and nested function
    test_wherefilter_parse!(
        multi_arg_function,
        "sum(1, max(2, 3), 4)",
        WhereFilter::new(Expr::FunctionCall {
            name: "sum".to_string(),
            args: vec![
                Expr::Literal(json!(1)),
                Expr::FunctionCall {
                    name: "max".to_string(),
                    args: vec![Expr::Literal(json!(2)), Expr::Literal(json!(3))],
                },
                Expr::Literal(json!(4)),
            ],
        })
    );

    // Unary NOT with parentheses
    test_wherefilter_parse!(
        not_with_parens,
        "!(a = 1)",
        WhereFilter::new(Expr::UnaryOp {
            op: UnaryOp::Not,
            expr: Box::new(Expr::BinaryOp {
                left: Box::new(Expr::Identifier("a".to_string())),
                op: BinaryOp::Eq,
                right: Box::new(Expr::Literal(json!(1))),
            }),
        })
    );

    // Complex combination with AND/OR
    test_wherefilter_parse!(
        and_or_mix,
        "x = 1 AND y = 2 OR z = 3",
        WhereFilter::new(Expr::BinaryOp {
            left: Box::new(Expr::BinaryOp {
                left: Box::new(Expr::Identifier("x".to_string())),
                op: BinaryOp::Eq,
                right: Box::new(Expr::Literal(json!(1))),
            }),
            op: BinaryOp::And,
            right: Box::new(Expr::BinaryOp {
                left: Box::new(Expr::Identifier("y".to_string())),
                op: BinaryOp::Eq,
                right: Box::new(Expr::Literal(json!(2))),
            }),
        })
    );

    // Edge case: function returning boolean, compared
    test_wherefilter_parse!(
        function_boolean,
        "is_done(task) = true",
        WhereFilter::new(Expr::BinaryOp {
            left: Box::new(Expr::FunctionCall {
                name: "is_done".to_string(),
                args: vec![Expr::Identifier("task".to_string())],
            }),
            op: BinaryOp::Eq,
            right: Box::new(Expr::Literal(json!(true))),
        })
    );

    // Edge case: subtraction with negative numbers
    test_wherefilter_parse!(
        negative_subtraction,
        "-5 - -10",
        WhereFilter::new(Expr::BinaryOp {
            left: Box::new(Expr::Literal(json!(-5))),
            op: BinaryOp::Sub,
            right: Box::new(Expr::Literal(json!(-10))),
        })
    );
}

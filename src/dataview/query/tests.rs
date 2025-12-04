use crate::dataview::query::where_filter::{BinaryOp, Expr, UnaryOp};

use super::*;

/// Macro to generate DataviewQuery parse tests
macro_rules! test_query_parse {
    // Without WHERE clause validation
    ($name:ident, $input:expr, $expected:expr) => {
        #[test]
        fn $name() {
            let (_, parsed) = DataviewQuery::parse($input).unwrap();
            assert_eq!(parsed, $expected);
        }
    };

    // With WHERE clause validation - creates both tests
    ($name:ident, $input:expr, $expected:expr, $expected_where:expr) => {
        // First create the basic test without WHERE validation
        test_query_parse!($name, $input, $expected);

        // Then create an additional test with WHERE validation
        paste::paste! {
            #[test]
            fn [<$name _where_validation>]() {
                let (_, parsed) = DataviewQuery::parse($input).unwrap();

                // Verify where clause expression parses to expected filter
                if let Some(ref where_clause) = parsed.where_clause {
                    let where_expr = where_clause.expr.as_str();
                    let (_, parsed_filter) = WhereFilter::parse(where_expr)
                        .expect(&format!("WHERE clause '{}' should parse successfully", where_expr));

                    assert_eq!(
                        parsed_filter,
                        $expected_where,
                        "WHERE clause expression '{}' parsed to unexpected filter",
                        where_expr
                    );
                } else {
                    panic!("Expected WHERE clause but query has none");
                }
            }
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
    },
    WhereFilter::new(Expr::BinaryOp {
        left: Box::new(Expr::Identifier("status".to_string())),
        op: BinaryOp::Eq,
        right: Box::new(Expr::Literal(json!("open")))
    })
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
    },
    WhereFilter::new(Expr::Identifier("follow-ups".to_string()))
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

test_query_parse!(
    list_recent,
    r#"LIST WHERE file.mtime >= date(today) - dur("1 day")"#,
    DataviewQuery {
        kind: QueryKind::List,
        selection: Selection::default(),
        from_clause: FromSource::Folder("/".to_string()),
        sort_clause: None,
        limit: None,
        where_clause: Some(WhereClause::new(
            "file.mtime >= date(today) - dur(\"1 day\")"
        )),
    },
    WhereFilter::new(Expr::BinaryOp {
        left: Box::new(Expr::Identifier("file.mtime".to_string())),
        op: BinaryOp::Gte,
        right: Box::new(Expr::BinaryOp {
            left: Box::new(Expr::FunctionCall {
                name: "date".to_string(),
                args: vec![Expr::Identifier("today".to_string())]
            }),
            op: BinaryOp::Sub,
            right: Box::new(Expr::FunctionCall {
                name: "dur".to_string(),
                args: vec![Expr::Literal(json!("1 day"))]
            })
        })
    })
);

test_query_parse!(
    list_old_projects,
    r#"LIST FROM #projects WHERE !completed AND file.ctime <= date(today) - dur("1 month")"#,
    DataviewQuery {
        kind: QueryKind::List,
        selection: Selection::default(),
        from_clause: FromSource::Tag("projects".to_string()),
        sort_clause: None,
        limit: None,
        where_clause: Some(WhereClause::new(
            "!completed AND file.ctime <= date(today) - dur(\"1 month\")"
        )),
    },
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
                    args: vec![Expr::Identifier("today".to_string())],
                }),
                op: BinaryOp::Sub,
                right: Box::new(Expr::FunctionCall {
                    name: "dur".to_string(),
                    args: vec![Expr::Literal(json!("1 month"))],
                }),
            }),
        }),
    })
);

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
    },
    WhereFilter::new(Expr::BinaryOp {
        left: Box::new(Expr::Identifier("price".to_string())),
        op: BinaryOp::Gt,
        right: Box::new(Expr::Literal(json!(10))),
    })
);

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
    },
    WhereFilter::new(Expr::BinaryOp {
        left: Box::new(Expr::Identifier("due".to_string())),
        op: BinaryOp::Lte,
        right: Box::new(Expr::FunctionCall {
            name: "date".to_string(),
            args: vec![Expr::Identifier("today".to_string())],
        }),
    })
);

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
    },
    WhereFilter::new(Expr::BinaryOp {
        left: Box::new(Expr::Identifier("status".to_string())),
        op: BinaryOp::Neq,
        right: Box::new(Expr::Literal(json!("done"))),
    })
);

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
    },
    WhereFilter::new(Expr::Identifier("file.day".to_string()))
);
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

test_query_parse!(
    list_with_nested_path,
    r#"LIST FROM "Projects/Active/2024""#,
    DataviewQuery {
        kind: QueryKind::List,
        selection: Selection::default(),
        from_clause: FromSource::Folder("Projects/Active/2024".to_string()),
        sort_clause: None,
        limit: None,
        where_clause: None,
    }
);

test_query_parse!(
    task_complex_filter,
    r#"TASK WHERE !completed AND priority = "high" SORT due"#,
    DataviewQuery {
        kind: QueryKind::Task,
        selection: Selection::default(),
        from_clause: FromSource::Folder("/".to_string()),
        sort_clause: Some(SortClause {
            expr: "due".to_string(),
            dir: SortDirection::Asc,
        }),
        limit: None,
        where_clause: Some(WhereClause::new("!completed AND priority = \"high\"")),
    },
    WhereFilter::new(Expr::BinaryOp {
        left: Box::new(Expr::UnaryOp {
            op: UnaryOp::Not,
            expr: Box::new(Expr::Identifier("completed".to_string())),
        }),
        op: BinaryOp::And,
        right: Box::new(Expr::BinaryOp {
            left: Box::new(Expr::Identifier("priority".to_string())),
            op: BinaryOp::Eq,
            right: Box::new(Expr::Literal(json!("high"))),
        }),
    })
);

test_query_parse!(
    table_with_multiple_tags,
    r#"TABLE status, priority FROM #work WHERE deadline < date(today)"#,
    DataviewQuery {
        kind: QueryKind::Table,
        selection: Selection::new("status, priority"),
        from_clause: FromSource::Tag("work".to_string()),
        sort_clause: None,
        limit: None,
        where_clause: Some(WhereClause::new("deadline < date(today)")),
    },
    WhereFilter::new(Expr::BinaryOp {
        left: Box::new(Expr::Identifier("deadline".to_string())),
        op: BinaryOp::Lt,
        right: Box::new(Expr::FunctionCall {
            name: "date".to_string(),
            args: vec![Expr::Identifier("today".to_string())],
        }),
    })
);

test_query_parse!(
    list_with_complex_sort,
    r#"LIST FROM #notes SORT file.name"#,
    DataviewQuery {
        kind: QueryKind::List,
        selection: Selection::default(),
        from_clause: FromSource::Tag("notes".to_string()),
        sort_clause: Some(SortClause {
            expr: "file.name".to_string(),
            dir: SortDirection::Asc,
        }),
        limit: None,
        where_clause: None,
    }
);

mod wherefilter {
    use crate::dataview::query::where_filter::*;
    use serde_json::json;

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

    test_wherefilter_parse!(literal_zero, "0", WhereFilter::new(Expr::Literal(json!(0))));

    test_wherefilter_parse!(
        literal_very_small_float,
        "0.001",
        WhereFilter::new(Expr::Literal(json!(0.001)))
    );

    test_wherefilter_parse!(
        literal_large_number,
        "999999",
        WhereFilter::new(Expr::Literal(json!(999999)))
    );

    // --- Identifier tests ---
    test_wherefilter_parse!(
        identifier,
        "file.ctime",
        WhereFilter::new(Expr::Identifier("file.ctime".to_string()))
    );

    test_wherefilter_parse!(
        identifier_simple,
        "status",
        WhereFilter::new(Expr::Identifier("status".to_string()))
    );

    test_wherefilter_parse!(
        identifier_with_underscores,
        "is_active",
        WhereFilter::new(Expr::Identifier("is_active".to_string()))
    );

    test_wherefilter_parse!(
        identifier_deeply_nested,
        "project.metadata.created_by",
        WhereFilter::new(Expr::Identifier("project.metadata.created_by".to_string()))
    );

    test_wherefilter_parse!(
        identifier_with_numbers,
        "field123",
        WhereFilter::new(Expr::Identifier("field123".to_string()))
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

    test_wherefilter_parse!(
        unary_not_with_nested_field,
        "!project.archived",
        WhereFilter::new(Expr::UnaryOp {
            op: UnaryOp::Not,
            expr: Box::new(Expr::Identifier("project.archived".to_string()))
        })
    );

    test_wherefilter_parse!(
        double_negation,
        "!!flag",
        WhereFilter::new(Expr::UnaryOp {
            op: UnaryOp::Not,
            expr: Box::new(Expr::UnaryOp {
                op: UnaryOp::Not,
                expr: Box::new(Expr::Identifier("flag".to_string()))
            })
        })
    );

    // --- Binary operations: Equality ---
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
        inequality,
        "x != 5",
        WhereFilter::new(Expr::BinaryOp {
            left: Box::new(Expr::Identifier("x".to_string())),
            op: BinaryOp::Neq,
            right: Box::new(Expr::Literal(json!(5))),
        })
    );

    test_wherefilter_parse!(
        string_equality,
        "name = \"Alice\"",
        WhereFilter::new(Expr::BinaryOp {
            left: Box::new(Expr::Identifier("name".to_string())),
            op: BinaryOp::Eq,
            right: Box::new(Expr::Literal(json!("Alice"))),
        })
    );

    test_wherefilter_parse!(
        null_check,
        "value = null",
        WhereFilter::new(Expr::BinaryOp {
            left: Box::new(Expr::Identifier("value".to_string())),
            op: BinaryOp::Eq,
            right: Box::new(Expr::Literal(json!(null))),
        })
    );

    // --- Binary operations: Comparison ---
    test_wherefilter_parse!(
        less_than,
        "age < 18",
        WhereFilter::new(Expr::BinaryOp {
            left: Box::new(Expr::Identifier("age".to_string())),
            op: BinaryOp::Lt,
            right: Box::new(Expr::Literal(json!(18))),
        })
    );

    test_wherefilter_parse!(
        less_than_or_equal,
        "score <= 100",
        WhereFilter::new(Expr::BinaryOp {
            left: Box::new(Expr::Identifier("score".to_string())),
            op: BinaryOp::Lte,
            right: Box::new(Expr::Literal(json!(100))),
        })
    );

    test_wherefilter_parse!(
        greater_than,
        "price > 50",
        WhereFilter::new(Expr::BinaryOp {
            left: Box::new(Expr::Identifier("price".to_string())),
            op: BinaryOp::Gt,
            right: Box::new(Expr::Literal(json!(50))),
        })
    );

    test_wherefilter_parse!(
        greater_than_or_equal,
        "count >= 10",
        WhereFilter::new(Expr::BinaryOp {
            left: Box::new(Expr::Identifier("count".to_string())),
            op: BinaryOp::Gte,
            right: Box::new(Expr::Literal(json!(10))),
        })
    );

    // --- Binary operations: Arithmetic ---
    test_wherefilter_parse!(
        arithmetic_binary,
        "3 + 5",
        WhereFilter::new(Expr::BinaryOp {
            left: Box::new(Expr::Literal(json!(3))),
            op: BinaryOp::Add,
            right: Box::new(Expr::Literal(json!(5))),
        })
    );

    test_wherefilter_parse!(
        subtraction,
        "10 - 3",
        WhereFilter::new(Expr::BinaryOp {
            left: Box::new(Expr::Literal(json!(10))),
            op: BinaryOp::Sub,
            right: Box::new(Expr::Literal(json!(3))),
        })
    );

    test_wherefilter_parse!(
        multiplication,
        "4 * 7",
        WhereFilter::new(Expr::BinaryOp {
            left: Box::new(Expr::Literal(json!(4))),
            op: BinaryOp::Mul,
            right: Box::new(Expr::Literal(json!(7))),
        })
    );

    test_wherefilter_parse!(
        division,
        "20 / 4",
        WhereFilter::new(Expr::BinaryOp {
            left: Box::new(Expr::Literal(json!(20))),
            op: BinaryOp::Div,
            right: Box::new(Expr::Literal(json!(4))),
        })
    );

    test_wherefilter_parse!(
        decimal_add,
        "1.5 + 2.25",
        WhereFilter::new(Expr::BinaryOp {
            left: Box::new(Expr::Literal(json!(1.5))),
            op: BinaryOp::Add,
            right: Box::new(Expr::Literal(json!(2.25))),
        })
    );

    test_wherefilter_parse!(
        arithmetic_with_identifiers,
        "a + b",
        WhereFilter::new(Expr::BinaryOp {
            left: Box::new(Expr::Identifier("a".to_string())),
            op: BinaryOp::Add,
            right: Box::new(Expr::Identifier("b".to_string())),
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
        function_no_args,
        "now()",
        WhereFilter::new(Expr::FunctionCall {
            name: "now".to_string(),
            args: vec![],
        })
    );

    test_wherefilter_parse!(
        function_with_identifier_arg,
        "length(name)",
        WhereFilter::new(Expr::FunctionCall {
            name: "length".to_string(),
            args: vec![Expr::Identifier("name".to_string())],
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

    test_wherefilter_parse!(
        function_with_underscore,
        "get_value(key)",
        WhereFilter::new(Expr::FunctionCall {
            name: "get_value".to_string(),
            args: vec![Expr::Identifier("key".to_string())],
        })
    );

    // --- Boolean logic ---
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

    // --- Parentheses ---
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

    test_wherefilter_parse!(
        deeply_nested_parens,
        "((a + b) * (c - d)) / e",
        WhereFilter::new(Expr::BinaryOp {
            left: Box::new(Expr::BinaryOp {
                left: Box::new(Expr::BinaryOp {
                    left: Box::new(Expr::Identifier("a".to_string())),
                    op: BinaryOp::Add,
                    right: Box::new(Expr::Identifier("b".to_string())),
                }),
                op: BinaryOp::Mul,
                right: Box::new(Expr::BinaryOp {
                    left: Box::new(Expr::Identifier("c".to_string())),
                    op: BinaryOp::Sub,
                    right: Box::new(Expr::Identifier("d".to_string())),
                }),
            }),
            op: BinaryOp::Div,
            right: Box::new(Expr::Identifier("e".to_string())),
        })
    );

    // --- Complex expressions ---
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

    test_wherefilter_parse!(
        complex_with_multiple_operators,
        "(priority = \"high\" OR priority = \"urgent\") AND !completed",
        WhereFilter::new(Expr::BinaryOp {
            left: Box::new(Expr::BinaryOp {
                left: Box::new(Expr::BinaryOp {
                    left: Box::new(Expr::Identifier("priority".to_string())),
                    op: BinaryOp::Eq,
                    right: Box::new(Expr::Literal(json!("high"))),
                }),
                op: BinaryOp::Or,
                right: Box::new(Expr::BinaryOp {
                    left: Box::new(Expr::Identifier("priority".to_string())),
                    op: BinaryOp::Eq,
                    right: Box::new(Expr::Literal(json!("urgent"))),
                }),
            }),
            op: BinaryOp::And,
            right: Box::new(Expr::UnaryOp {
                op: UnaryOp::Not,
                expr: Box::new(Expr::Identifier("completed".to_string())),
            }),
        })
    );

    // --- Edge cases ---
    test_wherefilter_parse!(
        negative_number,
        "-42",
        WhereFilter::new(Expr::Literal(json!(-42)))
    );

    test_wherefilter_parse!(
        negative_subtraction,
        "-5 - -10",
        WhereFilter::new(Expr::BinaryOp {
            left: Box::new(Expr::Literal(json!(-5))),
            op: BinaryOp::Sub,
            right: Box::new(Expr::Literal(json!(-10))),
        })
    );

    test_wherefilter_parse!(
        zero_comparison,
        "value = 0",
        WhereFilter::new(Expr::BinaryOp {
            left: Box::new(Expr::Identifier("value".to_string())),
            op: BinaryOp::Eq,
            right: Box::new(Expr::Literal(json!(0))),
        })
    );

    test_wherefilter_parse!(
        float_comparison,
        "3.14 < 3.15",
        WhereFilter::new(Expr::BinaryOp {
            left: Box::new(Expr::Literal(json!(3.14))),
            op: BinaryOp::Lt,
            right: Box::new(Expr::Literal(json!(3.15))),
        })
    );

    test_wherefilter_parse!(
        mixed_arithmetic_precedence,
        "2 + 3 * 4",
        WhereFilter::new(Expr::BinaryOp {
            left: Box::new(Expr::BinaryOp {
                left: Box::new(Expr::Literal(json!(2))),
                op: BinaryOp::Add,
                right: Box::new(Expr::Literal(json!(3))),
            }),
            op: BinaryOp::Mul,
            right: Box::new(Expr::Literal(json!(4))),
        })
    );

    test_wherefilter_parse!(
        function_in_comparison,
        "length(name) > 10",
        WhereFilter::new(Expr::BinaryOp {
            left: Box::new(Expr::FunctionCall {
                name: "length".to_string(),
                args: vec![Expr::Identifier("name".to_string())],
            }),
            op: BinaryOp::Gt,
            right: Box::new(Expr::Literal(json!(10))),
        })
    );

    test_wherefilter_parse!(
        arithmetic_with_functions,
        "calculate(a) + calculate(b)",
        WhereFilter::new(Expr::BinaryOp {
            left: Box::new(Expr::FunctionCall {
                name: "calculate".to_string(),
                args: vec![Expr::Identifier("a".to_string())],
            }),
            op: BinaryOp::Add,
            right: Box::new(Expr::FunctionCall {
                name: "calculate".to_string(),
                args: vec![Expr::Identifier("b".to_string())],
            }),
        })
    );

    test_wherefilter_parse!(
        triple_negation,
        "!!!value",
        WhereFilter::new(Expr::UnaryOp {
            op: UnaryOp::Not,
            expr: Box::new(Expr::UnaryOp {
                op: UnaryOp::Not,
                expr: Box::new(Expr::UnaryOp {
                    op: UnaryOp::Not,
                    expr: Box::new(Expr::Identifier("value".to_string())),
                }),
            }),
        })
    );

    test_wherefilter_parse!(
        string_with_spaces,
        "\"hello world\"",
        WhereFilter::new(Expr::Literal(json!("hello world")))
    );

    test_wherefilter_parse!(
        case_insensitive_and,
        "x = 1 and y = 2",
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

    test_wherefilter_parse!(
        case_insensitive_or,
        "x = 1 Or y = 2",
        WhereFilter::new(Expr::BinaryOp {
            left: Box::new(Expr::BinaryOp {
                left: Box::new(Expr::Identifier("x".to_string())),
                op: BinaryOp::Eq,
                right: Box::new(Expr::Literal(json!(1))),
            }),
            op: BinaryOp::Or,
            right: Box::new(Expr::BinaryOp {
                left: Box::new(Expr::Identifier("y".to_string())),
                op: BinaryOp::Eq,
                right: Box::new(Expr::Literal(json!(2))),
            }),
        })
    );

    test_wherefilter_parse!(
        whitespace_tolerance,
        "  x   =   1  ",
        WhereFilter::new(Expr::BinaryOp {
            left: Box::new(Expr::Identifier("x".to_string())),
            op: BinaryOp::Eq,
            right: Box::new(Expr::Literal(json!(1))),
        })
    );

    test_wherefilter_parse!(
        comparison_chain,
        "1 < 2 < 3",
        WhereFilter::new(Expr::BinaryOp {
            left: Box::new(Expr::BinaryOp {
                left: Box::new(Expr::Literal(json!(1))),
                op: BinaryOp::Lt,
                right: Box::new(Expr::Literal(json!(2))),
            }),
            op: BinaryOp::Lt,
            right: Box::new(Expr::Literal(json!(3))),
        })
    );

    test_wherefilter_parse!(
        function_with_arithmetic_args,
        "max(a + b, c * d)",
        WhereFilter::new(Expr::FunctionCall {
            name: "max".to_string(),
            args: vec![
                Expr::BinaryOp {
                    left: Box::new(Expr::Identifier("a".to_string())),
                    op: BinaryOp::Add,
                    right: Box::new(Expr::Identifier("b".to_string())),
                },
                Expr::BinaryOp {
                    left: Box::new(Expr::Identifier("c".to_string())),
                    op: BinaryOp::Mul,
                    right: Box::new(Expr::Identifier("d".to_string())),
                },
            ],
        })
    );

    test_wherefilter_parse!(
        negated_comparison,
        "!(x > 5)",
        WhereFilter::new(Expr::UnaryOp {
            op: UnaryOp::Not,
            expr: Box::new(Expr::BinaryOp {
                left: Box::new(Expr::Identifier("x".to_string())),
                op: BinaryOp::Gt,
                right: Box::new(Expr::Literal(json!(5))),
            }),
        })
    );

    test_wherefilter_parse!(
        multiple_comparisons_with_and,
        "x >= 0 AND x <= 100",
        WhereFilter::new(Expr::BinaryOp {
            left: Box::new(Expr::BinaryOp {
                left: Box::new(Expr::Identifier("x".to_string())),
                op: BinaryOp::Gte,
                right: Box::new(Expr::Literal(json!(0))),
            }),
            op: BinaryOp::And,
            right: Box::new(Expr::BinaryOp {
                left: Box::new(Expr::Identifier("x".to_string())),
                op: BinaryOp::Lte,
                right: Box::new(Expr::Literal(json!(100))),
            }),
        })
    );
}

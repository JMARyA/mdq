use std::collections::VecDeque;

use mdq::Index;
use nom::bytes::take_till;
use nom::character::complete::char;
use nom::combinator::complete;
use nom::{
    branch::alt,
    bytes::{complete::tag_no_case, tag, take_till1, take_until},
    character::complete::{multispace0, multispace1},
    combinator::{map, opt},
    multi::separated_list1,
    sequence::{delimited, pair, preceded, terminated},
    IResult, Parser,
};

#[derive(Debug, PartialEq, Eq)]
pub struct DataviewQuery {
    kind: QueryKind,
    selection: Selection,
    from_clause: FromSource,
    sort_clause: Option<SortClause>,
    limit: Option<LimitClause>,
    where_clause: Option<WhereClause>,
}

#[derive(Debug, PartialEq, Eq, Default)]
pub struct Selection {
    expr: String,
}

#[derive(Debug, PartialEq, Eq, Default)]
pub struct SelectionColumns {
    pub cols: VecDeque<SelectionField>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct SelectionField {
    pub expr: String,
    pub name: Option<String>,
}

impl SelectionColumns {
    pub fn columns(&self) -> Vec<String> {
        self.cols.iter().map(|x| x.expr.clone()).collect()
    }

    pub fn ensure_first_col(mut self) -> Self {
        if self.cols.is_empty() {
            self.cols.push_front(SelectionField {
                expr: "file.name".to_string(),
                name: Some("File".to_string()),
            });
        } else if let Some(first) = self.cols.get(0) {
            if first.expr != "file.name" {
                self.cols.push_front(SelectionField {
                    expr: "file.name".to_string(),
                    name: Some("File".to_string()),
                });
            }
        }

        self
    }

    pub fn headers(&self) -> Vec<String> {
        self.cols
            .iter()
            .map(|x| {
                if let Some(h) = &x.name {
                    h.clone()
                } else {
                    x.expr.clone()
                }
            })
            .collect()
    }

    pub fn parse(input: &str) -> IResult<&str, SelectionColumns> {
        map(
            separated_list1(
                delimited(
                    multispace0,
                    nom::character::complete::char(','),
                    multispace0,
                ),
                SelectionField::parse,
            ),
            |cols| SelectionColumns { cols: cols.into() },
        )
        .parse(input)
    }
}

impl SelectionField {
    pub fn parse(input: &str) -> IResult<&str, SelectionField> {
        let (rest, (expr, alias)) = pair(
            take_till(|c| c == ','), // parse until comma
            opt(preceded(
                delimited(
                    multispace0::<&str, nom::error::Error<&str>>,
                    tag_no_case("as"),
                    multispace0::<&str, nom::error::Error<&str>>,
                ),
                alt((
                    delimited(char('"'), take_till1(|c| c == '"'), char('"')),
                    take_till1(|c: char| c == ',' || c.is_whitespace()),
                )),
            )),
        )
        .map(|(expr, alias)| (expr.trim().to_string(), alias.map(|s| s.trim().to_string())))
        .parse(input)?;

        Ok((rest, SelectionField { expr, name: alias }))
    }
}

impl Selection {
    pub fn new(expr: &str) -> Self {
        Self {
            expr: expr.to_string(),
        }
    }

    pub fn parse_expr(&self) -> Option<SelectionColumns> {
        SelectionColumns::parse(&self.expr).ok().map(|x| x.1)
    }

    pub fn parse(input: &str) -> IResult<&str, Selection> {
        // A selection ends before keywords: from, where, sort, limit
        // We detect the earliest occurrence of those or fall back to end of input.
        let keywords = ["from", "where", "sort", "limit"];

        // Find the earliest keyword occurrence
        let mut end_index = input.len();
        let lower = input.to_ascii_lowercase();

        for kw in &keywords {
            if let Some(idx) = lower.find(kw) {
                // must be a standalone keyword boundary: preceded by whitespace
                if idx == 0 || lower.as_bytes()[idx - 1].is_ascii_whitespace() {
                    end_index = end_index.min(idx);
                }
            }
        }

        let (expr_str, rest) = input.split_at(end_index);
        let expr = expr_str.trim();

        Ok((
            rest,
            Selection {
                expr: expr.to_string(),
            },
        ))
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct LimitClause {
    limit: u64,
}

impl LimitClause {
    pub fn parse(input: &str) -> IResult<&str, LimitClause> {
        map(
            preceded(
                (multispace0, tag_no_case("limit"), multispace1),
                nom::character::complete::u64,
            ),
            |n| LimitClause { limit: n },
        )
        .parse(input)
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct WhereClause {
    pub expr: String,
}

impl WhereClause {
    pub fn new(expr: &str) -> Self {
        Self {
            expr: expr.to_string(),
        }
    }

    pub fn parse(input: &str) -> IResult<&str, WhereClause> {
        // First, consume the leading "WHERE" keyword
        let (input, _) =
            preceded(nom::character::complete::multispace0, tag_no_case("where")).parse(input)?;

        let (input, _) = multispace1(input)?;

        // Now input starts with the expression
        let stop_keywords = ["sort", "limit", "from"];

        let mut end_index = input.len();
        let lower = input.to_ascii_lowercase();

        for kw in &stop_keywords {
            if let Some(idx) = lower.find(kw) {
                // must be a standalone word boundary
                if idx == 0 || lower.as_bytes()[idx - 1].is_ascii_whitespace() {
                    end_index = end_index.min(idx);
                }
            }
        }

        let (expr_str, rest) = input.split_at(end_index);
        let expr = expr_str.trim().to_string();

        if expr.is_empty() {
            return Err(nom::Err::Error(nom::error::Error::new(
                input,
                nom::error::ErrorKind::TakeTill1,
            )));
        }

        Ok((rest, WhereClause { expr }))
    }
}

impl DataviewQuery {
    pub fn run_on(&self, index: Index) -> DataviewQueryResult {
        let selection = self
            .selection
            .parse_expr()
            .unwrap_or_default()
            .ensure_first_col();
        let cols = selection.columns();
        let headers = selection.headers();
        println!("Parsed columns: {cols:?}");

        let sort = self.sort_clause.clone().map(|x| x.expr);

        let i = index.apply(
            self.limit.clone().map(|x| x.limit).unwrap_or(0) as usize,
            0,
            sort,
            self.sort_clause
                .as_ref()
                .map(|x| matches!(x.dir, SortDirection::Desc))
                .unwrap_or(false),
        );

        let d = i.create_table_data(&cols);

        match self.kind {
            QueryKind::List => DataviewQueryResult::List(d.into_iter().flatten().collect()),
            QueryKind::Table => DataviewQueryResult::Table(d, headers),
            QueryKind::Task => DataviewQueryResult::Task,
        }
    }

    pub fn parse(input: &str) -> IResult<&str, DataviewQuery> {
        map(
            (
                delimited(multispace0, QueryKind::parse, opt(multispace1)),
                opt(Selection::parse),
                opt(FromSource::parse),
                opt(WhereClause::parse),
                opt(SortClause::parse),
                opt(LimitClause::parse),
            ),
            |(kind, selection, from_clause, where_clause, sort_clause, limit)| DataviewQuery {
                kind,
                selection: selection.unwrap_or_default(),
                from_clause: from_clause.unwrap_or(FromSource::Folder("/".to_string())),
                sort_clause,
                limit,
                where_clause,
            },
        )
        .parse(input)
    }
}

#[derive(Debug)]
pub enum DataviewQueryResult {
    List(Vec<String>),
    Table(Vec<Vec<String>>, Vec<String>),

    // TODO : impl tasks
    Task,
}

impl DataviewQueryResult {
    pub fn to_markdown(&self) -> String {
        let mut ret = String::new();

        match self {
            Self::List(lst) => {
                for e in lst {
                    ret.push_str(&format!("- {}\n", e));
                }
            }
            Self::Table(rows, headers) => {
                // Headers
                let header_line: Vec<String> =
                    headers.iter().map(|h| h.replace("|", "\\|")).collect();
                ret.push_str(&format!("| {} |\n", header_line.join(" | ")));

                // Separator
                let separator: Vec<String> = headers.iter().map(|_| "---".to_string()).collect();
                ret.push_str(&format!("| {} |\n", separator.join(" | ")));

                // Rows
                for row in rows {
                    let row_line: Vec<String> = row.iter().map(|c| c.replace("|", "\\|")).collect();
                    ret.push_str(&format!("| {} |\n", row_line.join(" | ")));
                }
            }
            Self::Task => unimplemented!(),
        }

        ret
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum QueryKind {
    List,
    Table,
    Task,
}

impl QueryKind {
    fn parse(input: &str) -> IResult<&str, QueryKind> {
        alt((
            map(tag_no_case("list"), |_| QueryKind::List),
            map(tag_no_case("table"), |_| QueryKind::Table),
            map(tag_no_case("task"), |_| QueryKind::Task),
        ))
        .parse(input)
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum FromSource {
    Folder(String),
    Tag(String),
}

impl FromSource {
    /// Parse the `from` clause: from "folder" | from #tag
    fn parse(input: &str) -> IResult<&str, FromSource> {
        preceded(
            delimited(multispace0, tag_no_case("from"), multispace1),
            alt((Self::from_folder, Self::from_tag)),
        )
        .parse(input)
    }

    /// Parse: from "folder"
    fn from_folder(input: &str) -> IResult<&str, FromSource> {
        let quoted = delimited(tag("\""), take_till1(|c| c == '"'), tag("\""));

        map(quoted, |s: &str| FromSource::Folder(s.to_string())).parse(input)
    }

    /// Parse: from #tag
    fn from_tag(input: &str) -> IResult<&str, FromSource> {
        map(
            preceded(tag("#"), complete(take_till1(|c: char| c.is_whitespace()))),
            |s: &str| FromSource::Tag(s.to_string()),
        )
        .parse(input)
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct SortClause {
    expr: String,
    dir: SortDirection,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum SortDirection {
    Asc,
    Desc,
}

impl SortClause {
    fn parse(input: &str) -> IResult<&str, SortClause> {
        map(
            (
                preceded(
                    (
                        multispace0::<&str, nom::error::Error<&str>>,
                        tag_no_case("sort"),
                        multispace1::<&str, nom::error::Error<&str>>,
                    ),
                    complete(take_till1(|c| c == ' ')),
                ),
                opt(preceded(
                    multispace1,
                    alt((
                        map(tag_no_case("asc"), |_| SortDirection::Asc),
                        map(tag_no_case("desc"), |_| SortDirection::Desc),
                    )),
                )),
            ),
            |(expr, dir)| SortClause {
                expr: expr.trim().to_string(),
                dir: dir.unwrap_or(SortDirection::Asc),
            },
        )
        .parse(input)
    }
}

#[cfg(test)]
mod tests {
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
}

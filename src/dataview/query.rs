use std::collections::VecDeque;

use mdq::Index;
use nom::character::complete::char;
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
        map(
            pair(
                // Parse the main expression (anything up to " as " or comma)
                terminated(
                    take_until(" as "),
                    opt(multispace0::<&str, nom::error::Error<&str>>),
                ),
                // Optional alias
                opt(preceded(
                    (multispace0, tag_no_case("as"), multispace0),
                    alt((
                        delimited(char('"'), nom::bytes::is_not("\""), char('"')), // quoted alias
                        take_till1(|c: char| c == ',' || c.is_whitespace()),       // bare alias
                    )),
                )),
            ),
            |(expr, alias)| SelectionField {
                expr: expr.trim().to_string(),
                name: alias.map(|s| s.trim().to_string()),
            },
        )
        .parse(input)
    }
}

impl Selection {
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
    pub fn parse(input: &str) -> IResult<&str, WhereClause> {
        // WHERE <expression...>
        let (rest, expr) = preceded(
            (multispace0, tag_no_case("where"), multispace1),
            take_till1(|c| c == '\n'),
        )
        .parse(input)?;

        Ok((
            rest,
            WhereClause {
                expr: expr.trim().to_string(),
            },
        ))
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
        }

        ret
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum QueryKind {
    List,
    Table,
}

impl QueryKind {
    fn parse(input: &str) -> IResult<&str, QueryKind> {
        alt((
            map(tag_no_case("list"), |_| QueryKind::List),
            map(tag_no_case("table"), |_| QueryKind::Table),
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
            delimited(multispace0, tag_no_case("from"), multispace0),
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
            preceded(tag("#"), take_till1(|c: char| c.is_whitespace())),
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
                    take_till1(|c| c == ' '),
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
}

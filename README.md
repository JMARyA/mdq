# Markdown Query

MDQ is a command line tool to query markdown documents which have yaml frontmatter.

## Usage
Usage: `mdq [OPTIONS] <dir>`

### Options
| Option                  | Description                                                                                                                                                      |
| ----------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `-j, --json`            | Output result as JSON                                                                                                                                            |
| `-l, --limit <LIMIT>`   | Limit number of results returned [default: 0]                                                                                                                    |
| `--offset <OFFSET>`     | Offset results by a factor. Useful when used with `--limit` [default: 0]                                                                                         |
| `-f, --filter <FILTER>` | Filter to apply to the documents. See filter section below.                                                                                                      |
| `-c, --column <COLUMN>` | Specify output columns. You can rename the text displayed in the header using the `:` character like this: `VariableName:OutputName` [default: file.title:Title] |
| `-s, --sortby <KEY>`    | Sort results based on specified key                                                                                                                              |
| `-g, --groupby <KEY>`   | Group results based on specified key                                                                                                                             |
| `-r, --reverse`         | Reverse the results                                                                                                                                              |
| `--noheader`            | Dont print header in CSV mode. Useful for scripting                                                                                                              |
| `-t, --inline-tags`     | Search for inline `#tags` and include them in frontmatter                                                                                                        |
| `--tasks`               | Extract GFM task list items and expose `tasks`, `tasks_done` and `tasks_open` fields                                                                             |

## Filters
You can query your document using filters. MDQ uses [jsonfilter](https://git.hydrar.de/jmarya/jsonfilter), so you can query similiar to the `find()` function of MongoDB.

Examples:
```shell
# Select documents with version 1.0
mdq -c file.title:Title -f '{"version": "1.0"}' ./docs

# Select documents which are high priority and assigned to me
mdq -c file.title:Title -f '{"priority": "high"}' -f '{"assigned": "me"}' ./docs

# Select documents which are assigned to names starting with A or B
mdq -c file.title:Title -f '{"$or": [{"assigned": {"$regex": "^A"}}, {"assigned": {"$regex": "^B}}]}' ./docs
```

## Tasks

When using `--tasks`, MDQ extracts [GFM task list items](https://github.github.com/gfm/#task-list-items-extension-) from the document body and exposes them as queryable fields:

| Field        | Type    | Description                          |
| ------------ | ------- | ------------------------------------ |
| `tasks`      | array   | All task items as `{text, done}` objects |
| `tasks_done` | integer | Number of completed tasks            |
| `tasks_open` | integer | Number of incomplete tasks           |

Examples:
```shell
# Show task counts per document
mdq --tasks -c file.title:Title -c tasks_done:Done -c tasks_open:Open ./docs

# Find documents that still have open tasks
mdq --tasks -f '{"tasks_open": {"$gt": 0}}' -c file.title:Title -c tasks_open:Open ./docs
```

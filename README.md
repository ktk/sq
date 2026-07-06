# sq — SPARQL client

A small command line client for querying SPARQL endpoints.

The original motivation was simple: when working with a SPARQL endpoint from the command line, the same endpoint, prefixes and options tend to be repeated over and over again. `sq` moves that information into a configuration file, provides sensible defaults, and keeps the common case concise.

A second design goal is to make SPARQL easier to use from agents. Rather than generating `curl` commands and handling request details themselves, agents can invoke `sq` directly and focus on the query.

```bash
sq 'SELECT ?s WHERE { ?s a schema:Person } LIMIT 5'
```

## What it does

- Uses a configured default endpoint, with optional per-project overrides via a configuration file.
- Automatically injects standard and project-specific prefixes, so queries can use `schema:Person` rather than full IRIs.
- Produces readable terminal output with colours, sensible column widths and compact IRIs.
- Emits plain TSV when writing to a pipe, making it straightforward to combine with standard Unix tools. Use `-j` for JSON output.
- Includes a small set of built-in commands for common inspection tasks.
- Performs updates through an explicit `sq update` command, with safeguards for destructive operations.

## Usage

```bash
# read (inline, -f FILE, or stdin)
sq 'SELECT ?s ?p ?o WHERE { ?s ?p ?o } LIMIT 10'
sq -f query.rq
echo 'ASK { ?s a schema:Person }' | sq
sq < query.rq

# bare `sq` (no argument, no -f) reads the query from stdin. This is
# primarily intended for pipes and redirects. On a terminal, finish input with
# Ctrl-D (EOF) or cancel with Ctrl-C. For interactive use, passing the query as
# an argument is usually more convenient.

# formats: text (terminal default), tsv (pipe default), csv, json/-j, xml;
# ttl/nt for CONSTRUCT queries
#
# When writing to a pipe, TSV output omits column headers by default.
# Use -H to include a header row, or --no-header to suppress it explicitly.
sq -r csv 'SELECT …' > out.csv
sq -j 'SELECT …' | jq -r '.results.bindings[].s.value'
sq --full 'SELECT …'

# built-ins
sq any
sq graphs
sq classes
sq count schema:Person
sq about example:resource
sq desc example:resource
sq preds example:resource
sq endpoints

# query aliases
sq people
sq aliases

# updates
sq update 'INSERT DATA { GRAPH <urn:g> { <urn:a> <urn:p> "x" } }'
sq update --dry-run '…'
sq update --force '…'
```

Read mode rejects update operations and asks you to use `sq update` instead. By default, `sq update` also refuses potentially destructive operations such as `DROP`, `CLEAR ALL` or an unbounded `DELETE WHERE` unless `--force` is specified.

## What the built-ins actually run

The built-ins are simply shortcuts for ordinary SPARQL queries. Prefixes are injected automatically, and `-v` prints the final query before it is sent.

Some of the built-ins rely on `COUNT` and `GROUP BY`, which can be expensive on some triple stores. They are included because they work well with QLever, and the corresponding SPARQL is shown below for clarity.

```sparql
# sq any [N]           (default N = 20)
SELECT * WHERE { ?s ?p ?o } LIMIT N

# sq graphs
SELECT ?graph (COUNT(*) AS ?triples)
WHERE { GRAPH ?graph { ?s ?p ?o } }
GROUP BY ?graph ORDER BY DESC(?triples)

# sq classes
SELECT ?class (COUNT(?s) AS ?n)
WHERE { ?s a ?class }
GROUP BY ?class ORDER BY DESC(?n)

# sq count <class>
SELECT (COUNT(?s) AS ?n) WHERE { ?s a <class> }

# sq about <node>       (incoming + outgoing, as a table)
SELECT ?dir ?predicate ?object WHERE {
  { <node> ?predicate ?object BIND("out" AS ?dir) }
  UNION
  { ?object ?predicate <node> BIND("in"  AS ?dir) }
} ORDER BY ?dir ?predicate

# sq desc <node>        (incoming + outgoing, as Turtle)
CONSTRUCT { <node> ?po ?oo . ?si ?pi <node> . }
WHERE     { { <node> ?po ?oo } UNION { ?si ?pi <node> } }

# sq preds <node>       (predicate profile of a node)
SELECT ?predicate (COUNT(*) AS ?n)
WHERE { <node> ?predicate ?object }
GROUP BY ?predicate ORDER BY DESC(?n)
```

## Config

Endpoint selection follows this order:

`-e/--endpoint` (name or URL) → `SQ_ENDPOINT` → `.sq.toml` (current directory or one of its parents) → `~/.config/sq/config.toml` → built-in localhost.

```toml
# .sq.toml  (workspace)  or  ~/.config/sq/config.toml
default = "local"

[endpoints.local]
url = "http://localhost:7077/api"
token_env = "QLEVER_TOKEN"

[endpoints.prod]
url = "https://kg.example.com/api"
token_env = "PROD_TOKEN"

[prefixes]                     # added on top of the built-in standard set
clockify = "https://data.zazuko.com/clockify/"

[queries]                      # named aliases; run with `sq <name>`, list with `sq aliases`
people = "SELECT ?name WHERE { ?p a schema:Person ; schema:name ?name } ORDER BY ?name"
review = """
SELECT ?s ?o WHERE { ?s skos:closeMatch ?o } LIMIT 50"""
```

\### Saved queries

The optional `[queries]` section lets you give frequently used queries a name.

Instead of typing the full query every time,

\```bash

sq people

\```

runs the query stored under `people`.

Use `sq aliases` to list all configured aliases.

Aliases are intended for read queries only. If an alias contains an update operation, it is rejected. Use `sq update` for `INSERT`, `DELETE` and other update queries.

If an alias has the same name as a built-in command, the built-in command takes precedence.

## Flags

| Flag | Description |
|------|-------------|
| `-e`, `--endpoint <NAME|URL>` | Select an endpoint by its configured name, or specify a QLever endpoint URL directly. |
| `-r`, `--results <FMT>` | Output format: `text`, `tsv`, `csv`, `json`, `xml` for result sets, or `ttl` / `nt` for graph queries. By default, `sq` produces a readable table on a terminal and TSV when writing to a pipe. |
| `-j`, `--json` | Shortcut for `-r json`. |
| `--full` | Show full IRIs instead of shrinking them to prefixed names such as `schema:Person`. |
| `-H`, `--header` | Include a header row in machine-readable output (TSV or CSV). Useful when the output is intended for tools that expect column names. |
| `--no-header` | Omit the header row. This is the default for TSV output when writing to a pipe. |
| `--no-prefixes` | Do not automatically inject `PREFIX` declarations. |
| `--color auto|always|never` | Control coloured terminal output. Colours are disabled automatically when output is not a terminal unless `always` is selected. |
| `-v`, `--verbose` | Print the resolved endpoint and the final SPARQL query before it is sent. |
| `-f`, `--file <FILE>` | Read the query from a file instead of the command line or standard input. |
| `--dry-run` | For `sq update`, validate the update and print the final SPARQL without sending it. |
| `--force` | For `sq update`, allow operations that are normally rejected because they are potentially destructive. |

## Build

```bash
cargo build --release   # target/release/sq
```

Deps: `clap`, `ureq`, `serde_json`, `toml`, `comfy-table`, `terminal_size`, `dirs`.

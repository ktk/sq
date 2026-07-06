# sq — SPARQL client for a QLever endpoint

Fast, native, built for querying SPARQL endpoints from the shell (and by agents) without the
`curl … --data-urlencode … | jq …` ritual.

```bash
sq 'SELECT ?s WHERE { ?s a schema:Person } LIMIT 5'
```

## What it does

- **Defaults the endpoint** — never type `--service`. Zero-config against
  `http://localhost:7077/api`; override per-workspace via `.sq.toml`.
- **Auto-injects prefixes** — write `schema:Person`, not full IRIs. Standard
  prefixes are built in; project prefixes come from config.
- **Readable output** — pretty, colored, width-aware table on a terminal; IRIs
  shrunk back to `schema:Person`.
- **Pipes cleanly** — when output isn't a terminal it emits **TSV** (no colors,
  no borders) so `| cut`/`awk`/`sort` just work. `-j` gives canonical JSON for `jq`.
- **Built-ins** for the things you type constantly.
- **Updates** behind a deliberate `sq update`, with guards.

## Usage

```bash
# read (inline, -f FILE, or stdin)
sq 'SELECT ?s ?p ?o WHERE { ?s ?p ?o } LIMIT 10'
sq -f query.rq
echo 'ASK { ?s a schema:Person }' | sq
sq < query.rq
# bare `sq` (no arg, no -f) reads the query from stdin. It's for pipes/redirects,
# not a line editor: on a terminal you get backspace-only editing, and you submit
# with Ctrl-D (EOF), cancel with Ctrl-C. For interactive use prefer `sq '<query>'`.

# formats: text (tty default), tsv (pipe default), csv, json/-j, xml; ttl/nt for CONSTRUCT
sq -r csv 'SELECT …' > out.csv
sq -j 'SELECT …' | jq -r '.results.bindings[].s.value'
sq --full 'SELECT …'          # keep full IRIs instead of shrinking

# built-ins
sq any                        # peek at 20 arbitrary triples (sq any 50 for more)
sq graphs                     # named graphs + triple counts
sq classes                    # class -> instance count
sq count schema:Person        # instances of a class
sq about clockify:user/abc    # all in+out triples about a node (table)
sq desc  clockify:user/abc    # same in+out coverage, as Turtle (DESCRIBE-style)
sq preds clockify:user/abc    # predicate profile of a node (predicate + usage count)
sq endpoints                  # list configured endpoints + which resolves

# updates (needs a token: SQ_TOKEN / QLEVER_TOKEN / config token_env)
sq update 'INSERT DATA { GRAPH <urn:g> { <urn:a> <urn:p> "x" } }'
sq update --dry-run '…'       # validate + print, send nothing
sq update --force '…'         # allow guarded destructive ops
```

Read mode rejects `INSERT/DELETE/…` ("use `sq update`"); `sq update` refuses
`DROP/CLEAR ALL` and unbounded `DELETE WHERE { ?s ?p ?o }` unless `--force`.

## What the built-ins actually run

Full transparency — each built-in expands to plain SPARQL (curies/IRIs are
substituted, standard prefixes auto-injected). `-v` prints the exact final query
before sending. Note that several use `COUNT`/`GROUP BY`, which some triple
stores evaluate lazily or expensively; QLever handles them fine, but this is why
it's spelled out.

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

Resolution: `-e/--service` (name or URL) → `SQ_ENDPOINT` → `.sq.toml` (cwd +
ancestors) → `~/.config/sq/config.toml` default → built-in localhost.

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
```

## Flags

`-e/--endpoint` · `-r/--results` · `-j/--json` · `--full` · `-H/--header` ·
`--no-header` · `--no-prefixes` · `--color auto|always|never` · `-v/--verbose` ·
`-f/--file` · `--dry-run` · `--force`

> Note: value flags (`-r`, `-e`, `-f`, `--color`) go **before** the query.
> Boolean flags (`--dry-run`, `--force`, `-v`, …) work anywhere.

## Build

```bash
cargo build --release   # target/release/sq
```

Deps: `clap`, `ureq`, `serde_json`, `toml`, `comfy-table`, `terminal_size`, `dirs`.

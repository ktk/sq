//! sq — a fast SPARQL client for a QLever endpoint.
//!
//! Read:   sq 'SELECT ?s WHERE { ?s a schema:Person } LIMIT 5'
//! Write:  sq update 'INSERT DATA { ... }'
//! Built:  sq graphs | classes | about <n> | preds <n> | count <class>

mod config;
mod prefixes;

use anyhow::{bail, Result};
use clap::Parser;
use comfy_table::{Attribute, Cell, Color, ContentArrangement, Table};
use std::collections::BTreeMap;
use std::io::{IsTerminal, Read};
use std::path::PathBuf;
use std::process::ExitCode;

#[derive(Parser)]
#[command(
    name = "sq",
    version,
    about = "SPARQL client for the QLever endpoint (auto-prefixes, readable output, built-ins).",
    long_about = "sq talks to the live SPARQL endpoint. It defaults the endpoint, injects known \
                  prefixes, shrinks IRIs in human output, and ships a few built-in queries.\n\n\
                  Read:    sq 'SELECT ?s WHERE { ?s a schema:Person } LIMIT 5'\n\
                  Update:  sq update 'INSERT DATA { ... }'\n\
                  Built-in: sq any [N] | graphs | classes | about <node> | preds <node> | count <class>\n\
                  Input can be an inline query, -f FILE, or stdin."
)]
struct Cli {
    /// Endpoint name (from config) or a raw URL
    #[arg(short = 'e', long, value_name = "NAME|URL")]
    endpoint: Option<String>,

    /// Result format: text, tsv, csv, json, xml (result sets); ttl, nt (graphs)
    #[arg(short = 'r', long, value_name = "FMT")]
    results: Option<String>,

    /// Shortcut for -r json (canonical, for piping to jq)
    #[arg(short = 'j', long)]
    json: bool,

    /// Keep full IRIs in human formats instead of shrinking to prefixed names
    #[arg(long)]
    full: bool,

    /// Include a header row in tsv output
    #[arg(short = 'H', long)]
    header: bool,

    /// Never emit a header row (tsv/csv)
    #[arg(long = "no-header")]
    no_header: bool,

    /// Do not auto-inject PREFIX declarations
    #[arg(long = "no-prefixes")]
    no_prefixes: bool,

    /// When to colorize: auto, always, never
    #[arg(long, default_value = "auto", value_name = "WHEN")]
    color: String,

    /// Print the resolved endpoint and final query to stderr
    #[arg(short = 'v', long)]
    verbose: bool,

    /// Read the query/update from FILE
    #[arg(short = 'f', long, value_name = "FILE")]
    file: Option<PathBuf>,

    /// For `update`: validate and print, but send nothing
    #[arg(long = "dry-run")]
    dry_run: bool,

    /// For `update`: allow destructive unbounded operations
    #[arg(long)]
    force: bool,

    /// Query, or a subcommand (update, graphs, classes, about, preds, count, endpoints)
    #[arg(trailing_var_arg = true, allow_hyphen_values = true, value_name = "QUERY|SUBCOMMAND")]
    rest: Vec<String>,
}

fn main() -> ExitCode {
    match run() {
        Ok(code) => code,
        Err(e) => {
            eprintln!("sq: {e:#}");
            ExitCode::from(2)
        }
    }
}

fn run() -> Result<ExitCode> {
    let mut cli = Cli::parse();
    normalize(&mut cli);

    // `endpoints` needs no connection.
    if cli.rest.first().map(String::as_str) == Some("endpoints") {
        return list_endpoints(&cli);
    }

    let resolved = config::resolve(cli.endpoint.as_deref())?;

    let sub = cli.rest.first().map(String::as_str);
    match sub {
        Some("any") => read(&cli, &resolved, any_q(cli.rest.get(1).map(String::as_str))),
        Some("graphs") => read(&cli, &resolved, GRAPHS_Q.to_string()),
        Some("classes") => read(&cli, &resolved, CLASSES_Q.to_string()),
        Some("count") => read(&cli, &resolved, count_q(&arg(&cli.rest, 1)?, &resolved.prefixes)),
        Some("about") => read(&cli, &resolved, about_q(&arg(&cli.rest, 1)?, &resolved.prefixes)),
        Some("preds") => read(&cli, &resolved, preds_q(&arg(&cli.rest, 1)?, &resolved.prefixes)),
        Some("update") => update(&cli, &resolved, query_text(&cli.rest[1..], &cli.file)?),
        _ => {
            let q = query_text(&cli.rest, &cli.file)?;
            if matches!(query_kind(&q), Kind::Update) {
                bail!("this looks like an update — use `sq update '<...>'`");
            }
            read(&cli, &resolved, q)
        }
    }
}

// ── input ───────────────────────────────────────────────────────────────────

fn arg(rest: &[String], i: usize) -> Result<String> {
    rest.get(i)
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("missing argument for `{}`", rest[0]))
}

fn query_text(parts: &[String], file: &Option<PathBuf>) -> Result<String> {
    if !parts.is_empty() {
        Ok(parts.join(" "))
    } else if let Some(f) = file {
        Ok(std::fs::read_to_string(f)?)
    } else {
        let mut s = String::new();
        std::io::stdin().read_to_string(&mut s)?;
        if s.trim().is_empty() {
            bail!("no query given (pass inline, -f FILE, or via stdin)");
        }
        Ok(s)
    }
}

// ── query kind ───────────────────────────────────────────────────────────────

enum Kind {
    Select,
    Ask,
    Graph, // CONSTRUCT / DESCRIBE
    Update,
    Unknown,
}

/// Find the first significant keyword, skipping PREFIX/BASE decls and comments.
fn query_kind(q: &str) -> Kind {
    for raw in q.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let head = line.split_whitespace().next().unwrap_or("").to_uppercase();
        match head.as_str() {
            "PREFIX" | "BASE" => continue,
            "SELECT" => return Kind::Select,
            "ASK" => return Kind::Ask,
            "CONSTRUCT" | "DESCRIBE" => return Kind::Graph,
            "INSERT" | "DELETE" | "LOAD" | "CLEAR" | "DROP" | "CREATE" | "ADD" | "MOVE"
            | "COPY" | "WITH" => return Kind::Update,
            _ => return Kind::Unknown,
        }
    }
    Kind::Unknown
}

// ── read path ────────────────────────────────────────────────────────────────

fn read(cli: &Cli, r: &config::Resolved, query: String) -> Result<ExitCode> {
    let final_q = if cli.no_prefixes {
        query
    } else {
        prefixes::inject(&query, &r.prefixes)
    };
    let format = resolve_format(cli);

    if cli.verbose {
        eprintln!("sq: endpoint {}", r.url);
        eprintln!("sq: query ⤵\n{final_q}");
    }

    let kind = query_kind(&final_q);
    if matches!(kind, Kind::Graph) {
        let accept = graph_accept(&format);
        let body = http_query(&r.url, &final_q, accept)?;
        print!("{body}");
        return Ok(ExitCode::SUCCESS);
    }

    if format == "xml" {
        let body = http_query(&r.url, &final_q, "application/sparql-results+xml")?;
        print!("{body}");
        return Ok(ExitCode::SUCCESS);
    }

    let body = http_query(&r.url, &final_q, "application/sparql-results+json")?;

    if format == "json" {
        match serde_json::from_str::<serde_json::Value>(&body) {
            Ok(v) => println!("{}", serde_json::to_string_pretty(&v)?),
            Err(_) => print!("{body}"),
        }
        return Ok(ExitCode::SUCCESS);
    }

    let parsed: SparqlResponse = serde_json::from_str(&body)
        .map_err(|e| anyhow::anyhow!("could not parse results JSON: {e}\n{body}"))?;

    if let Some(b) = parsed.boolean {
        println!("{b}");
        return Ok(ExitCode::SUCCESS);
    }

    render_table(cli, &format, &parsed, &r.prefixes);
    Ok(ExitCode::SUCCESS)
}

// ── update path ──────────────────────────────────────────────────────────────

fn update(cli: &Cli, r: &config::Resolved, upd: String) -> Result<ExitCode> {
    if !matches!(query_kind(&upd), Kind::Update) {
        bail!("this does not look like an update — use `sq '<query>'` for reads");
    }
    let final_u = if cli.no_prefixes {
        upd
    } else {
        prefixes::inject(&upd, &r.prefixes)
    };

    if !cli.force {
        if let Some(what) = destructive(&final_u) {
            bail!("refusing destructive op ({what}); re-run with --force if you really mean it");
        }
    }

    if cli.verbose || cli.dry_run {
        eprintln!("sq: endpoint {}", r.url);
        eprintln!("sq: update ⤵\n{final_u}");
    }
    if cli.dry_run {
        eprintln!("sq: dry-run — nothing sent");
        return Ok(ExitCode::SUCCESS);
    }

    let token = resolve_token(r);
    if token.is_none() {
        eprintln!("sq: no token found (SQ_TOKEN / QLEVER_TOKEN / config token_env); sending anyway");
    }
    let body = http_update(&r.url, &final_u, token.as_deref())?;
    let trimmed = body.trim();
    if trimmed.is_empty() {
        eprintln!("sq: update applied");
    } else {
        eprintln!("sq: update applied");
        println!("{trimmed}");
    }
    Ok(ExitCode::SUCCESS)
}

fn resolve_token(r: &config::Resolved) -> Option<String> {
    std::env::var("SQ_TOKEN")
        .ok()
        .or_else(|| std::env::var("QLEVER_TOKEN").ok())
        .or_else(|| r.token_env.as_ref().and_then(|e| std::env::var(e).ok()))
}

/// Heuristic guard against catastrophic unbounded writes.
fn destructive(u: &str) -> Option<String> {
    let flat = u.split_whitespace().collect::<Vec<_>>().join(" ").to_uppercase();
    for pat in ["DROP ALL", "CLEAR ALL", "DROP DEFAULT", "CLEAR DEFAULT"] {
        if flat.contains(pat) {
            return Some(pat.to_string());
        }
    }
    if let Some(idx) = flat.find("DELETE WHERE") {
        let after = &flat[idx + "DELETE WHERE".len()..];
        if let (Some(o), Some(c)) = (after.find('{'), after.rfind('}')) {
            let inner = &after[o + 1..c];
            let toks: Vec<&str> = inner.split_whitespace().filter(|t| *t != ".").collect();
            if !toks.is_empty() && toks.iter().all(|t| t.starts_with('?')) {
                return Some("unbounded DELETE WHERE".to_string());
            }
        }
    }
    None
}

// ── HTTP ─────────────────────────────────────────────────────────────────────

fn http_query(url: &str, query: &str, accept: &str) -> Result<String> {
    match ureq::post(url)
        .set("Accept", accept)
        .send_form(&[("query", query)])
    {
        Ok(resp) => Ok(resp.into_string()?),
        Err(ureq::Error::Status(code, resp)) => {
            let body = resp.into_string().unwrap_or_default();
            bail!("endpoint returned HTTP {code}: {}", body.trim())
        }
        Err(e) => bail!("request failed: {e}"),
    }
}

fn http_update(url: &str, update: &str, token: Option<&str>) -> Result<String> {
    let mut req = ureq::post(url).set("Accept", "application/json");
    if let Some(t) = token {
        req = req.set("Authorization", &format!("Bearer {t}"));
    }
    match req.send_form(&[("update", update)]) {
        Ok(resp) => Ok(resp.into_string()?),
        Err(ureq::Error::Status(code, resp)) => {
            let body = resp.into_string().unwrap_or_default();
            bail!("endpoint returned HTTP {code}: {}", body.trim())
        }
        Err(e) => bail!("request failed: {e}"),
    }
}

// ── result parsing & rendering ───────────────────────────────────────────────

#[derive(serde::Deserialize)]
struct SparqlResponse {
    #[serde(default)]
    head: Head,
    #[serde(default)]
    results: ResultsBlock,
    boolean: Option<bool>,
}

#[derive(serde::Deserialize, Default)]
struct Head {
    #[serde(default)]
    vars: Vec<String>,
}

#[derive(serde::Deserialize, Default)]
struct ResultsBlock {
    #[serde(default)]
    bindings: Vec<BTreeMap<String, Term>>,
}

#[derive(serde::Deserialize)]
struct Term {
    #[serde(rename = "type")]
    ttype: String,
    value: String,
}

enum TermKind {
    Uri,
    Literal,
    Bnode,
}

fn fmt_term(t: &Term, full: bool, rev: &[(String, String)]) -> (String, TermKind) {
    match t.ttype.as_str() {
        "uri" => {
            let s = if full {
                t.value.clone()
            } else {
                prefixes::shrink(&t.value, rev)
            };
            (s, TermKind::Uri)
        }
        "bnode" => (format!("_:{}", t.value), TermKind::Bnode),
        _ => (t.value.clone(), TermKind::Literal),
    }
}

fn resolve_format(cli: &Cli) -> String {
    if cli.json {
        return "json".to_string();
    }
    if let Some(r) = &cli.results {
        return r.to_lowercase();
    }
    if std::io::stdout().is_terminal() {
        "text".to_string()
    } else {
        "tsv".to_string()
    }
}

fn graph_accept(format: &str) -> &'static str {
    match format {
        "nt" | "ntriples" | "n-triples" => "application/n-triples",
        "nq" | "nquads" | "n-quads" => "application/n-quads",
        _ => "text/turtle",
    }
}

fn color_enabled(cli: &Cli, format: &str) -> bool {
    match cli.color.as_str() {
        "never" => false,
        "always" => true,
        _ => {
            format == "text"
                && std::io::stdout().is_terminal()
                && std::env::var_os("NO_COLOR").is_none()
        }
    }
}

fn term_width() -> u16 {
    terminal_size::terminal_size()
        .map(|(w, _)| w.0)
        .filter(|w| *w > 0)
        .unwrap_or(100)
}

fn render_table(cli: &Cli, format: &str, resp: &SparqlResponse, map: &BTreeMap<String, String>) {
    let rev = prefixes::reverse(map);
    let vars = &resp.head.vars;
    let rows = &resp.results.bindings;

    match format {
        "tsv" => {
            let show_header = cli.header && !cli.no_header;
            if show_header {
                println!("{}", vars.join("\t"));
            }
            for row in rows {
                let cells: Vec<String> = vars
                    .iter()
                    .map(|v| row.get(v).map(|t| plain(t, cli.full, &rev)).unwrap_or_default())
                    .collect();
                println!("{}", cells.join("\t"));
            }
        }
        "csv" => {
            let show_header = !cli.no_header;
            if show_header {
                println!("{}", vars.iter().map(|v| csv_quote(v)).collect::<Vec<_>>().join(","));
            }
            for row in rows {
                let cells: Vec<String> = vars
                    .iter()
                    .map(|v| csv_quote(&row.get(v).map(|t| plain(t, cli.full, &rev)).unwrap_or_default()))
                    .collect();
                println!("{}", cells.join(","));
            }
        }
        _ => {
            // text
            let color = color_enabled(cli, "text");
            let mut table = Table::new();
            table.load_preset(comfy_table::presets::UTF8_FULL);
            table.set_content_arrangement(ContentArrangement::Dynamic);
            table.set_width(term_width());
            table.set_header(vars.iter().map(|v| {
                let c = Cell::new(v);
                if color {
                    c.add_attribute(Attribute::Bold)
                } else {
                    c
                }
            }));
            for row in rows {
                let cells: Vec<Cell> = vars
                    .iter()
                    .map(|v| match row.get(v) {
                        Some(t) => {
                            let (disp, kind) = fmt_term(t, cli.full, &rev);
                            let cell = Cell::new(disp);
                            if color {
                                cell.fg(match kind {
                                    TermKind::Uri => Color::Cyan,
                                    TermKind::Literal => Color::Green,
                                    TermKind::Bnode => Color::DarkGrey,
                                })
                            } else {
                                cell
                            }
                        }
                        None => Cell::new(""),
                    })
                    .collect();
                table.add_row(cells);
            }
            println!("{table}");
            if rows.is_empty() {
                eprintln!("sq: 0 rows");
            }
        }
    }
}

fn plain(t: &Term, full: bool, rev: &[(String, String)]) -> String {
    let (s, _) = fmt_term(t, full, rev);
    s.replace(['\t', '\n', '\r'], " ")
}

fn csv_quote(s: &str) -> String {
    if s.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

// ── endpoints listing ────────────────────────────────────────────────────────

fn list_endpoints(cli: &Cli) -> Result<ExitCode> {
    let m = config::merged();
    let resolved = config::resolve(cli.endpoint.as_deref())?;
    println!("resolved: {}", resolved.url);
    if m.endpoints.is_empty() {
        println!("(no named endpoints configured; built-in default {})", config::BUILTIN_ENDPOINT);
    } else {
        println!("configured:");
        for (name, e) in &m.endpoints {
            let star = if m.default.as_deref() == Some(name) { "*" } else { " " };
            println!("  {star} {name:<10} {}", e.url);
        }
    }
    Ok(ExitCode::SUCCESS)
}

// ── built-in query templates ─────────────────────────────────────────────────

/// `sq any [N]` — peek at N arbitrary triples (default 20).
fn any_q(limit: Option<&str>) -> String {
    let n = limit.and_then(|s| s.parse::<u64>().ok()).unwrap_or(20);
    format!("SELECT * WHERE {{ ?s ?p ?o }} LIMIT {n}")
}

const GRAPHS_Q: &str =
    "SELECT ?g (COUNT(*) AS ?triples) WHERE { GRAPH ?g { ?s ?p ?o } } GROUP BY ?g ORDER BY DESC(?triples)";

const CLASSES_Q: &str =
    "SELECT ?class (COUNT(?s) AS ?n) WHERE { ?s a ?class } GROUP BY ?class ORDER BY DESC(?n)";

/// Turn a user-supplied node into a SPARQL term. Full IRIs and known curies are
/// expanded to `<...>` (so curies with slashes in the local part, like
/// `clockify:user/abc`, stay valid); unknown curies pass through.
fn expand_term(x: &str, map: &BTreeMap<String, String>) -> String {
    if x.contains("://") {
        return format!("<{x}>");
    }
    if let Some((p, local)) = x.split_once(':') {
        if let Some(ns) = map.get(p) {
            return format!("<{ns}{local}>");
        }
    }
    x.to_string()
}

fn count_q(class: &str, map: &BTreeMap<String, String>) -> String {
    format!("SELECT (COUNT(?s) AS ?n) WHERE {{ ?s a {} }}", expand_term(class, map))
}

fn about_q(node: &str, map: &BTreeMap<String, String>) -> String {
    let t = expand_term(node, map);
    format!(
        "SELECT ?dir ?p ?other WHERE {{ \
         {{ {t} ?p ?other BIND(\"out\" AS ?dir) }} UNION \
         {{ ?other ?p {t} BIND(\"in\" AS ?dir) }} }} ORDER BY ?dir ?p"
    )
}

fn preds_q(node: &str, map: &BTreeMap<String, String>) -> String {
    format!(
        "SELECT DISTINCT ?p WHERE {{ {} ?p ?o }} ORDER BY ?p",
        expand_term(node, map)
    )
}

/// Pull recognized boolean flags out of the trailing args, wherever they land
/// (clap's trailing_var_arg otherwise captures flags typed *after* a subcommand,
/// e.g. `sq update --dry-run '...'`). Value flags still go before the query.
fn normalize(cli: &mut Cli) {
    let mut cleaned = Vec::new();
    for tok in std::mem::take(&mut cli.rest) {
        match tok.as_str() {
            "--dry-run" => cli.dry_run = true,
            "--force" => cli.force = true,
            "--no-prefixes" => cli.no_prefixes = true,
            "--full" => cli.full = true,
            "-v" | "--verbose" => cli.verbose = true,
            "-H" | "--header" => cli.header = true,
            "--no-header" => cli.no_header = true,
            "-j" | "--json" => cli.json = true,
            _ => cleaned.push(tok),
        }
    }
    cli.rest = cleaned;
}

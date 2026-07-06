//! Standard prefix map (from the @zazuko/prefixes set that `rvv` also draws on)
//! plus injection/shrinking helpers. Project-specific prefixes are layered on
//! top from config (`.sq.toml` / `~/.config/sq/config.toml`).

use std::collections::BTreeMap;

/// Well-known prefixes. Kept as a static table so startup stays instant (no
/// cache load). Regenerate from `@zazuko/prefixes` if it drifts.
pub static STANDARD: &[(&str, &str)] = &[
    ("rdf", "http://www.w3.org/1999/02/22-rdf-syntax-ns#"),
    ("rdfs", "http://www.w3.org/2000/01/rdf-schema#"),
    ("owl", "http://www.w3.org/2002/07/owl#"),
    ("xsd", "http://www.w3.org/2001/XMLSchema#"),
    ("schema", "http://schema.org/"),
    ("skos", "http://www.w3.org/2004/02/skos/core#"),
    ("skosxl", "http://www.w3.org/2008/05/skos-xl#"),
    ("foaf", "http://xmlns.com/foaf/0.1/"),
    ("dc", "http://purl.org/dc/elements/1.1/"),
    ("dcterms", "http://purl.org/dc/terms/"),
    ("dcat", "http://www.w3.org/ns/dcat#"),
    ("void", "http://rdfs.org/ns/void#"),
    ("prov", "http://www.w3.org/ns/prov#"),
    ("org", "http://www.w3.org/ns/org#"),
    ("time", "http://www.w3.org/2006/time#"),
    ("vcard", "http://www.w3.org/2006/vcard/ns#"),
    ("geo", "http://www.w3.org/2003/01/geo/wgs84_pos#"),
    ("sh", "http://www.w3.org/ns/shacl#"),
    ("qb", "http://purl.org/linked-data/cube#"),
    ("sioc", "http://rdfs.org/sioc/ns#"),
    ("gr", "http://purl.org/goodrelations/v1#"),
    ("hydra", "http://www.w3.org/ns/hydra/core#"),
    ("csvw", "http://www.w3.org/ns/csvw#"),
    ("sd", "http://www.w3.org/ns/sparql-service-description#"),
    ("ldp", "http://www.w3.org/ns/ldp#"),
    ("oa", "http://www.w3.org/ns/oa#"),
    ("ssn", "http://www.w3.org/ns/ssn/"),
    ("sosa", "http://www.w3.org/ns/sosa/"),
    ("dqv", "http://www.w3.org/ns/dqv#"),
    ("cc", "http://creativecommons.org/ns#"),
    ("wd", "http://www.wikidata.org/entity/"),
    ("wdt", "http://www.wikidata.org/prop/direct/"),
    ("wikibase", "http://wikiba.se/ontology#"),
    ("ical", "http://www.w3.org/2002/12/cal/icaltzd#"),
];

/// Base prefix map: the standard set as an owned map, ready to be extended
/// with config-provided prefixes.
pub fn base_map() -> BTreeMap<String, String> {
    STANDARD
        .iter()
        .map(|(p, ns)| (p.to_string(), ns.to_string()))
        .collect()
}

/// (namespace, prefix) pairs sorted by namespace length descending — used for
/// shrinking so the most specific namespace wins.
pub fn reverse(map: &BTreeMap<String, String>) -> Vec<(String, String)> {
    let mut rev: Vec<(String, String)> = map.iter().map(|(p, ns)| (ns.clone(), p.clone())).collect();
    rev.sort_by(|a, b| b.0.len().cmp(&a.0.len()));
    rev
}

/// Prepend `PREFIX` declarations for every known prefix the query actually uses
/// and hasn't already declared. QLever tolerates unused prefixes, but we keep
/// the injected set minimal so `-v` output stays readable.
pub fn inject(query: &str, map: &BTreeMap<String, String>) -> String {
    let mut header = String::new();
    for (p, ns) in map {
        if uses_prefix(query, p) && !already_declared(query, p) {
            header.push_str(&format!("PREFIX {p}: <{ns}>\n"));
        }
    }
    format!("{header}{query}")
}

fn uses_prefix(q: &str, p: &str) -> bool {
    let needle = format!("{p}:");
    let bytes = q.as_bytes();
    let mut from = 0;
    while let Some(rel) = q[from..].find(&needle) {
        let abs = from + rel;
        let ok_before = abs == 0 || {
            let c = bytes[abs - 1] as char;
            !(c.is_ascii_alphanumeric() || c == '_' || c == '-' || c == '<' || c == '/' || c == ':')
        };
        if ok_before {
            return true;
        }
        from = abs + needle.len();
    }
    false
}

fn already_declared(q: &str, p: &str) -> bool {
    q.to_lowercase()
        .contains(&format!("prefix {}:", p.to_lowercase()))
}

/// Shrink a full IRI to `prefix:local` when a namespace matches and the local
/// part is a valid prefixed-name local (no slashes etc.); otherwise return the
/// IRI unchanged.
pub fn shrink(iri: &str, rev: &[(String, String)]) -> String {
    for (ns, p) in rev {
        if let Some(local) = iri.strip_prefix(ns.as_str()) {
            if is_pn_local(local) {
                return format!("{p}:{local}");
            }
        }
    }
    iri.to_string()
}

fn is_pn_local(s: &str) -> bool {
    let mut chars = s.chars();
    match chars.next() {
        Some(f) if f.is_ascii_alphanumeric() || f == '_' => {}
        _ => return false,
    }
    s.chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-' || c == '.')
}

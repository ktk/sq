//! Standard prefix map (from the @zazuko/prefixes set that `rvv` also draws on)
//! plus injection/shrinking helpers. Project-specific prefixes are layered on
//! top from config (`.sq.toml` / `~/.config/sq/config.toml`).

use std::collections::BTreeMap;

/// Zazuko's curated prefix set (https://github.com/zazuko/rdf-vocabularies),
/// embedded as a static table so startup stays instant. Regenerate from the
/// authoritative RDF with:
///   cat ontologies/*/meta.nt > meta.nt
///   sop parse meta.nt ! query -H 'PREFIX rdfa: <http://www.w3.org/ns/rdfa#> \
///     SELECT ?prefix ?uri WHERE { ?s rdfa:prefix ?prefix ; rdfa:uri ?uri } ORDER BY ?prefix'
pub static STANDARD: &[(&str, &str)] = &[
    ("acl", "http://www.w3.org/ns/auth/acl#"),
    ("as", "https://www.w3.org/ns/activitystreams#"),
    ("b59", "https://barnard59.zazuko.com/vocab#"),
    ("bibo", "http://purl.org/ontology/bibo/"),
    ("cc", "http://creativecommons.org/ns#"),
    ("cert", "http://www.w3.org/ns/auth/cert#"),
    ("cnt", "http://www.w3.org/2011/content#"),
    ("code", "https://code.described.at/"),
    ("constant", "http://qudt.org/vocab/constant/"),
    ("crm", "http://www.cidoc-crm.org/cidoc-crm/"),
    ("csvw", "http://www.w3.org/ns/csvw#"),
    ("ctag", "http://commontag.org/ns#"),
    ("cube", "https://cube.link/"),
    ("cur", "http://qudt.org/vocab/currency/"),
    ("dash", "http://datashapes.org/dash#"),
    ("dash-sparql", "http://datashapes.org/sparql#"),
    ("dbo", "http://dbpedia.org/ontology/"),
    ("dc11", "http://purl.org/dc/elements/1.1/"),
    ("dcam", "http://purl.org/dc/dcam/"),
    ("dcat", "http://www.w3.org/ns/dcat#"),
    ("dcmitype", "http://purl.org/dc/dcmitype/"),
    ("dcterms", "http://purl.org/dc/terms/"),
    ("dig", "http://www.ics.forth.gr/isl/CRMdig/"),
    ("discipline", "http://qudt.org/vocab/discipline/"),
    ("doap", "http://usefulinc.com/ns/doap#"),
    ("dprod", "https://ekgf.github.io/dprod/"),
    ("dpv", "http://www.w3.org/ns/dpv#"),
    ("dqv", "http://www.w3.org/ns/dqv#"),
    ("dtype", "http://www.linkedmodel.org/schema/dtype#"),
    ("duv", "http://www.w3.org/ns/duv#"),
    ("earl", "http://www.w3.org/ns/earl#"),
    ("ebucore", "http://www.ebu.ch/metadata/ontologies/ebucore/ebucore#"),
    ("exif", "http://www.w3.org/2003/12/exif/ns#"),
    ("foaf", "http://xmlns.com/foaf/0.1/"),
    ("frbr", "http://purl.org/vocab/frbr/core#"),
    ("geo", "http://www.opengis.net/ont/geosparql#"),
    ("geof", "http://www.opengis.net/def/function/geosparql/"),
    ("geor", "http://www.opengis.net/def/rule/geosparql/"),
    ("gml", "http://www.opengis.net/ont/gml#"),
    ("gn", "http://www.geonames.org/ontology#"),
    ("gr", "http://purl.org/goodrelations/v1#"),
    ("grddl", "http://www.w3.org/2003/g/data-view#"),
    ("gs1", "https://gs1.org/voc/"),
    ("gtfs", "http://vocab.gtfs.org/terms#"),
    ("http", "http://www.w3.org/2011/http#"),
    ("hydra", "http://www.w3.org/ns/hydra/core#"),
    ("ical", "http://www.w3.org/2002/12/cal/icaltzd#"),
    ("la", "https://linked.art/ns/terms/"),
    ("ldp", "http://www.w3.org/ns/ldp#"),
    ("list", "http://www.w3.org/2000/10/swap/list#"),
    ("locn", "http://www.w3.org/ns/locn#"),
    ("log", "http://www.w3.org/2000/10/swap/log#"),
    ("lvont", "http://lexvo.org/ontology#"),
    ("m4i", "http://w3id.org/nfdi4ing/metadata4ing#"),
    ("ma", "http://www.w3.org/ns/ma-ont#"),
    ("mads", "http://www.loc.gov/mads/rdf/v1#"),
    ("math", "http://www.w3.org/2000/10/swap/math#"),
    ("meta", "https://cube.link/meta/"),
    ("oa", "http://www.w3.org/ns/oa#"),
    ("og", "http://ogp.me/ns#"),
    ("oidc", "http://www.w3.org/ns/solid/oidc#"),
    ("org", "http://www.w3.org/ns/org#"),
    ("owl", "http://www.w3.org/2002/07/owl#"),
    ("pim", "http://www.w3.org/ns/pim/space#"),
    ("pipeline", "https://pipeline.described.at/"),
    ("prefix", "http://qudt.org/vocab/prefix/"),
    ("prov", "http://www.w3.org/ns/prov#"),
    ("qb", "http://purl.org/linked-data/cube#"),
    ("qkdv", "http://qudt.org/vocab/dimensionvector/"),
    ("quantitykind", "http://qudt.org/vocab/quantitykind/"),
    ("qudt", "http://qudt.org/schema/qudt/"),
    ("rdau", "http://rdaregistry.info/Elements/u/"),
    ("rdf", "http://www.w3.org/1999/02/22-rdf-syntax-ns#"),
    ("rdfa", "http://www.w3.org/ns/rdfa#"),
    ("rdfs", "http://www.w3.org/2000/01/rdf-schema#"),
    ("relation", "https://cube.link/relation/"),
    ("rev", "http://purl.org/stuff/rev#"),
    ("rico", "https://www.ica.org/standards/RiC/ontology#"),
    ("rr", "http://www.w3.org/ns/r2rml#"),
    ("rss", "http://purl.org/rss/1.0/"),
    ("schema", "http://schema.org/"),
    ("sd", "http://www.w3.org/ns/sparql-service-description#"),
    ("sdmx", "http://purl.org/linked-data/sdmx#"),
    ("sem", "http://semanticweb.cs.vu.nl/2009/11/sem/"),
    ("set", "http://www.w3.org/2000/10/swap/set#"),
    ("sf", "http://www.opengis.net/ont/sf#"),
    ("sh", "http://www.w3.org/ns/shacl#"),
    ("shex", "http://www.w3.org/ns/shex#"),
    ("shsh", "http://www.w3.org/ns/shacl-shacl#"),
    ("sioc", "http://rdfs.org/sioc/ns#"),
    ("skos", "http://www.w3.org/2004/02/skos/core#"),
    ("solid", "http://www.w3.org/ns/solid/terms#"),
    ("sosa", "http://www.w3.org/ns/sosa/"),
    ("sou", "http://qudt.org/vocab/sou/"),
    ("ssn", "http://www.w3.org/ns/ssn/"),
    ("stat", "http://www.w3.org/ns/posix/stat#"),
    ("string", "http://www.w3.org/2000/10/swap/string#"),
    ("test", "http://www.w3.org/2006/03/test-description#"),
    ("time", "http://www.w3.org/2006/time#"),
    ("unit", "http://qudt.org/vocab/unit/"),
    ("vaem", "http://www.linkedmodel.org/schema/vaem#"),
    ("vann", "http://purl.org/vocab/vann/"),
    ("vcard", "http://www.w3.org/2006/vcard/ns#"),
    ("void", "http://rdfs.org/ns/void#"),
    ("vs", "http://www.w3.org/2003/06/sw-vocab-status/ns#"),
    ("vso", "http://purl.org/vso/ns#"),
    ("wdrs", "http://www.w3.org/2007/05/powder-s#"),
    ("wgs", "http://www.w3.org/2003/01/geo/wgs84_pos#"),
    ("xkos", "http://rdf-vocabulary.ddialliance.org/xkos#"),
    ("xsd", "http://www.w3.org/2001/XMLSchema#"),
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

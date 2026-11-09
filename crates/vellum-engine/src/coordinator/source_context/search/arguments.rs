//! Passive logical literal-search/1 arguments, not source identity binding,
//! current-query authority, evaluation or a public wire/occurrence codec.
use super::*;
use crate::capture::InertValueV1 as D;
use crate::coordinator::{NativeArgumentDeclarationV1, NativeHandlerVariantDeclarationV1};
use std::mem::size_of;
const FIELDS: &[(&str, &str)] = &[
    ("contract", "RegisteredContractVersion"),
    ("operation", "ClosedLiteralSearchVariant"),
    ("context", "OriginalOpaqueSearchReadContext"),
    ("ref", "LogicalSourceReference"),
    ("query", "LiteralText"),
    ("profile", "NonemptyMatchingProfile"),
    ("scopes", "OrderedScopeDescriptions"),
    ("generation", "SafeNatural"),
    ("navigation", "OperationSpecificNavigation"),
];
const REFERENCE_FIELDS: &[&str] = &["authority", "document", "incarnation", "revision"];
const SCOPE_FIELDS: &[&str] = &["lo", "hi"];
const NAVIGATION_FIELDS: &[&str] = &["start", "current", "wrap"];
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LiteralSearchValueFieldsV1 {
    pub reference: &'static [&'static str],
    pub scope: &'static [&'static str],
    pub navigation: &'static [&'static str],
}
pub fn literal_search_value_fields_v1() -> LiteralSearchValueFieldsV1 {
    LiteralSearchValueFieldsV1 {
        reference: REFERENCE_FIELDS,
        scope: SCOPE_FIELDS,
        navigation: NAVIGATION_FIELDS,
    }
}
static DECLARATIONS: [NativeArgumentDeclarationV1; 3] = [
    NativeArgumentDeclarationV1 {
        contract: "literal-search/1",
        variant: NativeHandlerVariantDeclarationV1 {
            operation: "literal-matches",
            fields: FIELDS,
        },
    },
    NativeArgumentDeclarationV1 {
        contract: "literal-search/1",
        variant: NativeHandlerVariantDeclarationV1 {
            operation: "next-match",
            fields: FIELDS,
        },
    },
    NativeArgumentDeclarationV1 {
        contract: "literal-search/1",
        variant: NativeHandlerVariantDeclarationV1 {
            operation: "previous-match",
            fields: FIELDS,
        },
    },
];
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LiteralSearchOperationV1 {
    LiteralMatches,
    NextMatch,
    PreviousMatch,
}
impl LiteralSearchOperationV1 {
    pub fn argument_declaration_v1(self) -> &'static NativeArgumentDeclarationV1 {
        &DECLARATIONS[match self {
            Self::LiteralMatches => 0,
            Self::NextMatch => 1,
            Self::PreviousMatch => 2,
        }]
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LiteralSearchArgumentLimitsV1 {
    pub max_query_bytes: usize,
    pub max_label_bytes: usize,
    pub max_scope_ranges: usize,
    /// Normalized header, owned query/reference/profile strings and scope rows;
    /// excludes borrowed roles/actor, ingress, allocator/stack/RSS and evaluation.
    pub max_owned_bytes: usize,
}
/// Logical caller description only: these strings do not mint a DocumentId,
/// authenticate an incarnation or establish correspondence to a native snapshot.
#[derive(PartialEq, Eq)]
pub struct LiteralSearchReferenceV1 {
    pub authority: String,
    pub document: String,
    pub incarnation: u64,
    pub revision: u64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LiteralSearchNavigationArgumentsV1 {
    pub start: usize,
    pub current: Option<usize>,
    pub wrap: bool,
}
#[derive(Clone, Copy)]
pub enum LiteralSearchEnvelopeValueV1<'a, 'h> {
    Object(&'a [(&'a str, LiteralSearchEnvelopeValueV1<'a, 'h>)]),
    Data(D<'a>),
    Context(SearchReadContextV1<'h>),
}
pub struct NormalizedLiteralSearchArgumentsV1<'h> {
    source: SearchReadContextV1<'h>,
    actor: &'h SettingsExecutionContextV1,
    operation: LiteralSearchOperationV1,
    reference: LiteralSearchReferenceV1,
    query: String,
    profile: String,
    scopes: Vec<ByteRange>,
    generation: u64,
    navigation: Option<LiteralSearchNavigationArgumentsV1>,
}
macro_rules! redacted { ($($t:ty),+) => { $(impl std::fmt::Debug for $t {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { f.write_str("<literal-search-arguments:redacted>") }
})+ }; }
redacted!(
    LiteralSearchReferenceV1,
    LiteralSearchEnvelopeValueV1<'_, '_>,
    NormalizedLiteralSearchArgumentsV1<'_>
);
impl<'h> NormalizedLiteralSearchArgumentsV1<'h> {
    pub fn operation(&self) -> LiteralSearchOperationV1 {
        self.operation
    }
    pub fn argument_declaration_v1(&self) -> &'static NativeArgumentDeclarationV1 {
        self.operation.argument_declaration_v1()
    }
    pub fn source(&self) -> SearchReadContextV1<'h> {
        self.source
    }
    pub fn actor(&self) -> &'h SettingsExecutionContextV1 {
        self.actor
    }
    pub fn reference(&self) -> &LiteralSearchReferenceV1 {
        &self.reference
    }
    pub fn query(&self) -> &str {
        &self.query
    }
    pub fn matching_profile(&self) -> &str {
        &self.profile
    }
    pub fn scopes(&self) -> &[ByteRange] {
        &self.scopes
    }
    pub fn generation(&self) -> u64 {
        self.generation
    }
    pub fn navigation(&self) -> Option<LiteralSearchNavigationArgumentsV1> {
        self.navigation
    }
    pub fn required_owned_bytes(
        query: usize,
        authority: usize,
        document: usize,
        profile: usize,
        scopes: usize,
    ) -> Result<usize, Error> {
        [
            query,
            authority,
            document,
            profile,
            scopes
                .checked_mul(size_of::<ByteRange>())
                .ok_or(Error::Limit)?,
        ]
        .into_iter()
        .try_fold(size_of::<Self>(), |n, v| {
            n.checked_add(v).ok_or(Error::Limit)
        })
    }
    pub fn owned_bytes(&self) -> usize {
        size_of::<Self>()
            + self.query.len()
            + self.reference.authority.len()
            + self.reference.document.len()
            + self.profile.len()
            + self.scopes.len() * size_of::<ByteRange>()
    }
}
type V<'a, 'h> = LiteralSearchEnvelopeValueV1<'a, 'h>;
fn field<'a, 'h>(rows: &[(&str, V<'a, 'h>)], key: &str) -> Result<V<'a, 'h>, Error> {
    rows.iter()
        .find(|(k, _)| *k == key)
        .map(|(_, v)| *v)
        .ok_or(Error::InvalidRequest)
}
fn data<'a>(v: V<'a, '_>) -> Result<D<'a>, Error> {
    match v {
        V::Data(v) => Ok(v),
        _ => Err(Error::InvalidRequest),
    }
}
fn text(v: D<'_>) -> Result<&str, Error> {
    match v {
        D::Text(v) => Ok(v),
        _ => Err(Error::InvalidRequest),
    }
}
fn natural(v: D<'_>) -> Result<u64, Error> {
    match v {
        D::Number(v)
            if v.is_finite() && v >= 0.0 && v.fract() == 0.0 && v <= 9_007_199_254_740_991.0 =>
        {
            Ok(v as u64)
        }
        _ => Err(Error::InvalidRequest),
    }
}
fn index(v: D<'_>) -> Result<usize, Error> {
    usize::try_from(natural(v)?).map_err(|_| Error::Limit)
}
fn object<'a>(v: D<'a>, keys: &[&str]) -> Result<&'a [(&'a str, D<'a>)], Error> {
    let D::Object(rows) = v else {
        return Err(Error::InvalidRequest);
    };
    if rows.len() != keys.len()
        || keys
            .iter()
            .any(|k| rows.iter().filter(|(name, _)| name == k).count() != 1)
    {
        return Err(Error::InvalidRequest);
    }
    Ok(rows)
}
fn member<'a>(rows: &[(&str, D<'a>)], key: &str) -> Result<D<'a>, Error> {
    rows.iter()
        .find(|(k, _)| *k == key)
        .map(|(_, v)| *v)
        .ok_or(Error::InvalidRequest)
}
fn scope(v: D<'_>) -> Result<ByteRange, Error> {
    let r = object(v, SCOPE_FIELDS)?;
    let start = index(member(r, "lo")?)?;
    let end = index(member(r, "hi")?)?;
    if start > end {
        return Err(Error::InvalidRequest);
    }
    Ok(ByteRange { start, end })
}
fn copy(v: &str) -> Result<String, Error> {
    let mut out = String::new();
    out.try_reserve_exact(v.len())
        .map_err(|_| Error::Capacity)?;
    out.push_str(v);
    Ok(out)
}
fn normalize<'a, 'h>(
    envelope: V<'a, 'h>,
    actor: &'h SettingsExecutionContextV1,
    limits: LiteralSearchArgumentLimitsV1,
) -> Result<NormalizedLiteralSearchArgumentsV1<'h>, Error> {
    let V::Object(rows) = envelope else {
        return Err(Error::InvalidRequest);
    };
    if rows.len() != 9 {
        return Err(Error::InvalidRequest);
    }
    let operation = match text(data(field(rows, "operation")?)?)? {
        "literal-matches" => LiteralSearchOperationV1::LiteralMatches,
        "next-match" => LiteralSearchOperationV1::NextMatch,
        "previous-match" => LiteralSearchOperationV1::PreviousMatch,
        _ => return Err(Error::InvalidRequest),
    };
    let d = operation.argument_declaration_v1();
    if !d.fields_match(rows.iter().map(|(k, _)| *k))
        || text(data(field(rows, "contract")?)?)? != d.contract
    {
        return Err(Error::InvalidRequest);
    }
    let V::Context(source) = field(rows, "context")? else {
        return Err(Error::InvalidRequest);
    };
    let labels = [
        &actor.authority,
        &actor.engine,
        &actor.recipient,
        &actor.purpose,
        &actor.destination,
        &actor.profile,
        &actor.continuity,
    ];
    if labels.iter().any(|v| v.is_empty()) {
        return Err(Error::InvalidRequest);
    }
    if labels.iter().any(|v| v.len() > limits.max_label_bytes) {
        return Err(Error::Limit);
    }
    let reference = object(data(field(rows, "ref")?)?, REFERENCE_FIELDS)?;
    let authority = text(member(reference, "authority")?)?;
    let document = text(member(reference, "document")?)?;
    let incarnation = natural(member(reference, "incarnation")?)?;
    let revision = natural(member(reference, "revision")?)?;
    let query = text(data(field(rows, "query")?)?)?;
    let profile = text(data(field(rows, "profile")?)?)?;
    if authority != actor.authority
        || authority.is_empty()
        || document.is_empty()
        || profile.is_empty()
    {
        return Err(Error::InvalidRequest);
    }
    let generation = natural(data(field(rows, "generation")?)?)?;
    let D::Array(scopes) = data(field(rows, "scopes")?)? else {
        return Err(Error::InvalidRequest);
    };
    let nav = data(field(rows, "navigation")?)?;
    let navigation = if operation == LiteralSearchOperationV1::LiteralMatches {
        if !matches!(nav, D::Null) {
            return Err(Error::InvalidRequest);
        }
        None
    } else {
        let n = object(nav, NAVIGATION_FIELDS)?;
        let current = match member(n, "current")? {
            D::Null => None,
            v => Some(index(v)?),
        };
        let D::Boolean(wrap) = member(n, "wrap")? else {
            return Err(Error::InvalidRequest);
        };
        Some(LiteralSearchNavigationArgumentsV1 {
            start: index(member(n, "start")?)?,
            current,
            wrap,
        })
    };
    if query.len() > limits.max_query_bytes
        || [authority, document, profile]
            .iter()
            .any(|v| v.len() > limits.max_label_bytes)
        || scopes.len() > limits.max_scope_ranges
        || NormalizedLiteralSearchArgumentsV1::required_owned_bytes(
            query.len(),
            authority.len(),
            document.len(),
            profile.len(),
            scopes.len(),
        )? > limits.max_owned_bytes
    {
        return Err(Error::Limit);
    }
    for v in scopes {
        scope(*v)?;
    }
    let mut owned_scopes = Vec::new();
    owned_scopes
        .try_reserve_exact(scopes.len())
        .map_err(|_| Error::Capacity)?;
    for v in scopes {
        owned_scopes.push(scope(*v)?);
    }
    Ok(NormalizedLiteralSearchArgumentsV1 {
        source,
        actor,
        operation,
        reference: LiteralSearchReferenceV1 {
            authority: copy(authority)?,
            document: copy(document)?,
            incarnation,
            revision,
        },
        query: copy(query)?,
        profile: copy(profile)?,
        scopes: owned_scopes,
        generation,
        navigation,
    })
}
/// Preserves all logical request fields, including unsupported matching profiles
/// and empty query/scope values. Does not convert to SearchRequest, discard
/// generation/reference/navigation, bind a source label or issue search results.
pub fn normalize_literal_search_arguments_v1<'a, 'h>(
    envelope: V<'a, 'h>,
    actor: &'h SettingsExecutionContextV1,
    limits: LiteralSearchArgumentLimitsV1,
    r: &mut Recorder,
) -> Result<NormalizedLiteralSearchArgumentsV1<'h>, Error> {
    let timing = r.begin_service_timing();
    let result = normalize(envelope, actor, limits);
    r.record(Observation {
        boundary: Boundary::LiteralSearchArgumentNormalization,
        disposition: match &result {
            Ok(_) => Disposition::Completed,
            Err(Error::Faulted) => Disposition::Unknown,
            Err(_) => Disposition::Refused,
        },
        service_time: timing.finish(),
    });
    result
}

/// Closed typed/dynamic F05 header. This is an in-process opaque-role envelope,
/// not JSON identity decoding, authority or an external document-name resolver.
pub fn normalize_literal_search_envelope_v1<'a, 'h>(
    envelope: V<'a, 'h>,
    actor: &'h SettingsExecutionContextV1,
    limits: LiteralSearchArgumentLimitsV1,
    r: &mut Recorder,
) -> Result<NormalizedLiteralSearchArgumentsV1<'h>, Error> {
    let timer = r.begin_service_timing();
    let exact = |rows: &[(&str, V<'_, '_>)], keys: &[&str]| {
        rows.len() == keys.len()
            && keys
                .iter()
                .all(|key| rows.iter().filter(|(k, _)| k == key).count() == 1)
    };
    let result = (|| {
        let V::Object(outer) = envelope else {
            return Err(Error::InvalidRequest);
        };
        if !exact(outer, &["route", "request"]) {
            return Err(Error::InvalidRequest);
        }
        let route = text(data(field(outer, "route")?)?)?;
        let V::Object(request) = field(outer, "request")? else {
            return Err(Error::InvalidRequest);
        };
        match route {
            "typed"
                if exact(request, &["family", "variant", "version", "arguments"])
                    && matches!(data(field(request,"family")?)?, D::Number(n) if n==4.0) => {}
            "dynamic"
                if exact(request, &["name", "variant", "version", "arguments"])
                    && text(data(field(request, "name")?)?)? == "search-matches" => {}
            _ => return Err(Error::InvalidRequest),
        }
        if !matches!(data(field(request,"version")?)?, D::Number(n) if n==1.0) {
            return Err(Error::InvalidRequest);
        }
        let variant = text(data(field(request, "variant")?)?)?;
        if !matches!(variant, "literal-matches" | "next-match" | "previous-match") {
            return Err(Error::InvalidRequest);
        }
        let V::Object(args) = field(request, "arguments")? else {
            return Err(Error::InvalidRequest);
        };
        if variant != text(data(field(args, "operation")?)?)? {
            return Err(Error::InvalidRequest);
        }
        normalize(V::Object(args), actor, limits)
    })();
    r.record(Observation {
        boundary: Boundary::LiteralSearchEnvelopeNormalization,
        disposition: if result.is_ok() {
            Disposition::Completed
        } else {
            Disposition::Refused
        },
        service_time: timer.finish(),
    });
    result
}

//! Borrowed passive envelope construction for tests only; no JSON role codec.
use vellum_engine::capture::InertValueV1 as D;
use vellum_engine::coordinator::*;
pub fn with_install<'h, R>(
    id: &'h InvocationId,
    config: &ResourceCatalogConfigurationV1,
    basis: f64,
    f: impl FnOnce(&ScopedRegistrationEnvelopeValueV1<'_, 'h>) -> R,
) -> R {
    let mut fields = vec![
        ("identityDomain", D::Text(&config.identity_domain)),
        ("identityKey", D::Text(&config.identity_key)),
        ("identityKnown", D::Boolean(config.identity_known)),
        ("locator", D::Text(&config.locator)),
    ];
    match &config.capabilities {
        ResourceCatalogCapabilitiesV1::StrictUtf8OneBom {
            atomic_read,
            insertion_line_break,
        } => {
            fields.extend([
                ("profile", D::Text("strict-utf8-one-bom/1")),
                ("atomicRead", D::Boolean(*atomic_read)),
                (
                    "insertionLineBreak",
                    D::Text(match insertion_line_break {
                        CatalogInsertionLineBreakV1::Lf => "LF",
                        CatalogInsertionLineBreakV1::CrLf => "CRLF",
                        CatalogInsertionLineBreakV1::Cr => "CR",
                    }),
                ),
            ]);
        }
        ResourceCatalogCapabilitiesV1::AnnotationsV1 { atomic_read } => fields.extend([
            ("profile", D::Text("vellum.annotations/1")),
            ("atomicRead", D::Boolean(*atomic_read)),
        ]),
        ResourceCatalogCapabilitiesV1::Write {
            procedure,
            logical_continuity,
            synchronization_supported,
            conditional_write,
            causal_receipts,
            non_reuse_tokens,
            exclusive_create,
        } => {
            fields.extend([
                ("procedure", D::Text(procedure)),
                ("logicalContinuity", D::Boolean(*logical_continuity)),
                ("syncSupported", D::Boolean(*synchronization_supported)),
                ("conditionalWrite", D::Boolean(*conditional_write)),
                ("causalReceipts", D::Boolean(*causal_receipts)),
                ("nonReuseTokens", D::Boolean(*non_reuse_tokens)),
                ("exclusiveCreate", D::Boolean(*exclusive_create)),
            ]);
        }
    }
    // Deliberately different property order than the schema/typed products.
    fields.reverse();
    let change = [
        ("configuration", D::Object(&fields)),
        ("action", D::Text("install")),
        ("basis", D::Number(basis)),
    ];
    let changes = [D::Object(&change)];
    let request = [("changes", D::Array(&changes))];
    use ScopedRegistrationEnvelopeValueV1 as V;
    let outer = [
        ("request", V::Data(D::Object(&request))),
        ("invocation", V::Invocation(id)),
        (
            "profile",
            V::Data(D::Text(SCOPED_RESOURCE_REGISTRATION_PROFILE_V1)),
        ),
        ("operation", V::Data(D::Text("activate-context"))),
        (
            "contract",
            V::Data(D::Text("scoped-registration-invocation/1")),
        ),
    ];
    f(&V::Object(&outer))
}

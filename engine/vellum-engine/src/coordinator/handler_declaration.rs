//! Inert handler-owned semantic declarations, not value codecs or enablement.
mod context;
mod ordinary;
pub(in crate::coordinator) use ordinary::{
    ORDINARY_EDIT_DESCRIPTOR_CONTRACT_V1, ordinary_schema_matches_v1, ordinary_workspace_bytes_v1,
    with_ordinary_descriptor_v1,
};
#[cfg(test)]
mod tests;
pub use context::capture_native_context_declaration_v1;
pub(in crate::coordinator) use context::{context_workspace_bytes_v1, with_context_descriptor_v1};

/// Static inner argument shape, not an executable reference, permission or full
/// discovery schema. Nested values retain their owning native validators.
#[derive(Debug, PartialEq, Eq)]
pub struct NativeArgumentDeclarationV1 {
    pub contract: &'static str,
    pub variant: NativeHandlerVariantDeclarationV1,
}
impl NativeArgumentDeclarationV1 {
    /// Only owning decoders call this with borrowed slice iterators. The length
    /// preflight bounds scans to the fixed declaration; no allocation or user
    /// callback/role decoding occurs. Literal and nested checks remain separate.
    pub(in crate::coordinator) fn fields_match<'a>(
        &self,
        names: impl ExactSizeIterator<Item = &'a str> + Clone,
    ) -> bool {
        names.len() == self.variant.fields.len()
            && self
                .variant
                .fields
                .iter()
                .all(|(field, _)| names.clone().filter(|name| name == field).count() == 1)
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NativeHandlerVariantDeclarationV1 {
    pub operation: &'static str,
    pub fields: &'static [(&'static str, &'static str)],
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NativeHandlerDeclarationV1 {
    pub family: &'static str,
    pub contract: &'static str,
    pub variants: &'static [NativeHandlerVariantDeclarationV1],
    pub outcomes: &'static [(&'static str, &'static str)],
    pub admission: &'static [&'static str],
    pub current_predicates: &'static str,
    pub limits: &'static [&'static str],
    pub limit_values: &'static str,
    pub version_capacity: Option<&'static str>,
}

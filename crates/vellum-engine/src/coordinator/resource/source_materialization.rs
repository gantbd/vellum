//! One native allocator and binding installer for raw imports and read activation.
use super::*;
use crate::coordinator::{birth::PreparedDocumentBirth, root_identity::NativeRootIdentity};
pub(super) struct PreparedSourceMaterialization {
    before: NativeRootIdentity,
    read_registration: Arc<Registration>,
    pub(super) version: DocumentVersion,
    pub(super) key: BindingKey,
    birth: Option<(PreparedDocumentBirth, Binding)>,
}
impl PreparedSourceMaterialization {
    pub(super) fn creates(&self) -> bool {
        self.birth.is_some()
    }
    pub(super) fn validate(
        &self,
        o: &SourceCoordinator,
        r: &mut Recorder,
    ) -> Result<(), ResourceError> {
        o.ensure_operational().map_err(owner_error)?;
        if o.native_root_identity()
            .map_err(|_| ResourceError::Faulted)?
            != self.before
        {
            return Err(ResourceError::Stale);
        }
        o.provider_current(&self.read_registration)?;
        if o.resources
            .annotation_bindings
            .iter()
            .any(|b| b.key.identity == self.key.identity)
        {
            return Err(ResourceError::Conflict);
        }
        if let Some((birth, _)) = &self.birth {
            // Pending writers are not all represented by the semantic root stamp.
            if o.source_destination_reserved(&self.key.identity) {
                return Err(ResourceError::Blocked);
            }
            if o.resource_contested(&self.key.identity) {
                return Err(ResourceError::Contested);
            }
            birth.validate(o, r).map_err(owner_error)?;
        }
        Ok(())
    }
    pub(super) fn install_fenced(self, o: &mut SourceCoordinator) {
        debug_assert!(o.faulted);
        if let Some((birth, binding)) = self.birth {
            birth.install_fenced(o);
            o.resources.bindings.push(binding);
            #[cfg(test)]
            assert_ne!(
                o.publication_fault_after,
                Some(3),
                "injected source binding interruption"
            );
        }
    }
}
impl SourceCoordinator {
    pub(super) fn prepare_source_materialization(
        &mut self,
        read: ResourceRead,
        policy: GeneratedNewlineV1,
        r: &mut Recorder,
    ) -> Result<PreparedSourceMaterialization, ResourceError> {
        self.ensure_operational().map_err(owner_error)?;
        self.provider_current(&read.registration)?;
        let read_registration = read.registration.clone();
        if self
            .resources
            .annotation_bindings
            .iter()
            .any(|b| b.key.identity == read.identity)
        {
            return Err(ResourceError::Conflict);
        }
        let (version, key, birth) = if let Some(binding) = self
            .resources
            .bindings
            .iter()
            .find(|b| b.identity == read.identity)
        {
            let document = self.document(&binding.document).map_err(owner_error)?;
            (
                DocumentVersion {
                    document: binding.document.clone(),
                    revision: document.revision,
                },
                BindingKey {
                    identity: binding.identity.clone(),
                    registration: binding.registration.clone(),
                    generation: binding.generation,
                },
                None,
            )
        } else {
            if self.source_destination_reserved(&read.identity) {
                return Err(ResourceError::Blocked);
            }
            if self.resource_contested(&read.identity) {
                return Err(ResourceError::Contested);
            }
            self.resources
                .bindings
                .try_reserve(1)
                .map_err(|_| ResourceError::Capacity)?;
            let bytes =
                root_accounting::source_load_bytes(&read).map_err(|_| ResourceError::Limit)?;
            let doc = self
                .decode_imported_document(&read.bytes.0, policy, r)
                .map_err(owner_error)?;
            let birth = self
                .prepare_document_birth(doc, bytes, r)
                .map_err(owner_error)?;
            let version = birth.version.clone();
            let key = BindingKey {
                identity: read.identity.clone(),
                registration: read.registration.clone(),
                generation: 0,
            };
            let binding = Binding {
                document: version.document.clone(),
                identity: read.identity,
                registration: read.registration,
                generation: 0,
                baseline: Some(Baseline {
                    epoch: read.epoch,
                    bytes: read.bytes,
                    validator: Some(read.validator),
                    anchor: None,
                }),
            };
            (version, key, Some((birth, binding)))
        };
        let before = self
            .native_root_identity()
            .map_err(|_| ResourceError::Faulted)?;
        Ok(PreparedSourceMaterialization {
            before,
            read_registration,
            version,
            key,
            birth,
        })
    }
}

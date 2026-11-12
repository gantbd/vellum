//! Private, bounded, plaintext durable request journal.

use rustix::fs::{AtFlags, Dir, Mode, OFlags, mkdirat, openat, statat, unlinkat};
use std::{
    collections::BTreeMap,
    fs::{File, OpenOptions},
    io::{Read, Write},
    num::NonZeroU64,
    os::unix::fs::{FileTypeExt, MetadataExt, OpenOptionsExt},
    path::{Path, PathBuf},
};
use vellum_host::request_journal::{
    DurableOperationCompletionV1, DurableOperationIntentV1, DurableOperationRecordV1,
    DurableOperationRecoveryV1, DurableOperationRetirementRecordV1,
    DurableOperationRetirementRecoveryV1, DurableOperationRetirementResultV1,
    DurableOperationRetirementV1, RequestJournalCompletionV1, RequestJournalIntentV1,
    RequestJournalLimitsV1, RequestJournalPortV1, RequestJournalRecordV1, RequestJournalRecoveryV1,
    RequestJournalRefusalV1, RequestJournalScopeV1,
};
use vellum_protocol::DurableOperationRefV1;

const DIRECTORY_NAME: &str = ".vellum-request-journal-v1";
const INTENT_MAGIC: &[u8; 8] = b"VELLJIN1";
const REPLY_MAGIC: &[u8; 8] = b"VELLJRP1";
const HEADER_BYTES: usize = 8 + 8 * 5 + 16;
const LEGACY_OPERATION_DIRECTORY_NAME: &str = ".vellum-durable-operation-v1";
const OPERATION_DIRECTORY_NAME: &str = ".vellum-durable-operation-v2";
type DurableOperationKeyV2 = ([u8; 16], [u8; 16]);
type DurableOperationPartsV2 = (Option<String>, Option<String>, Option<String>);
const OPERATION_INTENT_MAGIC: &[u8; 8] = b"VELLOI02";
const OPERATION_REPLY_MAGIC: &[u8; 8] = b"VELLOR02";
const OPERATION_RETIRED_MAGIC: &[u8; 8] = b"VELLOT02";
const OPERATION_HEADER_BYTES: usize = 8 + 8 * 2 + 16 + 16 + 8;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct DirectoryIdentity {
    device: u64,
    inode: u64,
}

/// An opened journal namespace. All entry access is relative to this exact
/// descriptor; `namespace_path` is used only to prove that the workspace name
/// still denotes the descriptor before an operation is reported successful.
struct AnchoredDirectory {
    descriptor: File,
    namespace_path: PathBuf,
    identity: DirectoryIdentity,
}

struct DecodedRecord {
    scope: RequestJournalScopeV1,
    client: NonZeroU64,
    request: NonZeroU64,
    authenticated_subject: [u8; 16],
    frozen_request: Vec<u8>,
    frozen_reply: Vec<u8>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct LocalRequestJournalUsageV1 {
    pub recover_attempts: u64,
    pub recovered: u64,
    pub begin_attempts: u64,
    pub begun: u64,
    pub complete_attempts: u64,
    pub completed: u64,
    pub retirement_attempts: u64,
    pub retired: u64,
    pub retirement_absent_or_foreign: u64,
    pub retirement_conflicts: u64,
    pub retirement_pending: u64,
    pub refused: u64,
    pub outcome_unknown: u64,
    pub counter_saturated: bool,
}

#[derive(Default)]
pub struct LocalRequestJournalV1 {
    usage: LocalRequestJournalUsageV1,
}

impl LocalRequestJournalV1 {
    pub fn usage(&self) -> LocalRequestJournalUsageV1 {
        self.usage
    }
}

impl RequestJournalPortV1 for LocalRequestJournalV1 {
    fn recover(
        &mut self,
        directory: &Path,
        scope: RequestJournalScopeV1,
        limits: RequestJournalLimitsV1,
    ) -> RequestJournalRecoveryV1 {
        bump(
            &mut self.usage.recover_attempts,
            &mut self.usage.counter_saturated,
        );
        match recover(directory, scope, limits) {
            Ok(records) if records.is_empty() => RequestJournalRecoveryV1::Empty,
            Ok(records) => {
                bump(&mut self.usage.recovered, &mut self.usage.counter_saturated);
                RequestJournalRecoveryV1::Recovered(records)
            }
            Err(reason) => {
                bump(&mut self.usage.refused, &mut self.usage.counter_saturated);
                RequestJournalRecoveryV1::Refused(reason)
            }
        }
    }

    fn begin(
        &mut self,
        directory: &Path,
        intent: RequestJournalIntentV1<'_>,
        limits: RequestJournalLimitsV1,
    ) -> Result<(), RequestJournalRefusalV1> {
        bump(
            &mut self.usage.begin_attempts,
            &mut self.usage.counter_saturated,
        );
        match begin(directory, intent, limits) {
            Ok(()) => {
                bump(&mut self.usage.begun, &mut self.usage.counter_saturated);
                Ok(())
            }
            Err(reason) => {
                observe_refusal(&mut self.usage, reason);
                Err(reason)
            }
        }
    }

    fn complete(
        &mut self,
        directory: &Path,
        completion: RequestJournalCompletionV1<'_>,
        limits: RequestJournalLimitsV1,
    ) -> Result<(), RequestJournalRefusalV1> {
        bump(
            &mut self.usage.complete_attempts,
            &mut self.usage.counter_saturated,
        );
        match complete(directory, completion, limits) {
            Ok(()) => {
                bump(&mut self.usage.completed, &mut self.usage.counter_saturated);
                Ok(())
            }
            Err(reason) => {
                observe_refusal(&mut self.usage, reason);
                Err(reason)
            }
        }
    }

    fn recover_operations(
        &mut self,
        directory: &Path,
        scope: RequestJournalScopeV1,
        limits: RequestJournalLimitsV1,
    ) -> DurableOperationRecoveryV1 {
        bump(
            &mut self.usage.recover_attempts,
            &mut self.usage.counter_saturated,
        );
        match recover_operations(directory, scope, limits) {
            Ok(records) if records.is_empty() => DurableOperationRecoveryV1::Empty,
            Ok(records) => {
                bump(&mut self.usage.recovered, &mut self.usage.counter_saturated);
                DurableOperationRecoveryV1::Recovered(records)
            }
            Err(reason) => {
                observe_refusal(&mut self.usage, reason);
                DurableOperationRecoveryV1::Refused(reason)
            }
        }
    }

    fn begin_operation(
        &mut self,
        directory: &Path,
        intent: DurableOperationIntentV1<'_>,
        limits: RequestJournalLimitsV1,
    ) -> Result<(), RequestJournalRefusalV1> {
        bump(
            &mut self.usage.begin_attempts,
            &mut self.usage.counter_saturated,
        );
        let result = begin_operation(directory, intent, limits);
        match result {
            Ok(()) => bump(&mut self.usage.begun, &mut self.usage.counter_saturated),
            Err(reason) => observe_refusal(&mut self.usage, reason),
        }
        result
    }

    fn complete_operation(
        &mut self,
        directory: &Path,
        completion: DurableOperationCompletionV1<'_>,
        limits: RequestJournalLimitsV1,
    ) -> Result<(), RequestJournalRefusalV1> {
        bump(
            &mut self.usage.complete_attempts,
            &mut self.usage.counter_saturated,
        );
        let result = complete_operation(directory, completion, limits);
        match result {
            Ok(()) => bump(&mut self.usage.completed, &mut self.usage.counter_saturated),
            Err(reason) => observe_refusal(&mut self.usage, reason),
        }
        result
    }

    fn recover_operation_retirements(
        &mut self,
        directory: &Path,
        scope: RequestJournalScopeV1,
        limits: RequestJournalLimitsV1,
    ) -> DurableOperationRetirementRecoveryV1 {
        match recover_operation_retirements(directory, scope, limits) {
            Ok(records) if records.is_empty() => DurableOperationRetirementRecoveryV1::Empty,
            Ok(records) => DurableOperationRetirementRecoveryV1::Recovered(records),
            Err(reason) => DurableOperationRetirementRecoveryV1::Refused(reason),
        }
    }

    fn retire_operation(
        &mut self,
        directory: &Path,
        retirement: DurableOperationRetirementV1<'_>,
        limits: RequestJournalLimitsV1,
    ) -> Result<DurableOperationRetirementResultV1, RequestJournalRefusalV1> {
        bump(
            &mut self.usage.retirement_attempts,
            &mut self.usage.counter_saturated,
        );
        let result = retire_operation(directory, retirement, limits);
        match result {
            Ok(DurableOperationRetirementResultV1::Retired) => {
                bump(&mut self.usage.retired, &mut self.usage.counter_saturated)
            }
            Ok(DurableOperationRetirementResultV1::AbsentOrForeign) => bump(
                &mut self.usage.retirement_absent_or_foreign,
                &mut self.usage.counter_saturated,
            ),
            Ok(DurableOperationRetirementResultV1::RequestConflict) => bump(
                &mut self.usage.retirement_conflicts,
                &mut self.usage.counter_saturated,
            ),
            Ok(DurableOperationRetirementResultV1::Pending) => bump(
                &mut self.usage.retirement_pending,
                &mut self.usage.counter_saturated,
            ),
            Err(reason) => observe_refusal(&mut self.usage, reason),
        }
        result
    }
}

fn recover_operations(
    directory: &Path,
    scope: RequestJournalScopeV1,
    limits: RequestJournalLimitsV1,
) -> Result<Vec<DurableOperationRecordV1>, RequestJournalRefusalV1> {
    validate_limits(limits)?;
    refuse_legacy_operation_directory(directory)?;
    let journal = match open_existing_named_directory(directory, OPERATION_DIRECTORY_NAME) {
        Ok(journal) => journal,
        Err(RequestJournalRefusalV1::Missing) => return Ok(Vec::new()),
        Err(reason) => return Err(reason),
    };
    let mut parts: BTreeMap<DurableOperationKeyV2, DurableOperationPartsV2> = BTreeMap::new();
    for name in journal.entry_names()? {
        let (subject, operation, part) =
            parse_operation_name(&name).ok_or(RequestJournalRefusalV1::Malformed)?;
        let entry = parts.entry((subject, operation)).or_default();
        let slot = match part {
            OperationPartV2::Intent => &mut entry.0,
            OperationPartV2::Reply => &mut entry.1,
            OperationPartV2::Retired => &mut entry.2,
        };
        if slot.replace(name).is_some() {
            return Err(RequestJournalRefusalV1::Malformed);
        }
    }
    let active = parts.values().filter(|parts| parts.2.is_none()).count();
    let retired = parts.values().filter(|parts| parts.2.is_some()).count();
    if active > limits.max_entries || retired > limits.max_retired_operations {
        return Err(RequestJournalRefusalV1::Limit);
    }
    let mut records = Vec::new();
    records
        .try_reserve_exact(parts.len())
        .map_err(|_| RequestJournalRefusalV1::Limit)?;
    for ((name_subject, name_operation), (intent_path, reply_path, retired_path)) in parts {
        if let Some(retired_name) = retired_path {
            let retired = decode_operation_record(
                &journal.read_private_file(&retired_name, limits.max_encoded_bytes)?,
                OPERATION_RETIRED_MAGIC,
                limits,
            )?;
            if retired.authenticated_subject != name_subject
                || retired.operation.to_bytes() != name_operation
                || retired.scope.workspace() != scope.workspace()
                || retired.scope.generation() > scope.generation()
                || retired.frozen_reply.is_some()
            {
                return Err(RequestJournalRefusalV1::Conflict);
            }
            finish_committed_retirement(&journal, &retired, intent_path, reply_path, limits)?;
            continue;
        }
        let intent_name = intent_path.ok_or(RequestJournalRefusalV1::Malformed)?;
        let intent = decode_operation_record(
            &journal.read_private_file(&intent_name, limits.max_encoded_bytes)?,
            OPERATION_INTENT_MAGIC,
            limits,
        )?;
        if intent.authenticated_subject != name_subject
            || intent.operation.to_bytes() != name_operation
            || intent.scope.workspace() != scope.workspace()
            || intent.scope.generation() > scope.generation()
            || intent.frozen_reply.is_some()
        {
            return Err(RequestJournalRefusalV1::Conflict);
        }
        let frozen_reply = match reply_path {
            None => None,
            Some(reply_name) => {
                let reply = decode_operation_record(
                    &journal.read_private_file(&reply_name, limits.max_encoded_bytes)?,
                    OPERATION_REPLY_MAGIC,
                    limits,
                )?;
                if reply.scope != intent.scope
                    || reply.operation != intent.operation
                    || reply.authenticated_subject != intent.authenticated_subject
                    || reply.frozen_request != intent.frozen_request
                    || reply.frozen_reply.as_ref().is_none_or(Vec::is_empty)
                {
                    return Err(RequestJournalRefusalV1::Conflict);
                }
                reply.frozen_reply
            }
        };
        records.push(DurableOperationRecordV1 {
            frozen_reply,
            ..intent
        });
    }
    journal.revalidate()?;
    refuse_legacy_operation_directory(directory)?;
    Ok(records)
}

fn begin_operation(
    directory: &Path,
    intent: DurableOperationIntentV1<'_>,
    limits: RequestJournalLimitsV1,
) -> Result<(), RequestJournalRefusalV1> {
    validate_operation_payloads(intent.frozen_request, &[], limits)?;
    refuse_legacy_operation_directory(directory)?;
    let journal = open_or_create_named_directory(directory, OPERATION_DIRECTORY_NAME)?;
    let retired_name = operation_retired_file_name(intent.authenticated_subject, intent.operation);
    if journal.entry_exists(&retired_name)? {
        // A tombstoned operation identity is absorbing. Even an exact delayed
        // begin cannot become a new effect; changed input is also a conflict.
        return Err(RequestJournalRefusalV1::Conflict);
    }
    let name = operation_file_name(intent.authenticated_subject, intent.operation, true);
    let encoded = encode_operation_record(OPERATION_INTENT_MAGIC, intent, &[], limits)?;
    let mut existing = 0usize;
    for entry_name in journal.entry_names()? {
        let (_, _, part) =
            parse_operation_name(&entry_name).ok_or(RequestJournalRefusalV1::Malformed)?;
        if part == OperationPartV2::Intent {
            existing = existing
                .checked_add(1)
                .ok_or(RequestJournalRefusalV1::Limit)?;
        }
    }
    if !journal.entry_exists(&name)? && existing >= limits.max_entries {
        return Err(RequestJournalRefusalV1::Full);
    }
    match journal.create_private_file(&name, &encoded) {
        Ok(()) => journal.synchronize_and_revalidate(),
        Err(RequestJournalRefusalV1::Conflict) => {
            if journal.read_private_file(&name, limits.max_encoded_bytes)? == encoded {
                journal.synchronize_and_revalidate()
            } else {
                Err(RequestJournalRefusalV1::Conflict)
            }
        }
        Err(reason) => Err(reason),
    }?;
    refuse_legacy_operation_directory(directory)
}

fn complete_operation(
    directory: &Path,
    completion: DurableOperationCompletionV1<'_>,
    limits: RequestJournalLimitsV1,
) -> Result<(), RequestJournalRefusalV1> {
    validate_operation_payloads(
        completion.intent.frozen_request,
        completion.frozen_reply,
        limits,
    )?;
    refuse_legacy_operation_directory(directory)?;
    let journal = open_existing_named_directory(directory, OPERATION_DIRECTORY_NAME)?;
    if journal.entry_exists(&operation_retired_file_name(
        completion.intent.authenticated_subject,
        completion.intent.operation,
    ))? {
        return Err(RequestJournalRefusalV1::Conflict);
    }
    let intent_name = operation_file_name(
        completion.intent.authenticated_subject,
        completion.intent.operation,
        true,
    );
    let expected = encode_operation_record(OPERATION_INTENT_MAGIC, completion.intent, &[], limits)?;
    if journal.read_private_file(&intent_name, limits.max_encoded_bytes)? != expected {
        return Err(RequestJournalRefusalV1::Conflict);
    }
    let name = operation_file_name(
        completion.intent.authenticated_subject,
        completion.intent.operation,
        false,
    );
    let encoded = encode_operation_record(
        OPERATION_REPLY_MAGIC,
        completion.intent,
        completion.frozen_reply,
        limits,
    )?;
    match journal.create_private_file(&name, &encoded) {
        Ok(()) => journal.synchronize_and_revalidate(),
        Err(RequestJournalRefusalV1::Conflict) => {
            if journal.read_private_file(&name, limits.max_encoded_bytes)? == encoded {
                journal.synchronize_and_revalidate()
            } else {
                Err(RequestJournalRefusalV1::Conflict)
            }
        }
        Err(reason) => Err(reason),
    }?;
    refuse_legacy_operation_directory(directory)
}

fn recover_operation_retirements(
    directory: &Path,
    scope: RequestJournalScopeV1,
    limits: RequestJournalLimitsV1,
) -> Result<Vec<DurableOperationRetirementRecordV1>, RequestJournalRefusalV1> {
    validate_limits(limits)?;
    refuse_legacy_operation_directory(directory)?;
    let journal = match open_existing_named_directory(directory, OPERATION_DIRECTORY_NAME) {
        Ok(journal) => journal,
        Err(RequestJournalRefusalV1::Missing) => return Ok(Vec::new()),
        Err(reason) => return Err(reason),
    };
    let mut records = Vec::new();
    for name in journal.entry_names()? {
        let (name_subject, name_operation, part) =
            parse_operation_name(&name).ok_or(RequestJournalRefusalV1::Malformed)?;
        if part != OperationPartV2::Retired {
            continue;
        }
        let record = decode_operation_record(
            &journal.read_private_file(&name, limits.max_encoded_bytes)?,
            OPERATION_RETIRED_MAGIC,
            limits,
        )?;
        if record.authenticated_subject != name_subject
            || record.operation.to_bytes() != name_operation
            || record.scope.workspace() != scope.workspace()
            || record.scope.generation() > scope.generation()
            || record.frozen_reply.is_some()
        {
            return Err(RequestJournalRefusalV1::Conflict);
        }
        records
            .try_reserve(1)
            .map_err(|_| RequestJournalRefusalV1::Limit)?;
        records.push(DurableOperationRetirementRecordV1 {
            scope: record.scope,
            operation: record.operation,
            authenticated_subject: record.authenticated_subject,
            frozen_request: record.frozen_request,
        });
    }
    if records.len() > limits.max_retired_operations {
        return Err(RequestJournalRefusalV1::Limit);
    }
    journal.revalidate()?;
    refuse_legacy_operation_directory(directory)?;
    Ok(records)
}

fn retire_operation(
    directory: &Path,
    retirement: DurableOperationRetirementV1<'_>,
    limits: RequestJournalLimitsV1,
) -> Result<DurableOperationRetirementResultV1, RequestJournalRefusalV1> {
    validate_operation_payloads(retirement.frozen_request, &[], limits)?;
    refuse_legacy_operation_directory(directory)?;
    let journal = match open_existing_named_directory(directory, OPERATION_DIRECTORY_NAME) {
        Ok(journal) => journal,
        Err(RequestJournalRefusalV1::Missing) => {
            return Ok(DurableOperationRetirementResultV1::AbsentOrForeign);
        }
        Err(reason) => return Err(reason),
    };
    let retired_name =
        operation_retired_file_name(retirement.authenticated_subject, retirement.operation);
    if journal.entry_exists(&retired_name)? {
        let retired = decode_operation_record(
            &journal.read_private_file(&retired_name, limits.max_encoded_bytes)?,
            OPERATION_RETIRED_MAGIC,
            limits,
        )?;
        if retired.scope.workspace() != retirement.scope.workspace()
            || retired.scope.generation() > retirement.scope.generation()
            || retired.authenticated_subject != retirement.authenticated_subject
            || retired.operation != retirement.operation
        {
            return Err(RequestJournalRefusalV1::Conflict);
        }
        let outcome = if retired.frozen_request == retirement.frozen_request {
            DurableOperationRetirementResultV1::AbsentOrForeign
        } else {
            DurableOperationRetirementResultV1::RequestConflict
        };
        journal.revalidate()?;
        return Ok(outcome);
    }
    let intent_name =
        operation_file_name(retirement.authenticated_subject, retirement.operation, true);
    if !journal.entry_exists(&intent_name)? {
        journal.revalidate()?;
        return Ok(DurableOperationRetirementResultV1::AbsentOrForeign);
    }
    let intent = decode_operation_record(
        &journal.read_private_file(&intent_name, limits.max_encoded_bytes)?,
        OPERATION_INTENT_MAGIC,
        limits,
    )?;
    if intent.scope.workspace() != retirement.scope.workspace()
        || intent.scope.generation() > retirement.scope.generation()
        || intent.authenticated_subject != retirement.authenticated_subject
        || intent.operation != retirement.operation
        || intent.frozen_reply.is_some()
    {
        return Err(RequestJournalRefusalV1::Conflict);
    }
    if intent.frozen_request != retirement.frozen_request {
        journal.revalidate()?;
        return Ok(DurableOperationRetirementResultV1::RequestConflict);
    }
    let reply_name = operation_file_name(
        retirement.authenticated_subject,
        retirement.operation,
        false,
    );
    if !journal.entry_exists(&reply_name)? {
        journal.revalidate()?;
        return Ok(DurableOperationRetirementResultV1::Pending);
    }
    let reply = decode_operation_record(
        &journal.read_private_file(&reply_name, limits.max_encoded_bytes)?,
        OPERATION_REPLY_MAGIC,
        limits,
    )?;
    if reply.scope != intent.scope
        || reply.authenticated_subject != intent.authenticated_subject
        || reply.operation != intent.operation
        || reply.frozen_request != intent.frozen_request
        || reply.frozen_reply.as_ref().is_none_or(Vec::is_empty)
    {
        return Err(RequestJournalRefusalV1::Conflict);
    }
    let mut seen = BTreeMap::<DurableOperationKeyV2, [bool; 3]>::new();
    for name in journal.entry_names()? {
        let (subject, operation, part) =
            parse_operation_name(&name).ok_or(RequestJournalRefusalV1::Malformed)?;
        let slot = &mut seen.entry((subject, operation)).or_default()[match part {
            OperationPartV2::Intent => 0,
            OperationPartV2::Reply => 1,
            OperationPartV2::Retired => 2,
        }];
        if *slot {
            return Err(RequestJournalRefusalV1::Malformed);
        }
        *slot = true;
    }
    let retired_count = seen.values().filter(|parts| parts[2]).count();
    if retired_count >= limits.max_retired_operations {
        return Err(RequestJournalRefusalV1::Full);
    }
    let marker_intent = DurableOperationIntentV1 {
        scope: intent.scope,
        operation: intent.operation,
        authenticated_subject: intent.authenticated_subject,
        frozen_request: &intent.frozen_request,
    };
    let encoded = encode_operation_record(OPERATION_RETIRED_MAGIC, marker_intent, &[], limits)?;
    journal.create_private_file(&retired_name, &encoded)?;
    // Commit point: active capacity is not released until this directory sync.
    journal.synchronize_and_revalidate()?;
    finish_committed_retirement(
        &journal,
        &intent,
        Some(intent_name),
        Some(reply_name),
        limits,
    )?;
    Ok(DurableOperationRetirementResultV1::Retired)
}

fn finish_committed_retirement(
    journal: &AnchoredDirectory,
    retired: &DurableOperationRecordV1,
    intent_name: Option<String>,
    reply_name: Option<String>,
    limits: RequestJournalLimitsV1,
) -> Result<(), RequestJournalRefusalV1> {
    if let Some(intent_name) = intent_name.as_deref() {
        let intent = decode_operation_record(
            &journal.read_private_file(intent_name, limits.max_encoded_bytes)?,
            OPERATION_INTENT_MAGIC,
            limits,
        )?;
        if intent.scope != retired.scope
            || intent.operation != retired.operation
            || intent.authenticated_subject != retired.authenticated_subject
            || intent.frozen_request != retired.frozen_request
            || intent.frozen_reply.is_some()
        {
            return Err(RequestJournalRefusalV1::Conflict);
        }
    }
    if let Some(reply_name) = reply_name.as_deref() {
        let reply = decode_operation_record(
            &journal.read_private_file(reply_name, limits.max_encoded_bytes)?,
            OPERATION_REPLY_MAGIC,
            limits,
        )?;
        if reply.scope != retired.scope
            || reply.operation != retired.operation
            || reply.authenticated_subject != retired.authenticated_subject
            || reply.frozen_request != retired.frozen_request
            || reply.frozen_reply.as_ref().is_none_or(Vec::is_empty)
        {
            return Err(RequestJournalRefusalV1::Conflict);
        }
    }
    let removed = intent_name.is_some() || reply_name.is_some();
    // Mutate only after every surviving component has independently validated
    // against the committed tombstone.
    if let Some(reply_name) = reply_name.as_deref() {
        journal.remove_private_file(reply_name)?;
    }
    if let Some(intent_name) = intent_name.as_deref() {
        journal.remove_private_file(intent_name)?;
    }
    if removed {
        journal.synchronize_and_revalidate()
    } else {
        journal.revalidate()
    }
}

fn validate_operation_payloads(
    request: &[u8],
    reply: &[u8],
    limits: RequestJournalLimitsV1,
) -> Result<(), RequestJournalRefusalV1> {
    validate_limits(limits)?;
    if request.is_empty()
        || request.len() > limits.max_request_bytes
        || reply.len() > limits.max_reply_bytes
        || OPERATION_HEADER_BYTES
            .checked_add(request.len())
            .and_then(|n| n.checked_add(reply.len()))
            .is_none_or(|n| n > limits.max_encoded_bytes)
    {
        return Err(RequestJournalRefusalV1::Limit);
    }
    Ok(())
}

fn encode_operation_record(
    magic: &[u8; 8],
    intent: DurableOperationIntentV1<'_>,
    reply: &[u8],
    limits: RequestJournalLimitsV1,
) -> Result<Vec<u8>, RequestJournalRefusalV1> {
    validate_operation_payloads(intent.frozen_request, reply, limits)?;
    let mut encoded =
        Vec::with_capacity(OPERATION_HEADER_BYTES + intent.frozen_request.len() + reply.len());
    encoded.extend_from_slice(magic);
    encoded.extend_from_slice(&intent.scope.workspace().to_be_bytes());
    encoded.extend_from_slice(&intent.scope.generation().to_be_bytes());
    encoded.extend_from_slice(&intent.operation.to_bytes());
    encoded.extend_from_slice(&intent.authenticated_subject);
    encoded.extend_from_slice(
        &u64::try_from(intent.frozen_request.len())
            .map_err(|_| RequestJournalRefusalV1::Limit)?
            .to_be_bytes(),
    );
    encoded.extend_from_slice(intent.frozen_request);
    encoded.extend_from_slice(reply);
    Ok(encoded)
}

fn decode_operation_record(
    encoded: &[u8],
    magic: &[u8; 8],
    limits: RequestJournalLimitsV1,
) -> Result<DurableOperationRecordV1, RequestJournalRefusalV1> {
    if encoded.len() < OPERATION_HEADER_BYTES || &encoded[..8] != magic {
        return Err(RequestJournalRefusalV1::Malformed);
    }
    let u64_at = |offset: usize| -> Result<u64, RequestJournalRefusalV1> {
        Ok(u64::from_be_bytes(
            encoded
                .get(offset..offset + 8)
                .ok_or(RequestJournalRefusalV1::Malformed)?
                .try_into()
                .map_err(|_| RequestJournalRefusalV1::Malformed)?,
        ))
    };
    let scope = RequestJournalScopeV1::new(u64_at(8)?, u64_at(16)?)
        .ok_or(RequestJournalRefusalV1::Malformed)?;
    let operation = DurableOperationRefV1::new(
        encoded[24..40]
            .try_into()
            .map_err(|_| RequestJournalRefusalV1::Malformed)?,
    )
    .ok_or(RequestJournalRefusalV1::Malformed)?;
    let authenticated_subject = encoded[40..56]
        .try_into()
        .map_err(|_| RequestJournalRefusalV1::Malformed)?;
    let request_len = usize::try_from(u64_at(56)?).map_err(|_| RequestJournalRefusalV1::Limit)?;
    let request_end = OPERATION_HEADER_BYTES
        .checked_add(request_len)
        .ok_or(RequestJournalRefusalV1::Limit)?;
    let frozen_request = encoded
        .get(OPERATION_HEADER_BYTES..request_end)
        .ok_or(RequestJournalRefusalV1::Malformed)?
        .to_vec();
    let reply = encoded
        .get(request_end..)
        .ok_or(RequestJournalRefusalV1::Malformed)?
        .to_vec();
    validate_operation_payloads(&frozen_request, &reply, limits)?;
    Ok(DurableOperationRecordV1 {
        scope,
        operation,
        authenticated_subject,
        frozen_request,
        frozen_reply: (!reply.is_empty()).then_some(reply),
    })
}

fn operation_file_name(
    authenticated_subject: [u8; 16],
    operation: DurableOperationRefV1,
    intent: bool,
) -> String {
    let mut value = String::with_capacity(64 + 7);
    for byte in authenticated_subject {
        use std::fmt::Write as _;
        let _ = write!(&mut value, "{byte:02x}");
    }
    for byte in operation.to_bytes() {
        use std::fmt::Write as _;
        let _ = write!(&mut value, "{byte:02x}");
    }
    value.push_str(if intent { ".intent" } else { ".reply" });
    value
}

fn operation_retired_file_name(
    authenticated_subject: [u8; 16],
    operation: DurableOperationRefV1,
) -> String {
    let mut value = operation_file_name(authenticated_subject, operation, true);
    value.truncate(value.len() - ".intent".len());
    value.push_str(".retired");
    value
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum OperationPartV2 {
    Intent,
    Reply,
    Retired,
}

fn parse_operation_name(name: &str) -> Option<([u8; 16], [u8; 16], OperationPartV2)> {
    let (stem, suffix) = name.rsplit_once('.')?;
    if stem.len() != 64 {
        return None;
    }
    if !stem
        .bytes()
        .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return None;
    }
    let part = match suffix {
        "intent" => OperationPartV2::Intent,
        "reply" => OperationPartV2::Reply,
        "retired" => OperationPartV2::Retired,
        _ => return None,
    };
    let mut subject = [0u8; 16];
    for (index, byte) in subject.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&stem[index * 2..index * 2 + 2], 16).ok()?;
    }
    let mut operation = [0u8; 16];
    for (index, byte) in operation.iter_mut().enumerate() {
        let offset = 32 + index * 2;
        *byte = u8::from_str_radix(&stem[offset..offset + 2], 16).ok()?;
    }
    DurableOperationRefV1::new(operation)?;
    Some((subject, operation, part))
}

fn refuse_legacy_operation_directory(directory: &Path) -> Result<(), RequestJournalRefusalV1> {
    // The caller supplies the trusted workspace directory. Pin it for this
    // observation so a raced legacy-name replacement cannot redirect lookup.
    let parent = open_directory_descriptor(directory)?;
    match statat(
        &parent,
        LEGACY_OPERATION_DIRECTORY_NAME,
        AtFlags::SYMLINK_NOFOLLOW,
    ) {
        Ok(_) => Err(RequestJournalRefusalV1::Conflict),
        Err(error) if error == rustix::io::Errno::NOENT => Ok(()),
        Err(_) => Err(RequestJournalRefusalV1::Io),
    }
}

fn recover(
    directory: &Path,
    scope: RequestJournalScopeV1,
    limits: RequestJournalLimitsV1,
) -> Result<Vec<RequestJournalRecordV1>, RequestJournalRefusalV1> {
    validate_limits(limits)?;
    let journal = match open_existing_directory(directory) {
        Ok(journal) => journal,
        Err(RequestJournalRefusalV1::Missing) => return Ok(Vec::new()),
        Err(reason) => return Err(reason),
    };
    let mut parts: BTreeMap<(u64, u64), (Option<String>, Option<String>)> = BTreeMap::new();
    for name in journal.entry_names()? {
        let Some((client, request, is_intent)) = parse_name(&name) else {
            return Err(RequestJournalRefusalV1::Malformed);
        };
        let pair = parts.entry((client, request)).or_default();
        let slot = if is_intent { &mut pair.0 } else { &mut pair.1 };
        if slot.replace(name).is_some() {
            return Err(RequestJournalRefusalV1::Malformed);
        }
    }
    if parts.len() > limits.max_entries {
        return Err(RequestJournalRefusalV1::Limit);
    }
    let mut records = Vec::new();
    records
        .try_reserve_exact(parts.len())
        .map_err(|_| RequestJournalRefusalV1::Limit)?;
    for ((client, request), (intent, reply)) in parts {
        let intent = intent.ok_or(RequestJournalRefusalV1::Malformed)?;
        let intent_bytes = journal.read_private_file(&intent, limits.max_encoded_bytes)?;
        let intent = decode_record(&intent_bytes, INTENT_MAGIC, limits)?;
        if !intent.frozen_reply.is_empty()
            || intent.scope.workspace() != scope.workspace()
            || intent.scope.generation() > scope.generation()
            || intent.client.get() != client
            || intent.request.get() != request
        {
            return Err(RequestJournalRefusalV1::Conflict);
        }
        let reply = reply.ok_or(RequestJournalRefusalV1::Pending)?;
        let reply_bytes = journal.read_private_file(&reply, limits.max_encoded_bytes)?;
        let reply = decode_record(&reply_bytes, REPLY_MAGIC, limits)?;
        if reply.scope != intent.scope
            || reply.client != intent.client
            || reply.request != intent.request
            || reply.authenticated_subject != intent.authenticated_subject
            || reply.frozen_request != intent.frozen_request
            || reply.frozen_reply.is_empty()
        {
            return Err(RequestJournalRefusalV1::Conflict);
        }
        records.push(RequestJournalRecordV1 {
            scope: intent.scope,
            client: intent.client,
            request: intent.request,
            authenticated_subject: intent.authenticated_subject,
            frozen_request: intent.frozen_request,
            frozen_reply: reply.frozen_reply,
        });
    }
    journal.revalidate()?;
    Ok(records)
}

fn begin(
    directory: &Path,
    intent: RequestJournalIntentV1<'_>,
    limits: RequestJournalLimitsV1,
) -> Result<(), RequestJournalRefusalV1> {
    validate_payloads(intent.frozen_request, &[], limits)?;
    let journal = open_or_create_directory(directory)?;
    let name = file_name(intent.client, intent.request, true);
    let encoded = encode_record(INTENT_MAGIC, intent, &[], limits)?;
    let mut existing = 0_usize;
    for entry_name in journal.entry_names()? {
        let Some((_, _, intent)) = parse_name(&entry_name) else {
            return Err(RequestJournalRefusalV1::Malformed);
        };
        if intent {
            existing = existing
                .checked_add(1)
                .ok_or(RequestJournalRefusalV1::Limit)?;
        }
    }
    let already_exists = journal.entry_exists(&name)?;
    if !already_exists && existing >= limits.max_entries {
        return Err(RequestJournalRefusalV1::Full);
    }
    match journal.create_private_file(&name, &encoded) {
        Ok(()) => journal.synchronize_and_revalidate(),
        Err(RequestJournalRefusalV1::Conflict) => {
            let existing = journal.read_private_file(&name, limits.max_encoded_bytes)?;
            if existing == encoded {
                journal.synchronize_and_revalidate()
            } else {
                Err(RequestJournalRefusalV1::Conflict)
            }
        }
        Err(reason) => Err(reason),
    }
}

fn complete(
    directory: &Path,
    completion: RequestJournalCompletionV1<'_>,
    limits: RequestJournalLimitsV1,
) -> Result<(), RequestJournalRefusalV1> {
    validate_payloads(
        completion.intent.frozen_request,
        completion.frozen_reply,
        limits,
    )?;
    let journal = open_existing_directory(directory)?;
    let intent_name = file_name(completion.intent.client, completion.intent.request, true);
    let intent = journal.read_private_file(&intent_name, limits.max_encoded_bytes)?;
    let expected_intent = encode_record(INTENT_MAGIC, completion.intent, &[], limits)?;
    if intent != expected_intent {
        return Err(RequestJournalRefusalV1::Conflict);
    }
    let name = file_name(completion.intent.client, completion.intent.request, false);
    let encoded = encode_record(
        REPLY_MAGIC,
        completion.intent,
        completion.frozen_reply,
        limits,
    )?;
    match journal.create_private_file(&name, &encoded) {
        Ok(()) => journal.synchronize_and_revalidate(),
        Err(RequestJournalRefusalV1::Conflict) => {
            let existing = journal.read_private_file(&name, limits.max_encoded_bytes)?;
            if existing == encoded {
                journal.synchronize_and_revalidate()
            } else {
                Err(RequestJournalRefusalV1::Conflict)
            }
        }
        Err(reason) => Err(reason),
    }
}

fn validate_limits(limits: RequestJournalLimitsV1) -> Result<(), RequestJournalRefusalV1> {
    if limits.max_entries == 0
        || limits.max_retired_operations == 0
        || limits.max_request_bytes == 0
        || limits.max_reply_bytes == 0
        || limits.max_encoded_bytes < HEADER_BYTES
    {
        Err(RequestJournalRefusalV1::InvalidLimits)
    } else {
        Ok(())
    }
}

fn validate_payloads(
    request: &[u8],
    reply: &[u8],
    limits: RequestJournalLimitsV1,
) -> Result<(), RequestJournalRefusalV1> {
    validate_limits(limits)?;
    if request.is_empty()
        || request.len() > limits.max_request_bytes
        || reply.len() > limits.max_reply_bytes
    {
        return Err(RequestJournalRefusalV1::Limit);
    }
    let encoded = HEADER_BYTES
        .checked_add(request.len())
        .and_then(|value| value.checked_add(reply.len()))
        .ok_or(RequestJournalRefusalV1::Limit)?;
    if encoded > limits.max_encoded_bytes {
        return Err(RequestJournalRefusalV1::Limit);
    }
    Ok(())
}

fn encode_record(
    magic: &[u8; 8],
    intent: RequestJournalIntentV1<'_>,
    frozen_reply: &[u8],
    limits: RequestJournalLimitsV1,
) -> Result<Vec<u8>, RequestJournalRefusalV1> {
    validate_payloads(intent.frozen_request, frozen_reply, limits)?;
    let mut encoded =
        Vec::with_capacity(HEADER_BYTES + intent.frozen_request.len() + frozen_reply.len());
    encoded.extend_from_slice(magic);
    for value in [
        intent.scope.workspace(),
        intent.scope.generation(),
        intent.client.get(),
        intent.request.get(),
        u64::try_from(intent.frozen_request.len()).map_err(|_| RequestJournalRefusalV1::Limit)?,
    ] {
        encoded.extend_from_slice(&value.to_be_bytes());
    }
    encoded.extend_from_slice(&intent.authenticated_subject);
    encoded.extend_from_slice(intent.frozen_request);
    encoded.extend_from_slice(frozen_reply);
    Ok(encoded)
}

fn decode_record(
    encoded: &[u8],
    magic: &[u8; 8],
    limits: RequestJournalLimitsV1,
) -> Result<DecodedRecord, RequestJournalRefusalV1> {
    if encoded.len() < HEADER_BYTES || &encoded[..8] != magic {
        return Err(RequestJournalRefusalV1::Malformed);
    }
    let mut offset = 8;
    let mut next = || {
        let end = offset + 8;
        let bytes: [u8; 8] = encoded.get(offset..end)?.try_into().ok()?;
        offset = end;
        Some(u64::from_be_bytes(bytes))
    };
    let scope = RequestJournalScopeV1::new(
        next().ok_or(RequestJournalRefusalV1::Malformed)?,
        next().ok_or(RequestJournalRefusalV1::Malformed)?,
    )
    .ok_or(RequestJournalRefusalV1::Malformed)?;
    let client = NonZeroU64::new(next().ok_or(RequestJournalRefusalV1::Malformed)?)
        .ok_or(RequestJournalRefusalV1::Malformed)?;
    let request = NonZeroU64::new(next().ok_or(RequestJournalRefusalV1::Malformed)?)
        .ok_or(RequestJournalRefusalV1::Malformed)?;
    let request_len = usize::try_from(next().ok_or(RequestJournalRefusalV1::Malformed)?)
        .map_err(|_| RequestJournalRefusalV1::Limit)?;
    let subject: [u8; 16] = encoded
        .get(48..64)
        .ok_or(RequestJournalRefusalV1::Malformed)?
        .try_into()
        .map_err(|_| RequestJournalRefusalV1::Malformed)?;
    if request_len == 0
        || request_len > limits.max_request_bytes
        || encoded.len() > limits.max_encoded_bytes
    {
        return Err(RequestJournalRefusalV1::Limit);
    }
    let request_end = HEADER_BYTES
        .checked_add(request_len)
        .ok_or(RequestJournalRefusalV1::Limit)?;
    let frozen_request = encoded
        .get(HEADER_BYTES..request_end)
        .ok_or(RequestJournalRefusalV1::Malformed)?
        .to_vec();
    let frozen_reply = encoded
        .get(request_end..)
        .ok_or(RequestJournalRefusalV1::Malformed)?
        .to_vec();
    if frozen_reply.len() > limits.max_reply_bytes {
        return Err(RequestJournalRefusalV1::Limit);
    }
    Ok(DecodedRecord {
        scope,
        client,
        request,
        authenticated_subject: subject,
        frozen_request,
        frozen_reply,
    })
}

fn file_name(client: NonZeroU64, request: NonZeroU64, intent: bool) -> String {
    format!(
        "c{:016x}-r{:016x}.{}",
        client.get(),
        request.get(),
        if intent { "intent" } else { "reply" }
    )
}

fn parse_name(name: &str) -> Option<(u64, u64, bool)> {
    let (stem, suffix) = name.rsplit_once('.')?;
    let is_intent = match suffix {
        "intent" => true,
        "reply" => false,
        _ => return None,
    };
    if stem.len() != 35 || !stem.starts_with('c') || &stem[17..19] != "-r" {
        return None;
    }
    let client = u64::from_str_radix(&stem[1..17], 16).ok()?;
    let request = u64::from_str_radix(&stem[19..35], 16).ok()?;
    NonZeroU64::new(client)?;
    NonZeroU64::new(request)?;
    Some((client, request, is_intent))
}

fn open_or_create_directory(parent: &Path) -> Result<AnchoredDirectory, RequestJournalRefusalV1> {
    open_or_create_named_directory(parent, DIRECTORY_NAME)
}

fn open_or_create_named_directory(
    parent: &Path,
    name: &str,
) -> Result<AnchoredDirectory, RequestJournalRefusalV1> {
    let path = parent.join(name);
    let parent_descriptor = open_directory_descriptor(parent)?;
    match mkdirat(&parent_descriptor, name, Mode::from_raw_mode(0o700)) {
        Ok(()) => {}
        Err(error) if error == rustix::io::Errno::EXIST => {}
        Err(_) => return Err(RequestJournalRefusalV1::Io),
    }
    // This also discharges a prior call whose mkdir succeeded but whose parent
    // sync returned outcome-unknown before this call observed EEXIST.
    synchronize(&parent_descriptor).map_err(|_| RequestJournalRefusalV1::OutcomeUnknown)?;
    open_anchored_directory(&parent_descriptor, name, path)
}

fn open_existing_directory(parent: &Path) -> Result<AnchoredDirectory, RequestJournalRefusalV1> {
    open_existing_named_directory(parent, DIRECTORY_NAME)
}

fn open_existing_named_directory(
    parent: &Path,
    name: &str,
) -> Result<AnchoredDirectory, RequestJournalRefusalV1> {
    let path = parent.join(name);
    let parent_descriptor = open_directory_descriptor(parent)?;
    open_anchored_directory(&parent_descriptor, name, path)
}

fn open_directory_descriptor(path: &Path) -> Result<File, RequestJournalRefusalV1> {
    let flags = i32::try_from((OFlags::NOFOLLOW | OFlags::DIRECTORY | OFlags::CLOEXEC).bits())
        .map_err(|_| RequestJournalRefusalV1::Io)?;
    OpenOptions::new()
        .read(true)
        .custom_flags(flags)
        .open(path)
        .map_err(|error| {
            if error.kind() == std::io::ErrorKind::NotFound {
                RequestJournalRefusalV1::Missing
            } else {
                RequestJournalRefusalV1::Io
            }
        })
}

fn open_anchored_directory(
    parent: &File,
    name: &str,
    namespace_path: PathBuf,
) -> Result<AnchoredDirectory, RequestJournalRefusalV1> {
    let descriptor = File::from(
        openat(
            parent,
            name,
            OFlags::RDONLY | OFlags::CLOEXEC | OFlags::NOFOLLOW | OFlags::DIRECTORY,
            Mode::empty(),
        )
        .map_err(|error| {
            if error == rustix::io::Errno::NOENT {
                RequestJournalRefusalV1::Missing
            } else if error == rustix::io::Errno::LOOP || error == rustix::io::Errno::NOTDIR {
                RequestJournalRefusalV1::UnsupportedResource
            } else {
                RequestJournalRefusalV1::Io
            }
        })?,
    );
    let metadata = descriptor
        .metadata()
        .map_err(|_| RequestJournalRefusalV1::Io)?;
    if !metadata.file_type().is_dir()
        || metadata.uid() != rustix::process::geteuid().as_raw()
        || metadata.mode() & 0o7777 != 0o700
    {
        return Err(RequestJournalRefusalV1::UnsupportedResource);
    }
    Ok(AnchoredDirectory {
        descriptor,
        namespace_path,
        identity: DirectoryIdentity {
            device: metadata.dev(),
            inode: metadata.ino(),
        },
    })
}

impl AnchoredDirectory {
    fn entry_names(&self) -> Result<Vec<String>, RequestJournalRefusalV1> {
        let mut directory =
            Dir::read_from(&self.descriptor).map_err(|_| RequestJournalRefusalV1::Io)?;
        let mut names = Vec::new();
        for entry in &mut directory {
            let entry = entry.map_err(|_| RequestJournalRefusalV1::Io)?;
            let bytes = entry.file_name().to_bytes();
            if bytes == b"." || bytes == b".." {
                continue;
            }
            let name =
                std::str::from_utf8(bytes).map_err(|_| RequestJournalRefusalV1::Malformed)?;
            names
                .try_reserve(1)
                .map_err(|_| RequestJournalRefusalV1::Limit)?;
            names.push(name.to_owned());
        }
        Ok(names)
    }

    fn entry_exists(&self, name: &str) -> Result<bool, RequestJournalRefusalV1> {
        match statat(&self.descriptor, name, AtFlags::SYMLINK_NOFOLLOW) {
            Ok(_) => Ok(true),
            Err(error) if error == rustix::io::Errno::NOENT => Ok(false),
            Err(_) => Err(RequestJournalRefusalV1::Io),
        }
    }

    fn create_private_file(&self, name: &str, bytes: &[u8]) -> Result<(), RequestJournalRefusalV1> {
        let mut file = File::from(
            openat(
                &self.descriptor,
                name,
                OFlags::WRONLY | OFlags::CREATE | OFlags::EXCL | OFlags::CLOEXEC | OFlags::NOFOLLOW,
                Mode::from_raw_mode(0o600),
            )
            .map_err(|error| {
                if error == rustix::io::Errno::EXIST {
                    RequestJournalRefusalV1::Conflict
                } else {
                    RequestJournalRefusalV1::Io
                }
            })?,
        );
        file.write_all(bytes)
            .map_err(|_| RequestJournalRefusalV1::OutcomeUnknown)?;
        synchronize(&file).map_err(|_| RequestJournalRefusalV1::OutcomeUnknown)
    }

    fn read_private_file(
        &self,
        name: &str,
        max: usize,
    ) -> Result<Vec<u8>, RequestJournalRefusalV1> {
        let file = File::from(
            openat(
                &self.descriptor,
                name,
                OFlags::RDONLY | OFlags::CLOEXEC | OFlags::NOFOLLOW | OFlags::NONBLOCK,
                Mode::empty(),
            )
            .map_err(|error| {
                if error == rustix::io::Errno::NOENT {
                    RequestJournalRefusalV1::Missing
                } else if error == rustix::io::Errno::LOOP {
                    RequestJournalRefusalV1::UnsupportedResource
                } else {
                    RequestJournalRefusalV1::Io
                }
            })?,
        );
        let metadata = file.metadata().map_err(|_| RequestJournalRefusalV1::Io)?;
        if !metadata.file_type().is_file()
            || metadata.file_type().is_socket()
            || metadata.uid() != rustix::process::geteuid().as_raw()
            || metadata.mode() & 0o7777 != 0o600
            || metadata.nlink() != 1
            || usize::try_from(metadata.len()).map_or(true, |length| length > max)
        {
            return Err(RequestJournalRefusalV1::UnsupportedResource);
        }
        let mut bytes = Vec::new();
        file.take(u64::try_from(max).map_err(|_| RequestJournalRefusalV1::Limit)? + 1)
            .read_to_end(&mut bytes)
            .map_err(|_| RequestJournalRefusalV1::Io)?;
        if bytes.len() > max {
            return Err(RequestJournalRefusalV1::Limit);
        }
        Ok(bytes)
    }

    fn revalidate(&self) -> Result<(), RequestJournalRefusalV1> {
        let observed = open_directory_descriptor(&self.namespace_path).and_then(|directory| {
            let metadata = directory
                .metadata()
                .map_err(|_| RequestJournalRefusalV1::Io)?;
            Ok(DirectoryIdentity {
                device: metadata.dev(),
                inode: metadata.ino(),
            })
        });
        match observed {
            Ok(identity) if identity == self.identity => Ok(()),
            Ok(_) | Err(RequestJournalRefusalV1::Missing) => Err(RequestJournalRefusalV1::Conflict),
            Err(reason) => Err(reason),
        }
    }

    fn synchronize_and_revalidate(&self) -> Result<(), RequestJournalRefusalV1> {
        synchronize(&self.descriptor).map_err(|_| RequestJournalRefusalV1::OutcomeUnknown)?;
        self.revalidate()
            .map_err(|_| RequestJournalRefusalV1::OutcomeUnknown)
    }

    fn remove_private_file(&self, name: &str) -> Result<(), RequestJournalRefusalV1> {
        unlinkat(&self.descriptor, name, AtFlags::empty()).map_err(|error| {
            if error == rustix::io::Errno::NOENT {
                RequestJournalRefusalV1::Conflict
            } else {
                RequestJournalRefusalV1::OutcomeUnknown
            }
        })
    }
}

fn synchronize(file: &File) -> Result<(), ()> {
    #[cfg(target_os = "macos")]
    {
        rustix::fs::fcntl_fullfsync(file).map_err(|_| ())
    }
    #[cfg(not(target_os = "macos"))]
    {
        file.sync_all().map_err(|_| ())
    }
}

fn observe_refusal(usage: &mut LocalRequestJournalUsageV1, reason: RequestJournalRefusalV1) {
    if reason == RequestJournalRefusalV1::OutcomeUnknown {
        bump(&mut usage.outcome_unknown, &mut usage.counter_saturated);
    } else {
        bump(&mut usage.refused, &mut usage.counter_saturated);
    }
}

fn bump(value: &mut u64, saturated: &mut bool) {
    if let Some(next) = value.checked_add(1) {
        *value = next;
    } else {
        *saturated = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        fs,
        os::unix::fs::{PermissionsExt, symlink},
        sync::atomic::{AtomicU64, Ordering},
    };
    use vellum_host::{
        request_journal::{
            CanonicalDurableEffectV1, CanonicalDurableOutcomeV1, DurableEffectRecoveryStateV1,
            DurableOperationOwnerV1, validate_recovered_durable_effect_v1,
        },
        workspace::{
            WorkspaceIdentityAllocationV1, WorkspaceIdentityLimitsV1, WorkspaceIdentityMetadataV1,
        },
    };
    use vellum_protocol::{
        DurableEffectIntentV1, DurableEffectReceiptV1, DurableExecuteOutcomeV1,
        DurableExportCopyPersistenceV1, DurableExportCopyReceiptV1, WorkspaceDocumentIdV1,
        WorkspaceIdV1,
    };

    static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(1);

    struct Fixture(PathBuf);

    impl Fixture {
        fn new() -> Self {
            let ordinal = NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "vellum-request-journal-{}-{ordinal}",
                std::process::id()
            ));
            fs::create_dir(&path).unwrap();
            Self(path)
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn limits(max_entries: usize) -> RequestJournalLimitsV1 {
        RequestJournalLimitsV1 {
            max_entries,
            max_retired_operations: max_entries.saturating_mul(4).max(1),
            max_request_bytes: 128,
            max_reply_bytes: 128,
            max_encoded_bytes: 320,
        }
    }

    fn scope(generation: u64) -> RequestJournalScopeV1 {
        RequestJournalScopeV1::new(7, generation).unwrap()
    }

    fn intent<'a>(
        generation: u64,
        client: NonZeroU64,
        request: NonZeroU64,
        subject: [u8; 16],
        frozen_request: &'a [u8],
    ) -> RequestJournalIntentV1<'a> {
        RequestJournalIntentV1 {
            scope: scope(generation),
            client,
            request,
            authenticated_subject: subject,
            frozen_request,
        }
    }

    #[test]
    fn complete_record_recovers_exact_plaintext_and_subject_across_later_generation() {
        let fixture = Fixture::new();
        let mut journal = LocalRequestJournalV1::default();
        let client = NonZeroU64::new(11).unwrap();
        let request = NonZeroU64::new(13).unwrap();
        journal
            .begin(
                &fixture.0,
                intent(2, client, request, [3; 16], b"request"),
                limits(2),
            )
            .unwrap();
        journal
            .complete(
                &fixture.0,
                RequestJournalCompletionV1 {
                    intent: intent(2, client, request, [3; 16], b"request"),
                    frozen_reply: b"reply",
                },
                limits(2),
            )
            .unwrap();
        let RequestJournalRecoveryV1::Recovered(records) =
            journal.recover(&fixture.0, scope(4), limits(2))
        else {
            panic!("expected one recovered receipt");
        };
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].authenticated_subject, [3; 16]);
        assert_eq!(records[0].frozen_request, b"request");
        assert_eq!(records[0].frozen_reply, b"reply");
    }

    #[test]
    fn pending_intent_and_changed_subject_fail_closed() {
        let fixture = Fixture::new();
        let mut journal = LocalRequestJournalV1::default();
        let client = NonZeroU64::new(1).unwrap();
        let request = NonZeroU64::new(1).unwrap();
        journal
            .begin(
                &fixture.0,
                intent(1, client, request, [1; 16], b"request"),
                limits(1),
            )
            .unwrap();
        assert_eq!(
            journal.recover(&fixture.0, scope(1), limits(1)),
            RequestJournalRecoveryV1::Refused(RequestJournalRefusalV1::Pending)
        );
        assert_eq!(
            journal.begin(
                &fixture.0,
                intent(1, client, request, [2; 16], b"request"),
                limits(1),
            ),
            Err(RequestJournalRefusalV1::Conflict)
        );
    }

    #[test]
    fn capacity_is_explicit_and_never_evicts() {
        let fixture = Fixture::new();
        let mut journal = LocalRequestJournalV1::default();
        journal
            .begin(
                &fixture.0,
                intent(
                    1,
                    NonZeroU64::new(1).unwrap(),
                    NonZeroU64::new(1).unwrap(),
                    [1; 16],
                    b"one",
                ),
                limits(1),
            )
            .unwrap();
        assert_eq!(
            journal.begin(
                &fixture.0,
                intent(
                    1,
                    NonZeroU64::new(2).unwrap(),
                    NonZeroU64::new(2).unwrap(),
                    [2; 16],
                    b"two",
                ),
                limits(1),
            ),
            Err(RequestJournalRefusalV1::Full)
        );
        assert!(
            fixture
                .0
                .join(DIRECTORY_NAME)
                .join(file_name(
                    NonZeroU64::new(1).unwrap(),
                    NonZeroU64::new(1).unwrap(),
                    true
                ))
                .exists()
        );
    }

    #[test]
    fn trailing_bytes_unsafe_mode_and_symlink_are_refused() {
        let fixture = Fixture::new();
        let mut journal = LocalRequestJournalV1::default();
        let client = NonZeroU64::new(1).unwrap();
        let request = NonZeroU64::new(1).unwrap();
        journal
            .begin(
                &fixture.0,
                intent(1, client, request, [1; 16], b"request"),
                limits(1),
            )
            .unwrap();
        let intent = fixture
            .0
            .join(DIRECTORY_NAME)
            .join(file_name(client, request, true));
        OpenOptions::new()
            .append(true)
            .open(&intent)
            .unwrap()
            .write_all(b"x")
            .unwrap();
        assert_eq!(
            journal.recover(&fixture.0, scope(1), limits(1)),
            RequestJournalRecoveryV1::Refused(RequestJournalRefusalV1::Conflict)
        );

        fs::set_permissions(&intent, fs::Permissions::from_mode(0o644)).unwrap();
        assert_eq!(
            journal.recover(&fixture.0, scope(1), limits(1)),
            RequestJournalRecoveryV1::Refused(RequestJournalRefusalV1::UnsupportedResource)
        );

        fs::remove_file(&intent).unwrap();
        symlink(&fixture.0, &intent).unwrap();
        assert_eq!(
            journal.recover(&fixture.0, scope(1), limits(1)),
            RequestJournalRecoveryV1::Refused(RequestJournalRefusalV1::UnsupportedResource)
        );
    }

    fn operation(value: u8) -> DurableOperationRefV1 {
        DurableOperationRefV1::new([value; 16]).unwrap()
    }

    fn operation_intent(
        value: u8,
        subject: [u8; 16],
        request: &[u8],
    ) -> DurableOperationIntentV1<'_> {
        DurableOperationIntentV1 {
            scope: scope(1),
            operation: operation(value),
            authenticated_subject: subject,
            frozen_request: request,
        }
    }

    fn export_intent_and_outcome() -> (DurableEffectIntentV1, DurableExecuteOutcomeV1) {
        let workspace = WorkspaceIdV1::new(7).unwrap();
        let document = WorkspaceDocumentIdV1::new(workspace, 2).unwrap();
        (
            DurableEffectIntentV1::ExportDocumentCopyExclusive {
                document,
                expected_revision: 11,
                destination: "/private/destination".to_owned(),
            },
            DurableExecuteOutcomeV1::Completed(
                DurableEffectReceiptV1::DocumentCopyExportedExclusive(
                    DurableExportCopyReceiptV1::try_new(
                        document,
                        11,
                        DurableExportCopyPersistenceV1::Durable,
                        None,
                        false,
                    )
                    .unwrap(),
                ),
            ),
        )
    }

    fn export_identity() -> WorkspaceIdentityMetadataV1 {
        WorkspaceIdentityMetadataV1::try_new(
            7,
            2,
            WorkspaceIdentityAllocationV1::try_new(2, 8, 1, 8).unwrap(),
            WorkspaceIdentityLimitsV1 {
                max_documents: 8,
                max_roots: 8,
                max_encoded_bytes: 4096,
            },
        )
        .unwrap()
    }

    #[test]
    fn export_restart_crash_cut_preserves_unknown_then_exact_complete_receipt() {
        const INTENT: &[u8] = b"canonical-export-intent";
        const OUTCOME: &[u8] = b"canonical-export-outcome";
        let fixture = Fixture::new();
        let operation_intent = operation_intent(7, [3; 16], INTENT);
        LocalRequestJournalV1::default()
            .begin_operation(&fixture.0, operation_intent, limits(2))
            .unwrap();

        let DurableOperationRecoveryV1::Recovered(pending) =
            LocalRequestJournalV1::default().recover_operations(&fixture.0, scope(2), limits(2))
        else {
            panic!("synchronized export intent must recover after the crash cut")
        };
        let (intent, outcome) = export_intent_and_outcome();
        let owner = DurableOperationOwnerV1 {
            scope: scope(2),
            authenticated_subject: [3; 16],
        };
        assert_eq!(
            validate_recovered_durable_effect_v1(
                &pending[0],
                owner,
                CanonicalDurableEffectV1 {
                    intent: &intent,
                    canonical_intent: INTENT,
                    outcome: CanonicalDurableOutcomeV1::Pending,
                },
                &export_identity(),
            ),
            Ok(DurableEffectRecoveryStateV1::OutcomeUnknown)
        );

        LocalRequestJournalV1::default()
            .complete_operation(
                &fixture.0,
                DurableOperationCompletionV1 {
                    intent: operation_intent,
                    frozen_reply: OUTCOME,
                },
                limits(2),
            )
            .unwrap();
        let DurableOperationRecoveryV1::Recovered(complete) =
            LocalRequestJournalV1::default().recover_operations(&fixture.0, scope(2), limits(2))
        else {
            panic!("completed export receipt must recover after restart")
        };
        assert_eq!(
            validate_recovered_durable_effect_v1(
                &complete[0],
                owner,
                CanonicalDurableEffectV1 {
                    intent: &intent,
                    canonical_intent: INTENT,
                    outcome: CanonicalDurableOutcomeV1::Complete {
                        outcome: &outcome,
                        canonical: OUTCOME,
                    },
                },
                &export_identity(),
            ),
            Ok(DurableEffectRecoveryStateV1::Complete)
        );
    }

    #[test]
    fn operation_format_recovers_pending_then_exact_completion() {
        let fixture = Fixture::new();
        let mut journal = LocalRequestJournalV1::default();
        let intent = operation_intent(7, [3; 16], b"stable-intent");
        journal
            .begin_operation(&fixture.0, intent, limits(2))
            .unwrap();
        let DurableOperationRecoveryV1::Recovered(pending) =
            journal.recover_operations(&fixture.0, scope(1), limits(2))
        else {
            panic!("pending intent must remain inspectable")
        };
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].operation, operation(7));
        assert_eq!(pending[0].frozen_request, b"stable-intent");
        assert_eq!(pending[0].frozen_reply, None);

        journal
            .complete_operation(
                &fixture.0,
                DurableOperationCompletionV1 {
                    intent,
                    frozen_reply: b"stable-receipt",
                },
                limits(2),
            )
            .unwrap();
        let DurableOperationRecoveryV1::Recovered(complete) =
            journal.recover_operations(&fixture.0, scope(1), limits(2))
        else {
            panic!("complete operation must recover")
        };
        assert_eq!(
            complete[0].frozen_reply.as_deref(),
            Some(&b"stable-receipt"[..])
        );
    }

    #[test]
    fn operation_identity_conflict_capacity_and_legacy_separation_are_closed() {
        let fixture = Fixture::new();
        let mut journal = LocalRequestJournalV1::default();
        let first = operation_intent(1, [1; 16], b"one");
        journal
            .begin_operation(&fixture.0, first, limits(1))
            .unwrap();
        assert_eq!(
            journal.begin_operation(
                &fixture.0,
                operation_intent(1, [1; 16], b"different"),
                limits(1),
            ),
            Err(RequestJournalRefusalV1::Conflict)
        );
        assert_eq!(
            journal.begin_operation(&fixture.0, operation_intent(2, [1; 16], b"two"), limits(1),),
            Err(RequestJournalRefusalV1::Full)
        );
        assert_eq!(
            journal.recover(&fixture.0, scope(1), limits(1)),
            RequestJournalRecoveryV1::Empty
        );
    }

    #[test]
    fn operation_recovery_accepts_older_generation_and_refuses_unsafe_entries() {
        let fixture = Fixture::new();
        let mut journal = LocalRequestJournalV1::default();
        journal
            .begin_operation(
                &fixture.0,
                operation_intent(1, [1; 16], b"intent"),
                limits(2),
            )
            .unwrap();
        assert!(matches!(
            journal.recover_operations(&fixture.0, scope(2), limits(2)),
            DurableOperationRecoveryV1::Recovered(_)
        ));
        let unsafe_entry = fixture.0.join(OPERATION_DIRECTORY_NAME).join("unexpected");
        fs::write(&unsafe_entry, b"content").unwrap();
        fs::set_permissions(&unsafe_entry, fs::Permissions::from_mode(0o600)).unwrap();
        assert_eq!(
            journal.recover_operations(&fixture.0, scope(2), limits(2)),
            DurableOperationRecoveryV1::Refused(RequestJournalRefusalV1::Malformed)
        );
    }

    #[test]
    fn legacy_operation_directory_requires_explicit_migration() {
        let fixture = Fixture::new();
        let legacy = fixture.0.join(LEGACY_OPERATION_DIRECTORY_NAME);
        fs::create_dir(&legacy).unwrap();
        fs::set_permissions(&legacy, fs::Permissions::from_mode(0o700)).unwrap();
        let mut journal = LocalRequestJournalV1::default();
        assert_eq!(
            journal.recover_operations(&fixture.0, scope(1), limits(2)),
            DurableOperationRecoveryV1::Refused(RequestJournalRefusalV1::Conflict)
        );
        assert_eq!(
            journal.begin_operation(
                &fixture.0,
                operation_intent(1, [1; 16], b"intent"),
                limits(2),
            ),
            Err(RequestJournalRefusalV1::Conflict)
        );
        assert!(!fixture.0.join(OPERATION_DIRECTORY_NAME).exists());
    }

    #[test]
    fn journal_descriptors_do_not_follow_legacy_or_operation_namespace_swaps() {
        for name in [DIRECTORY_NAME, OPERATION_DIRECTORY_NAME] {
            let fixture = Fixture::new();
            let journal = open_or_create_named_directory(&fixture.0, name).unwrap();
            let namespace = fixture.0.join(name);
            let relocated = fixture.0.join(format!("{name}.relocated"));
            fs::rename(&namespace, &relocated).unwrap();
            fs::create_dir(&namespace).unwrap();
            fs::set_permissions(&namespace, fs::Permissions::from_mode(0o700)).unwrap();

            journal.create_private_file("probe", b"anchored").unwrap();
            assert_eq!(journal.read_private_file("probe", 32).unwrap(), b"anchored");
            assert!(relocated.join("probe").is_file());
            assert!(!namespace.join("probe").exists());
            assert_eq!(journal.revalidate(), Err(RequestJournalRefusalV1::Conflict));
            assert_eq!(
                journal.synchronize_and_revalidate(),
                Err(RequestJournalRefusalV1::OutcomeUnknown)
            );
        }
    }

    #[test]
    fn retirement_is_absorbing_and_releases_only_active_capacity() {
        let fixture = Fixture::new();
        let mut journal = LocalRequestJournalV1::default();
        let intent = operation_intent(1, [4; 16], b"intent");
        journal
            .begin_operation(&fixture.0, intent, limits(1))
            .unwrap();
        let retirement = DurableOperationRetirementV1 {
            scope: scope(2),
            operation: operation(1),
            authenticated_subject: [4; 16],
            frozen_request: b"intent",
        };
        assert_eq!(
            journal.retire_operation(&fixture.0, retirement, limits(1)),
            Ok(DurableOperationRetirementResultV1::Pending)
        );
        journal
            .complete_operation(
                &fixture.0,
                DurableOperationCompletionV1 {
                    intent,
                    frozen_reply: b"receipt",
                },
                limits(1),
            )
            .unwrap();
        assert_eq!(
            journal.retire_operation(&fixture.0, retirement, limits(1)),
            Ok(DurableOperationRetirementResultV1::Retired)
        );
        assert_eq!(
            journal.retire_operation(&fixture.0, retirement, limits(1)),
            Ok(DurableOperationRetirementResultV1::AbsentOrForeign)
        );
        assert_eq!(
            journal.retire_operation(
                &fixture.0,
                DurableOperationRetirementV1 {
                    frozen_request: b"changed",
                    ..retirement
                },
                limits(1),
            ),
            Ok(DurableOperationRetirementResultV1::RequestConflict)
        );
        assert_eq!(
            journal.begin_operation(&fixture.0, intent, limits(1)),
            Err(RequestJournalRefusalV1::Conflict)
        );
        journal
            .begin_operation(&fixture.0, operation_intent(2, [4; 16], b"next"), limits(1))
            .unwrap();
        assert!(matches!(
            journal.recover_operation_retirements(&fixture.0, scope(2), limits(1)),
            DurableOperationRetirementRecoveryV1::Recovered(records) if records.len() == 1
        ));
    }

    #[test]
    fn committed_retirement_recovers_every_partial_cleanup_subset() {
        for (keep_intent, keep_reply) in
            [(true, true), (true, false), (false, true), (false, false)]
        {
            let fixture = Fixture::new();
            let subject = [8; 16];
            let intent = operation_intent(8, subject, b"stable");
            let mut journal = LocalRequestJournalV1::default();
            journal
                .begin_operation(&fixture.0, intent, limits(2))
                .unwrap();
            journal
                .complete_operation(
                    &fixture.0,
                    DurableOperationCompletionV1 {
                        intent,
                        frozen_reply: b"receipt",
                    },
                    limits(2),
                )
                .unwrap();
            let directory = fixture.0.join(OPERATION_DIRECTORY_NAME);
            let marker =
                encode_operation_record(OPERATION_RETIRED_MAGIC, intent, &[], limits(2)).unwrap();
            let marker_path = directory.join(operation_retired_file_name(subject, operation(8)));
            fs::write(&marker_path, marker).unwrap();
            fs::set_permissions(&marker_path, fs::Permissions::from_mode(0o600)).unwrap();
            if !keep_intent {
                fs::remove_file(directory.join(operation_file_name(subject, operation(8), true)))
                    .unwrap();
            }
            if !keep_reply {
                fs::remove_file(directory.join(operation_file_name(subject, operation(8), false)))
                    .unwrap();
            }
            assert_eq!(
                journal.recover_operations(&fixture.0, scope(2), limits(2)),
                DurableOperationRecoveryV1::Empty
            );
            assert!(marker_path.exists());
            assert!(
                !directory
                    .join(operation_file_name(subject, operation(8), true))
                    .exists()
            );
            assert!(
                !directory
                    .join(operation_file_name(subject, operation(8), false))
                    .exists()
            );
        }
    }

    #[test]
    fn tombstone_capacity_refuses_without_erasing_complete_active_receipt() {
        let fixture = Fixture::new();
        let mut bounded = limits(1);
        bounded.max_retired_operations = 1;
        let mut journal = LocalRequestJournalV1::default();
        for value in [1, 2] {
            let request = [value];
            let reply = [value, value];
            let intent = operation_intent(value, [2; 16], &request);
            journal
                .begin_operation(&fixture.0, intent, bounded)
                .unwrap();
            journal
                .complete_operation(
                    &fixture.0,
                    DurableOperationCompletionV1 {
                        intent,
                        frozen_reply: &reply,
                    },
                    bounded,
                )
                .unwrap();
            let result = journal.retire_operation(
                &fixture.0,
                DurableOperationRetirementV1 {
                    scope: scope(1),
                    operation: operation(value),
                    authenticated_subject: [2; 16],
                    frozen_request: &request,
                },
                bounded,
            );
            if value == 1 {
                assert_eq!(result, Ok(DurableOperationRetirementResultV1::Retired));
            } else {
                assert_eq!(result, Err(RequestJournalRefusalV1::Full));
            }
        }
        let DurableOperationRecoveryV1::Recovered(active) =
            journal.recover_operations(&fixture.0, scope(1), bounded)
        else {
            panic!("second complete receipt must remain")
        };
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].operation, operation(2));
        assert_eq!(active[0].frozen_reply.as_deref(), Some(&[2, 2][..]));
    }

    #[test]
    fn retirement_replay_refuses_future_generation_tombstone() {
        let fixture = Fixture::new();
        let subject = [9; 16];
        let intent = DurableOperationIntentV1 {
            scope: scope(2),
            operation: operation(9),
            authenticated_subject: subject,
            frozen_request: b"intent",
        };
        let mut journal = LocalRequestJournalV1::default();
        journal
            .begin_operation(&fixture.0, intent, limits(2))
            .unwrap();
        journal
            .complete_operation(
                &fixture.0,
                DurableOperationCompletionV1 {
                    intent,
                    frozen_reply: b"receipt",
                },
                limits(2),
            )
            .unwrap();
        assert_eq!(
            journal.retire_operation(
                &fixture.0,
                DurableOperationRetirementV1 {
                    scope: scope(2),
                    operation: operation(9),
                    authenticated_subject: subject,
                    frozen_request: b"intent",
                },
                limits(2),
            ),
            Ok(DurableOperationRetirementResultV1::Retired)
        );
        assert_eq!(
            journal.retire_operation(
                &fixture.0,
                DurableOperationRetirementV1 {
                    scope: scope(1),
                    operation: operation(9),
                    authenticated_subject: subject,
                    frozen_request: b"intent",
                },
                limits(2),
            ),
            Err(RequestJournalRefusalV1::Conflict)
        );
    }

    #[test]
    fn conflicting_post_marker_reply_fences_before_any_active_unlink() {
        let fixture = Fixture::new();
        let subject = [6; 16];
        let intent = operation_intent(6, subject, b"stable");
        let mut journal = LocalRequestJournalV1::default();
        journal
            .begin_operation(&fixture.0, intent, limits(2))
            .unwrap();
        journal
            .complete_operation(
                &fixture.0,
                DurableOperationCompletionV1 {
                    intent,
                    frozen_reply: b"receipt",
                },
                limits(2),
            )
            .unwrap();
        let directory = fixture.0.join(OPERATION_DIRECTORY_NAME);
        let marker =
            encode_operation_record(OPERATION_RETIRED_MAGIC, intent, &[], limits(2)).unwrap();
        let marker_path = directory.join(operation_retired_file_name(subject, operation(6)));
        fs::write(&marker_path, marker).unwrap();
        fs::set_permissions(&marker_path, fs::Permissions::from_mode(0o600)).unwrap();
        let reply_path = directory.join(operation_file_name(subject, operation(6), false));
        let mut corrupt = fs::read(&reply_path).unwrap();
        corrupt[OPERATION_HEADER_BYTES] ^= 1;
        fs::write(&reply_path, corrupt).unwrap();
        assert!(matches!(
            journal.recover_operations(&fixture.0, scope(1), limits(2)),
            DurableOperationRecoveryV1::Refused(_)
        ));
        assert!(
            directory
                .join(operation_file_name(subject, operation(6), true))
                .exists()
        );
        assert!(reply_path.exists());
    }
}

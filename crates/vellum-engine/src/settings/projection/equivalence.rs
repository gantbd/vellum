//! Bounded structural equality independent of native indices and map enumeration.
use super::*;
fn step(work: &mut usize, n: usize) -> Result<(), SettingsError> {
    *work = work.checked_sub(n).ok_or(SettingsError::Limit)?;
    Ok(())
}
fn label(a: &str, b: &str, work: &mut usize) -> Result<bool, SettingsError> {
    step(
        work,
        a.len()
            .checked_add(b.len())
            .and_then(|n| n.checked_add(1))
            .ok_or(SettingsError::Limit)?,
    )?;
    Ok(a == b)
}
fn space(
    stack: &mut Vec<(usize, usize)>,
    count: usize,
    bytes: usize,
    work: &mut usize,
) -> Result<(), SettingsError> {
    step(work, count)?;
    if stack
        .len()
        .checked_add(count)
        .and_then(|n| n.checked_mul(size_of::<(usize, usize)>()))
        .is_none_or(|n| n > bytes)
    {
        return Err(SettingsError::Limit);
    }
    stack
        .try_reserve(count)
        .map_err(|_| SettingsError::Capacity)
}
fn values(
    a: &[Node],
    ar: usize,
    b: &[Node],
    br: usize,
    work: &mut usize,
    bytes: usize,
) -> Result<bool, SettingsError> {
    let mut stack = Vec::new();
    space(&mut stack, 1, bytes, work)?;
    stack.push((ar, br));
    while let Some((ar, br)) = stack.pop() {
        step(work, 1)?;
        match (&a[ar], &b[br]) {
            (Node::Null, Node::Null) => {}
            (Node::Boolean(a), Node::Boolean(b)) if a == b => {}
            (Node::Number(a), Node::Number(b)) if a == b => {}
            (Node::Text(a), Node::Text(b)) if label(a, b, work)? => {}
            (Node::List(a), Node::List(b)) if a.len() == b.len() => {
                space(&mut stack, a.len(), bytes, work)?;
                stack.extend(a.iter().copied().zip(b.iter().copied()));
            }
            (Node::Map(a), Node::Map(b)) if a.len() == b.len() => {
                space(&mut stack, a.len(), bytes, work)?;
                for (key, child) in a {
                    let mut found = None;
                    for (other, child) in b {
                        if label(key, other, work)? {
                            found = Some(*child);
                            break;
                        }
                    }
                    let Some(other) = found else {
                        return Ok(false);
                    };
                    stack.push((*child, other));
                }
            }
            _ => return Ok(false),
        }
    }
    Ok(true)
}
fn types(
    a: &SettingProjectionTypeV1,
    b: &SettingProjectionTypeV1,
    work: &mut usize,
    bytes: usize,
) -> Result<bool, SettingsError> {
    let mut stack = Vec::new();
    space(&mut stack, 1, bytes, work)?;
    stack.push((a.root, b.root));
    while let Some((ar, br)) = stack.pop() {
        step(work, 1)?;
        match (&a.types[ar], &b.types[br]) {
            (TypeNode::Boolean, TypeNode::Boolean) | (TypeNode::Text, TypeNode::Text) => {}
            (TypeNode::Integer(a, b), TypeNode::Integer(c, d)) if a == c && b == d => {}
            (TypeNode::Enum(x), TypeNode::Enum(y)) if x.len() == y.len() => {
                step(work, x.len())?;
                let available = bytes
                    .checked_sub(
                        stack
                            .len()
                            .checked_mul(size_of::<(usize, usize)>())
                            .ok_or(SettingsError::Limit)?,
                    )
                    .ok_or(SettingsError::Limit)?;
                for (x, y) in x.iter().zip(y) {
                    if !values(&a.values, *x, &b.values, *y, work, available)? {
                        return Ok(false);
                    }
                }
            }
            (TypeNode::List(a), TypeNode::List(b)) => {
                space(&mut stack, 1, bytes, work)?;
                stack.push((*a, *b));
            }
            (TypeNode::Map(x), TypeNode::Map(y)) if x.len() == y.len() => {
                space(&mut stack, x.len(), bytes, work)?;
                for (key, child) in x {
                    let mut found = None;
                    for (other, child) in y {
                        if label(key, other, work)? {
                            found = Some(*child);
                            break;
                        }
                    }
                    let Some(other) = found else {
                        return Ok(false);
                    };
                    stack.push((*child, other));
                }
            }
            _ => return Ok(false),
        }
    }
    Ok(true)
}
fn origins(
    a: &SettingProjectionOriginsV1,
    b: &SettingProjectionOriginsV1,
    work: &mut usize,
) -> Result<bool, SettingsError> {
    // Identity-list equality is charged by the enclosing copied-byte envelope.
    match (a, b) {
        (SettingProjectionOriginsV1::Whole(a), SettingProjectionOriginsV1::Whole(b)) => Ok(a == b),
        (SettingProjectionOriginsV1::Fields(a), SettingProjectionOriginsV1::Fields(b))
            if a.len() == b.len() =>
        {
            for (key, ids) in a {
                let mut found = None;
                for (other, ids) in b {
                    if label(key, other, work)? {
                        found = Some(ids);
                        break;
                    }
                }
                if found != Some(ids) {
                    return Ok(false);
                }
            }
            Ok(true)
        }
        _ => Ok(false),
    }
}
impl NativeSettingsProjectionV1 {
    pub(crate) fn same_recipient_payload(
        &self,
        other: &Self,
        work: &mut usize,
        scratch_bytes: usize,
    ) -> Result<bool, SettingsError> {
        if self.capabilities != other.capabilities || self.rows.len() != other.rows.len() {
            return Ok(false);
        }
        step(
            work,
            self.logical_bytes
                .checked_add(other.logical_bytes)
                .ok_or(SettingsError::Limit)?,
        )?;
        for (a, b) in self.rows.iter().zip(&other.rows) {
            if !label(&a.key, &b.key, work)? {
                return Ok(false);
            }
            match (&a.value, &b.value) {
                (Some(a), Some(b)) => {
                    if !values(&a.nodes, a.root, &b.nodes, b.root, work, scratch_bytes)? {
                        return Ok(false);
                    }
                }
                (None, None) => {}
                _ => return Ok(false),
            }
            match (&a.origins, &b.origins) {
                (Some(a), Some(b)) => {
                    if !origins(a, b, work)? {
                        return Ok(false);
                    }
                }
                (None, None) => {}
                _ => return Ok(false),
            }
            match (&a.declaration, &b.declaration) {
                (Some(a), Some(b)) => {
                    if !label(&a.key, &b.key, work)?
                        || a.domain != b.domain
                        || a.scopes != b.scopes
                        || a.composition != b.composition
                        || a.sensitive != b.sensitive
                        || !types(&a.value_type, &b.value_type, work, scratch_bytes)?
                    {
                        return Ok(false);
                    }
                }
                (None, None) => {}
                _ => return Ok(false),
            }
        }
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn projection(reverse: bool, changed: bool) -> NativeSettingsProjectionV1 {
        let (fields, nodes) = if reverse {
            (
                vec![("b".into(), 1), ("a".into(), 2)],
                vec![Node::Null, Node::Text("b".into()), Node::Boolean(!changed)],
            )
        } else {
            (
                vec![("a".into(), 1), ("b".into(), 2)],
                vec![Node::Null, Node::Boolean(true), Node::Text("b".into())],
            )
        };
        let mut nodes = nodes;
        nodes[0] = Node::Map(fields);
        let declaration = SettingProjectionDeclarationV1 {
            key: "map".into(),
            value_type: SettingProjectionTypeV1 {
                types: if reverse {
                    vec![
                        TypeNode::Map(vec![("b".into(), 1), ("a".into(), 2)]),
                        TypeNode::Text,
                        TypeNode::Boolean,
                    ]
                } else {
                    vec![
                        TypeNode::Map(vec![("a".into(), 1), ("b".into(), 2)]),
                        TypeNode::Boolean,
                        TypeNode::Text,
                    ]
                },
                values: vec![],
                root: 0,
            },
            domain: SettingDomainV1::Analysis,
            scopes: vec![SettingScopeV1::Host],
            composition: SettingCompositionV1::MergeMap,
            sensitive: true,
        };
        let origins = if reverse {
            vec![
                ("b".into(), vec![]),
                ("a".into(), vec![SettingProjectionOriginV1::Library]),
            ]
        } else {
            vec![
                ("a".into(), vec![SettingProjectionOriginV1::Library]),
                ("b".into(), vec![]),
            ]
        };
        NativeSettingsProjectionV1 {
            rows: vec![SettingProjectionRowV1 {
                key: "map".into(),
                value: Some(SettingProjectionValueV1 { nodes, root: 0 }),
                origins: Some(SettingProjectionOriginsV1::Fields(origins)),
                declaration: Some(declaration),
            }],
            capabilities: true,
            logical_bytes: if reverse { 2048 } else { 1024 },
        }
    }
    #[test]
    fn equality_ignores_map_enumeration_flat_indices_and_construction_charges_not_payload_changes()
    {
        let a = projection(false, false);
        let b = projection(true, false);
        assert!(a.same_recipient_payload(&b, &mut 65536, 65536).unwrap());
        let changed = projection(true, true);
        assert!(
            !a.same_recipient_payload(&changed, &mut 65536, 65536)
                .unwrap()
        );
        assert_eq!(
            a.same_recipient_payload(&b, &mut 0, 65536),
            Err(SettingsError::Limit)
        );
        assert_eq!(
            a.same_recipient_payload(&b, &mut 65536, 0),
            Err(SettingsError::Limit)
        );
    }
    #[test]
    fn deep_semantic_comparison_and_drop_do_not_use_the_call_stack() {
        std::thread::Builder::new()
            .stack_size(64 * 1024)
            .spawn(|| {
                let make = || {
                    let mut nodes = vec![Node::Boolean(true)];
                    for i in 0..4000 {
                        nodes.push(Node::List(vec![i]));
                    }
                    NativeSettingsProjectionV1 {
                        rows: vec![SettingProjectionRowV1 {
                            key: "deep".into(),
                            value: Some(SettingProjectionValueV1 { nodes, root: 4000 }),
                            origins: None,
                            declaration: None,
                        }],
                        capabilities: false,
                        logical_bytes: 400_000,
                    }
                };
                let a = make();
                let b = make();
                assert!(
                    a.same_recipient_payload(&b, &mut 2_000_000, 400_000)
                        .unwrap()
                );
                drop((a, b));
            })
            .unwrap()
            .join()
            .unwrap();
    }
}

use super::compile::{Meter, reserve};
use super::*;
pub(super) fn equal(
    s: &CompiledSettingsV1,
    a: usize,
    b: usize,
    m: &mut Meter,
    exact_numbers: bool,
) -> Result<bool, SettingsError> {
    equal_between(s, a, s, b, m, exact_numbers)
}
pub(super) fn equal_between(
    sa: &CompiledSettingsV1,
    a: usize,
    sb: &CompiledSettingsV1,
    b: usize,
    m: &mut Meter,
    exact_numbers: bool,
) -> Result<bool, SettingsError> {
    let mut work = Vec::new();
    reserve(&mut work, 1)?;
    work.push((a, b));
    while let Some((a, b)) = work.pop() {
        m.tick()?;
        if std::ptr::eq(sa, sb) && a == b {
            continue;
        }
        match (&sa.nodes[a], &sb.nodes[b]) {
            (Node::Null, Node::Null) => {}
            (Node::Boolean(a), Node::Boolean(b)) if a == b => {}
            (Node::Text(a), Node::Text(b)) if a == b => {}
            (Node::Number(a), Node::Number(b))
                if if exact_numbers {
                    a == b
                } else {
                    f64::from_bits(*a) == f64::from_bits(*b)
                } => {}
            (Node::List(a), Node::List(b)) if a.len() == b.len() => {
                m.steps(a.len())?;
                reserve(&mut work, a.len())?;
                work.extend(a.iter().zip(b).map(|(a, b)| (*a, *b)));
            }
            (Node::Map(a), Node::Map(b)) if a.len() == b.len() => {
                m.steps(a.len())?;
                reserve(&mut work, a.len())?;
                for (key, a) in a {
                    let mut found = None;
                    for (other, b) in b {
                        m.tick()?;
                        if key == other {
                            found = Some(*b);
                            break;
                        }
                    }
                    let Some(b) = found else { return Ok(false) };
                    work.push((*a, b));
                }
            }
            _ => return Ok(false),
        }
    }
    Ok(true)
}
pub(super) fn typed(
    s: &CompiledSettingsV1,
    t: usize,
    v: usize,
    partial: bool,
    m: &mut Meter,
) -> Result<(), SettingsError> {
    let mut work = Vec::new();
    reserve(&mut work, 1)?;
    work.push((t, v, partial));
    while let Some((t, v, partial)) = work.pop() {
        m.tick()?;
        match (&s.types[t], &s.nodes[v]) {
            (TypeNode::Boolean, Node::Boolean(_)) | (TypeNode::Text, Node::Text(_)) => {}
            (TypeNode::Integer(min, max), Node::Number(bits)) => {
                let n = f64::from_bits(*bits);
                if n.fract() != 0.0 || n < *min as f64 || n > *max as f64 {
                    return Err(SettingsError::InvalidValue);
                }
            }
            (TypeNode::Enum(values), _) => {
                let mut found = false;
                for allowed in values {
                    if equal(s, *allowed, v, m, false)? {
                        found = true;
                        break;
                    }
                }
                if !found {
                    return Err(SettingsError::InvalidValue);
                }
            }
            (TypeNode::List(element), Node::List(values)) => {
                m.steps(values.len())?;
                reserve(&mut work, values.len())?;
                work.extend(values.iter().map(|v| (*element, *v, false)));
            }
            (TypeNode::Map(fields), Node::Map(values)) => {
                if !partial && fields.len() != values.len() {
                    return Err(SettingsError::InvalidValue);
                }
                m.steps(values.len())?;
                reserve(&mut work, values.len())?;
                for (key, value) in values {
                    let mut found = None;
                    for (field, t) in fields {
                        m.tick()?;
                        if key == field {
                            found = Some(*t);
                            break;
                        }
                    }
                    work.push((found.ok_or(SettingsError::InvalidValue)?, *value, false));
                }
            }
            _ => return Err(SettingsError::InvalidValue),
        }
    }
    Ok(())
}
#[derive(Clone, Copy)]
struct Row {
    rank: i8,
    value: usize,
    origin: Option<usize>,
}
fn select(
    s: &CompiledSettingsV1,
    rows: &[Row],
    m: &mut Meter,
) -> Result<(usize, Vec<Option<usize>>), SettingsError> {
    if rows.is_empty() {
        return Err(SettingsError::Faulted);
    }
    let mut rank = -1i8;
    let mut top = rows[0].value;
    for (i, row) in rows.iter().enumerate() {
        m.tick()?;
        for peer in &rows[..i] {
            m.tick()?;
            if peer.rank == row.rank && !equal(s, peer.value, row.value, m, false)? {
                return Err(SettingsError::Conflict);
            }
        }
        if row.rank > rank {
            rank = row.rank;
            top = row.value;
        }
    }
    let count = rows.iter().filter(|r| r.rank == rank).count();
    m.edges::<Option<usize>>(count)?;
    let mut origins = Vec::new();
    reserve(&mut origins, count)?;
    for row in rows {
        m.tick()?;
        if row.rank == rank {
            origins.push(row.origin);
        }
    }
    for i in 1..origins.len() {
        let mut j = i;
        while j > 0 {
            m.tick()?;
            let key = |index: Option<usize>| {
                index.map(|i| {
                    let id = &s.layers[i].identity;
                    (&id.authority, &id.id, id.incarnation)
                })
            };
            if key(origins[j - 1]) <= key(origins[j]) {
                break;
            }
            origins.swap(j - 1, j);
            j -= 1;
        }
    }
    Ok((top, origins))
}
enum CandidateValue {
    Existing(usize),
    Map(Vec<(String, usize)>),
}
struct Candidate {
    value: CandidateValue,
    origins: Origins,
}
fn resolve_one(
    s: &CompiledSettingsV1,
    key: usize,
    layers: &[usize],
    m: &mut Meter,
) -> Result<Candidate, SettingsError> {
    let schema = &s.schemas[key];
    let mut rows = Vec::new();
    reserve(
        &mut rows,
        layers.len().checked_add(1).ok_or(SettingsError::Limit)?,
    )?;
    rows.push(Row {
        rank: -1,
        value: schema.default,
        origin: None,
    });
    for layer in layers {
        m.tick()?;
        let data = &s.layers[*layer];
        for (setting, contribution) in &data.values {
            m.tick()?;
            if *setting != key {
                continue;
            }
            let value = match contribution {
                Contribution::Reset => schema.default,
                Contribution::Set(value) => *value,
            };
            rows.push(Row {
                rank: data.scope as i8,
                value,
                origin: Some(*layer),
            });
        }
    }
    if schema.composition == SettingCompositionV1::Replace {
        let (value, origins) = select(s, &rows, m)?;
        return Ok(Candidate {
            value: CandidateValue::Existing(value),
            origins: Origins::Whole(origins),
        });
    }
    let TypeNode::Map(fields) = &s.types[schema.value_type] else {
        return Err(SettingsError::Faulted);
    };
    m.edges::<(String, usize)>(fields.len())?;
    m.edges::<(String, Vec<Option<usize>>)>(fields.len())?;
    let mut values = Vec::new();
    let mut origins = Vec::new();
    reserve(&mut values, fields.len())?;
    reserve(&mut origins, fields.len())?;
    for (field, _) in fields {
        let mut field_rows = Vec::new();
        reserve(&mut field_rows, rows.len())?;
        for row in &rows {
            m.tick()?;
            let Node::Map(values) = &s.nodes[row.value] else {
                return Err(SettingsError::Faulted);
            };
            for (key, value) in values {
                m.tick()?;
                if key == field {
                    field_rows.push(Row {
                        value: *value,
                        ..*row
                    });
                    break;
                }
            }
        }
        let (value, origin) = select(s, &field_rows, m)?;
        values.push((m.identifier(field)?, value));
        origins.push((m.identifier(field)?, origin));
    }
    Ok(Candidate {
        value: CandidateValue::Map(values),
        origins: Origins::Fields(origins),
    })
}
pub(super) fn resolve_all(s: &mut CompiledSettingsV1, m: &mut Meter) -> Result<(), SettingsError> {
    for target in 0..s.targets.len() {
        m.edges::<Resolved>(s.schemas.len())?;
        let mut resolved = Vec::new();
        reserve(&mut resolved, s.schemas.len())?;
        for schema in 0..s.schemas.len() {
            m.tick()?;
            let Candidate { value, origins } =
                resolve_one(s, schema, &s.targets[target].layers, m)?;
            let value = match value {
                CandidateValue::Existing(value) => value,
                CandidateValue::Map(fields) => {
                    if s.nodes.len() >= m.limits.max_value_nodes {
                        return Err(SettingsError::Limit);
                    }
                    m.bytes(std::mem::size_of::<Node>())?;
                    reserve(&mut s.nodes, 1)?;
                    let value = s.nodes.len();
                    s.nodes.push(Node::Map(fields));
                    value
                }
            };
            typed(s, s.schemas[schema].value_type, value, false, m)?;
            for (key, allowed) in &s.restrictions {
                m.tick()?;
                if *key == schema {
                    let mut found = false;
                    for value_allowed in allowed {
                        if equal(s, *value_allowed, value, m, false)? {
                            found = true;
                            break;
                        }
                    }
                    if !found {
                        return Err(SettingsError::RestrictionDenied);
                    }
                }
            }
            resolved.push(Resolved { value, origins });
        }
        s.targets[target].resolved = resolved;
    }
    Ok(())
}

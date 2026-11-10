//! Closed, iterative schema conversion. Native legality remains the compiler's job.
use super::*;
fn integer(v: &C) -> Result<i64> {
    let C::Number(n) = v else { return bad() };
    if !n.is_finite() || n.fract() != 0.0 || n.abs() > 9_007_199_254_740_991.0 {
        return bad();
    }
    Ok(*n as i64)
}
fn children(c: &InertCaptureV1, i: usize, todo: &mut Vec<usize>) -> Result<()> {
    let t = node(c, i)?;
    match text(field(c, t, "kind")?)? {
        "list" => todo.push(index(t, "element")?),
        "map" => {
            for (_, n) in object(field(c, t, "fields")?)? {
                todo.push(*n);
            }
        }
        _ => {}
    }
    Ok(())
}
pub(super) fn cost(c: &InertCaptureV1, start: usize, limit: usize) -> Result<usize> {
    let mut todo = Vec::new();
    reserve(&mut todo, c.usage().nodes)?;
    todo.push(start);
    let mut cost = 0;
    while let Some(i) = todo.pop() {
        let t = node(c, i)?;
        cost = add(cost, size_of::<SettingTypeV1>())?;
        match text(field(c, t, "kind")?)? {
            "boolean" | "text" => exact(t, &["kind"])?,
            "integer" => {
                exact(t, &["kind", "min", "max"])?;
                if integer(field(c, t, "min")?)? > integer(field(c, t, "max")?)? {
                    return bad();
                }
            }
            "enum" => {
                exact(t, &["kind", "values"])?;
                let rows = array(field(c, t, "values")?)?;
                if rows.is_empty() {
                    return bad();
                }
                for n in rows {
                    cost = add(cost, value_cost(c, *n)?)?;
                }
            }
            "list" => exact(t, &["kind", "element"])?,
            "map" => {
                exact(t, &["kind", "fields"])?;
                let rows = object(field(c, t, "fields")?)?;
                cost = add(cost, mul(rows.len(), size_of::<String>())?)?;
                for (k, _) in rows {
                    if k.is_empty() || k.len() > limit {
                        return bad();
                    }
                    cost = add(cost, k.len())?;
                }
            }
            _ => return bad(),
        }
        children(c, i, &mut todo)?;
    }
    Ok(cost)
}
pub(super) fn build(c: &InertCaptureV1, start: usize) -> Result<SettingTypeV1> {
    let mut todo = Vec::new();
    let mut order = Vec::new();
    let mut out = Vec::new();
    reserve(&mut todo, c.usage().nodes)?;
    reserve(&mut order, c.usage().nodes)?;
    reserve(&mut out, c.usage().nodes)?;
    out.resize_with(c.usage().nodes, || None);
    todo.push(start);
    while let Some(i) = todo.pop() {
        order.push(i);
        children(c, i, &mut todo)?;
    }
    for i in order.into_iter().rev() {
        let t = node(c, i)?;
        let ty = match text(field(c, t, "kind")?)? {
            "boolean" => SettingTypeV1::Boolean,
            "text" => SettingTypeV1::Text,
            "integer" => SettingTypeV1::Integer {
                min: integer(field(c, t, "min")?)?,
                max: integer(field(c, t, "max")?)?,
            },
            "enum" => {
                let rows = array(field(c, t, "values")?)?;
                let mut values = Vec::new();
                reserve(&mut values, rows.len())?;
                for n in rows {
                    values.push(value(c, *n)?);
                }
                SettingTypeV1::Enum(values)
            }
            "list" => SettingTypeV1::List(Box::new(
                out[index(t, "element")?]
                    .take()
                    .ok_or(InvocationError::Faulted)?,
            )),
            "map" => {
                let rows = object(field(c, t, "fields")?)?;
                let mut fields = Vec::new();
                reserve(&mut fields, rows.len())?;
                for (k, n) in rows {
                    fields.push((copy(k)?, out[*n].take().ok_or(InvocationError::Faulted)?));
                }
                SettingTypeV1::Map(fields)
            }
            _ => return Err(InvocationError::Faulted),
        };
        out[i] = Some(ty);
    }
    out[start].take().ok_or(InvocationError::Faulted)
}

//! Finite inert native-data copying for northbound normalization.
//!
//! Already-decoded Rust values only: no parser, callbacks, opaque handles or wire
//! identity codec. Limits measure native logical output plus traversal workspace,
//! not JSON bytes, allocator capacity, caller input, RSS or global headroom.
use crate::instrumentation::{Boundary, Disposition, Observation, Recorder};
use std::mem::size_of;

#[derive(Clone, Copy)]
pub enum InertValueV1<'a> {
    Null,
    Boolean(bool),
    Number(f64),
    Text(&'a str),
    Array(&'a [InertValueV1<'a>]),
    Object(&'a [(&'a str, InertValueV1<'a>)]),
}
/// Children are indices in the owning immutable capture, not capabilities.
pub enum CapturedValueV1 {
    Null,
    Boolean(bool),
    Number(f64),
    Text(String),
    Array(Vec<usize>),
    Object(Vec<(String, usize)>),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CaptureLimitsV1 {
    pub max_bytes: usize,
    pub max_nodes: usize,
    /// Root depth is one, including a scalar root.
    pub max_depth: usize,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CaptureErrorV1 {
    Limit,
    Capacity,
    NonfiniteNumber,
    DuplicateField,
    Cycle,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CaptureUsageV1 {
    pub retained_bytes: usize,
    /// Output plus live traversal frames; excludes caller-owned input.
    pub peak_bytes: usize,
    pub nodes: usize,
    pub depth: usize,
}
pub struct InertCaptureV1 {
    nodes: Vec<CapturedValueV1>,
    usage: CaptureUsageV1,
}
macro_rules! redacted {
    ($name:ty) => {
        impl std::fmt::Debug for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str("<inert-data:redacted>")
            }
        }
    };
}
redacted!(InertValueV1<'_>);
redacted!(CapturedValueV1);
redacted!(InertCaptureV1);
type Result<T> = std::result::Result<T, CaptureErrorV1>;
struct Frame<'a> {
    input: &'a InertValueV1<'a>,
    node: usize,
    next: usize,
    depth: usize,
}
struct Meter {
    limits: CaptureLimitsV1,
    usage: CaptureUsageV1,
    live_frames: usize,
}
impl Meter {
    fn check(&mut self) -> Result<()> {
        let bytes = self
            .live_frames
            .checked_mul(size_of::<Frame<'_>>())
            .and_then(|n| n.checked_add(self.usage.retained_bytes))
            .ok_or(CaptureErrorV1::Limit)?;
        if bytes > self.limits.max_bytes {
            return Err(CaptureErrorV1::Limit);
        }
        self.usage.peak_bytes = self.usage.peak_bytes.max(bytes);
        Ok(())
    }
    fn bytes(&mut self, n: usize) -> Result<()> {
        self.usage.retained_bytes = self
            .usage
            .retained_bytes
            .checked_add(n)
            .ok_or(CaptureErrorV1::Limit)?;
        self.check()
    }
    fn node(&mut self, depth: usize) -> Result<()> {
        self.usage.nodes = self
            .usage
            .nodes
            .checked_add(1)
            .ok_or(CaptureErrorV1::Limit)?;
        if self.usage.nodes > self.limits.max_nodes || depth > self.limits.max_depth {
            return Err(CaptureErrorV1::Limit);
        }
        self.usage.depth = self.usage.depth.max(depth);
        self.bytes(size_of::<CapturedValueV1>())
    }
    fn children(&self, n: usize) -> Result<()> {
        if n > self.limits.max_nodes - self.usage.nodes {
            Err(CaptureErrorV1::Limit)
        } else {
            Ok(())
        }
    }
}
fn reserve<T>(values: &mut Vec<T>, n: usize) -> Result<()> {
    values
        .try_reserve_exact(n)
        .map_err(|_| CaptureErrorV1::Capacity)
}
fn text(input: &str) -> Result<String> {
    let mut result = String::new();
    result
        .try_reserve_exact(input.len())
        .map_err(|_| CaptureErrorV1::Capacity)?;
    result.push_str(input);
    Ok(result)
}
fn enter<'a>(
    input: &'a InertValueV1<'a>,
    depth: usize,
    nodes: &mut Vec<CapturedValueV1>,
    stack: &mut Vec<Frame<'a>>,
    meter: &mut Meter,
) -> Result<usize> {
    if stack.iter().any(|frame| std::ptr::eq(frame.input, input)) {
        return Err(CaptureErrorV1::Cycle);
    }
    meter.node(depth)?;
    let value = match input {
        InertValueV1::Null => CapturedValueV1::Null,
        InertValueV1::Boolean(v) => CapturedValueV1::Boolean(*v),
        InertValueV1::Number(v) => {
            if !v.is_finite() {
                return Err(CaptureErrorV1::NonfiniteNumber);
            }
            CapturedValueV1::Number(*v)
        }
        InertValueV1::Text(v) => {
            meter.bytes(v.len())?;
            CapturedValueV1::Text(text(v)?)
        }
        InertValueV1::Array(v) => {
            meter.children(v.len())?;
            meter.bytes(
                v.len()
                    .checked_mul(size_of::<usize>())
                    .ok_or(CaptureErrorV1::Limit)?,
            )?;
            let mut children = Vec::new();
            reserve(&mut children, v.len())?;
            CapturedValueV1::Array(children)
        }
        InertValueV1::Object(v) => {
            meter.children(v.len())?;
            meter.bytes(
                v.len()
                    .checked_mul(size_of::<(String, usize)>())
                    .ok_or(CaptureErrorV1::Limit)?,
            )?;
            for (i, (key, _)) in v.iter().enumerate() {
                meter.bytes(key.len())?;
                if v[..i].iter().any(|(prior, _)| prior == key) {
                    return Err(CaptureErrorV1::DuplicateField);
                }
            }
            let mut children = Vec::new();
            reserve(&mut children, v.len())?;
            CapturedValueV1::Object(children)
        }
    };
    let index = nodes.len();
    reserve(nodes, 1)?;
    nodes.push(value);
    if matches!(input, InertValueV1::Array(_) | InertValueV1::Object(_)) {
        meter.live_frames = meter
            .live_frames
            .checked_add(1)
            .ok_or(CaptureErrorV1::Limit)?;
        meter.check()?;
        reserve(stack, 1)?;
        stack.push(Frame {
            input,
            node: index,
            next: 0,
            depth,
        });
    }
    Ok(index)
}
impl InertCaptureV1 {
    pub fn usage(&self) -> CaptureUsageV1 {
        self.usage
    }
    pub fn root(&self) -> &CapturedValueV1 {
        &self.nodes[0]
    }
    pub fn node(&self, index: usize) -> Option<&CapturedValueV1> {
        self.nodes.get(index)
    }
    pub fn copy(
        input: &InertValueV1<'_>,
        limits: CaptureLimitsV1,
        recorder: &mut Recorder,
    ) -> Result<Self> {
        let result = Self::capture(input, limits);
        recorder.record(Observation {
            boundary: Boundary::InertDataCapture,
            disposition: if result.is_ok() {
                Disposition::Completed
            } else {
                Disposition::Refused
            },
            service_time: None,
        });
        result
    }
    fn capture(input: &InertValueV1<'_>, limits: CaptureLimitsV1) -> Result<Self> {
        let mut meter = Meter {
            limits,
            usage: CaptureUsageV1 {
                retained_bytes: size_of::<Self>() + size_of::<Vec<Frame<'_>>>(),
                peak_bytes: 0,
                nodes: 0,
                depth: 0,
            },
            live_frames: 0,
        };
        meter.check()?;
        let mut nodes = Vec::new();
        let mut stack = Vec::new();
        enter(input, 1, &mut nodes, &mut stack, &mut meter)?;
        while let Some(frame) = stack.last_mut() {
            let child = match frame.input {
                InertValueV1::Array(v) => v.get(frame.next).map(|v| (None, v)),
                InertValueV1::Object(v) => v.get(frame.next).map(|(k, v)| (Some(*k), v)),
                _ => unreachable!("only containers enter the traversal stack"),
            };
            if let Some((key, child)) = child {
                let parent = frame.node;
                let depth = frame.depth.checked_add(1).ok_or(CaptureErrorV1::Limit)?;
                frame.next += 1;
                let key = key.map(text).transpose()?;
                let index = enter(child, depth, &mut nodes, &mut stack, &mut meter)?;
                match &mut nodes[parent] {
                    CapturedValueV1::Array(v) => v.push(index),
                    CapturedValueV1::Object(v) => v.push((key.expect("object key"), index)),
                    _ => unreachable!("only containers have child edges"),
                }
            } else {
                stack.pop();
                meter.live_frames -= 1;
            }
        }
        // Traversal's Vec header is temporary, not part of retained output.
        meter.usage.retained_bytes -= size_of::<Vec<Frame<'_>>>();
        Ok(Self {
            nodes,
            usage: meter.usage,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn meter() -> Meter {
        Meter {
            limits: CaptureLimitsV1 {
                max_bytes: usize::MAX,
                max_nodes: usize::MAX,
                max_depth: usize::MAX,
            },
            usage: CaptureUsageV1 {
                retained_bytes: 0,
                peak_bytes: 0,
                nodes: 0,
                depth: 0,
            },
            live_frames: 0,
        }
    }
    #[test]
    fn native_counter_overflow_refuses_without_wrapping_or_panicking() {
        let mut m = meter();
        m.usage.retained_bytes = usize::MAX;
        assert_eq!(m.bytes(1), Err(CaptureErrorV1::Limit));
        let mut m = meter();
        m.live_frames = usize::MAX;
        assert_eq!(m.check(), Err(CaptureErrorV1::Limit));
        let mut m = meter();
        m.usage.nodes = usize::MAX;
        assert_eq!(m.node(1), Err(CaptureErrorV1::Limit));
    }
    #[test]
    fn impossible_allocation_capacity_maps_to_resource_refusal() {
        let mut v = Vec::<u8>::new();
        assert_eq!(reserve(&mut v, usize::MAX), Err(CaptureErrorV1::Capacity));
        assert!(v.is_empty());
    }
}

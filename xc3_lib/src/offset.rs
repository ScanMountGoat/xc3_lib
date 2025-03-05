//! Utilities for validating offsets during parsing.
use std::sync::{Arc, Mutex};

use tracing_subscriber::Layer;

/// Named byte range for `[start, end)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OffsetRange {
    pub start: u64,
    pub end: u64,
    pub parent_type_names: Vec<String>,
    pub type_name: String,
}

/// Unexpected cases while checking offset ranges that usually indicate some sort of error.
#[derive(Debug, PartialEq, Eq)]
pub enum OffsetValidationError<'a> {
    /// Two ranges overlap.
    ///
    /// The parser should only parse a byte range once?
    /// Overlaps indicate a struct size or array length is incorrect.
    OverlappingRange {
        current: OffsetRange,
        next: OffsetRange,
    },

    /// A byte between ranges is not zero.
    ///
    /// Non padding bytes between ranges are likely missed data.
    GapWithNonPaddingBytes {
        before: OffsetRange,
        after: OffsetRange,
        gap_bytes: &'a [u8],
    },
}

pub fn validate_ranges<'a>(
    ranges: &[OffsetRange],
    bytes: &'a [u8],
) -> Vec<OffsetValidationError<'a>> {
    let mut ranges = ranges.to_vec();

    // Gap detection assumes offsets are sorted.
    ranges.sort_by_key(|r| r.start);

    let mut errors = Vec::new();

    // TODO: Detect gaps between header and first offset?
    // TODO: How to handle empty ranges?
    for i in 0..ranges.len().saturating_sub(1) {
        let current = ranges[i].clone();
        let next = ranges[i + 1].clone();

        if current.end > next.start {
            errors.push(OffsetValidationError::OverlappingRange { current, next });
        } else if current.end < next.start {
            let gap_bytes = &bytes[current.end as usize..next.start as usize];

            if gap_bytes.iter().any(|b| *b != 0) {
                errors.push(OffsetValidationError::GapWithNonPaddingBytes {
                    before: current,
                    after: next,
                    gap_bytes,
                });
            }
        }
    }

    errors
}

pub struct OffsetLayer(pub Arc<Mutex<Vec<OffsetRange>>>);

// TODO: Possible to derive this for structs with serde?
#[derive(Debug, Default)]
struct OffsetRangeVisitor {
    start: Option<u64>,
    end: Option<u64>,
    type_name: Option<String>,
}

impl tracing::field::Visit for OffsetRangeVisitor {
    fn record_u64(&mut self, field: &tracing::field::Field, value: u64) {
        if field.name() == "start" {
            self.start = Some(value);
        } else if field.name() == "end" {
            self.end = Some(value);
        }
    }

    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        if field.name() == "type_name" {
            self.type_name = Some(value.to_string());
        }
    }

    fn record_debug(&mut self, _field: &tracing::field::Field, _value: &dyn std::fmt::Debug) {}
}

#[derive(Debug, Default)]
struct TypeNameVisitor {
    type_name: Option<String>,
}

impl tracing::field::Visit for TypeNameVisitor {
    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        if field.name() == "type_name" {
            self.type_name = Some(value.to_string());
        }
    }

    fn record_debug(&mut self, _field: &tracing::field::Field, _value: &dyn std::fmt::Debug) {}
}

struct TypeName(String);

impl<S> Layer<S> for OffsetLayer
where
    S: tracing::Subscriber,
    S: for<'a> tracing_subscriber::registry::LookupSpan<'a>,
{
    fn on_event(&self, event: &tracing::Event<'_>, ctx: tracing_subscriber::layer::Context<'_, S>) {
        let mut visitor = OffsetRangeVisitor::default();
        event.record(&mut visitor);
        if let Some(start) = visitor.start {
            if let Some(end) = visitor.end {
                if let Some(type_name) = visitor.type_name {
                    let mut parent_type_names = Vec::new();

                    let scope = ctx.event_scope(event).unwrap();
                    for span in scope.from_root() {
                        if let Some(TypeName(n)) = span.extensions().get() {
                            // TODO: Is there a better way to not include the current span?
                            if n != &type_name {
                                parent_type_names.push(n.clone());
                            }
                        }
                    }

                    self.0.lock().unwrap().push(OffsetRange {
                        start,
                        end,
                        type_name,
                        parent_type_names,
                    });
                }
            }
        }
    }

    fn on_new_span(
        &self,
        attrs: &tracing::span::Attributes<'_>,
        id: &tracing::span::Id,
        ctx: tracing_subscriber::layer::Context<'_, S>,
    ) {
        let mut visitor = TypeNameVisitor::default();
        attrs.values().record(&mut visitor);

        if let Some(n) = visitor.type_name {
            ctx.span(id).unwrap().extensions_mut().insert(TypeName(n));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_ranges_empty() {
        assert!(validate_ranges(&[], &[]).is_empty());
    }

    #[test]
    fn validate_ranges_valid() {
        let ranges = vec![
            OffsetRange {
                start: 0,
                end: 4,
                type_name: "a".to_string(),
                parent_type_names: Vec::new(),
            },
            OffsetRange {
                start: 8,
                end: 12,
                type_name: "b".to_string(),
                parent_type_names: Vec::new(),
            },
            OffsetRange {
                start: 12,
                end: 16,
                type_name: "c".to_string(),
                parent_type_names: Vec::new(),
            },
        ];
        let bytes = [1, 1, 1, 1, 0, 0, 0, 0, 1, 1, 1, 1, 1, 1, 1, 1];
        assert!(validate_ranges(&ranges, &bytes).is_empty());
    }

    #[test]
    fn validate_ranges_gap_overlap() {
        let ranges = vec![
            OffsetRange {
                start: 0,
                end: 4,
                type_name: "a".to_string(),
                parent_type_names: Vec::new(),
            },
            OffsetRange {
                start: 3,
                end: 5,
                type_name: "b".to_string(),
                parent_type_names: Vec::new(),
            },
            OffsetRange {
                start: 8,
                end: 12,
                type_name: "c".to_string(),
                parent_type_names: Vec::new(),
            },
        ];
        let bytes = [1, 1, 1, 2, 2, 3, 3, 3, 1, 1, 1, 1];
        assert_eq!(
            vec![
                OffsetValidationError::OverlappingRange {
                    current: OffsetRange {
                        start: 0,
                        end: 4,
                        type_name: "a".to_string(),
                        parent_type_names: Vec::new(),
                    },
                    next: OffsetRange {
                        start: 3,
                        end: 5,
                        type_name: "b".to_string(),
                        parent_type_names: Vec::new(),
                    },
                },
                OffsetValidationError::GapWithNonPaddingBytes {
                    before: OffsetRange {
                        start: 3,
                        end: 5,
                        type_name: "b".to_string(),
                        parent_type_names: Vec::new(),
                    },
                    after: OffsetRange {
                        start: 8,
                        end: 12,
                        type_name: "c".to_string(),
                        parent_type_names: Vec::new(),
                    },
                    gap_bytes: &[3, 3, 3]
                }
            ],
            validate_ranges(&ranges, &bytes)
        );
    }
}

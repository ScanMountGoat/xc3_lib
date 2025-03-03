use std::sync::Mutex;

pub static OFFSET_LOGGER: Mutex<OffsetLogger> = Mutex::new(OffsetLogger {
    type_ranges: Vec::new(),
});

/// A logger for storing ranges of offsets during parsing.
///
/// Reset the logger with [OffsetLogger::clear].
#[derive(Default)]
pub struct OffsetLogger {
    type_ranges: Vec<OffsetRange>,
}

/// Named byte range for `[start, end)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OffsetRange {
    pub start: u64,
    pub end: u64,
    pub name: &'static str,
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

impl OffsetLogger {
    // TODO: Store a backtrace of parent structs for types?
    pub fn log_range(&mut self, start: u64, end: u64, name: &'static str) {
        self.type_ranges.push(OffsetRange { start, end, name });
    }

    pub fn validate_ranges<'a>(&self, bytes: &'a [u8]) -> Vec<OffsetValidationError<'a>> {
        let mut ranges = self.type_ranges.clone();

        // Gap detection assumes offsets are sorted.
        ranges.sort_by_key(|r| r.start);

        let mut errors = Vec::new();

        // TODO: Detect gaps between header and first offset?
        for i in 0..ranges.len().saturating_sub(1) {
            let current = ranges[i];
            let next = ranges[i + 1];

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

    pub fn clear(&mut self) {
        self.type_ranges.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_ranges_empty() {
        assert!(OffsetLogger::default().validate_ranges(&[]).is_empty());
    }

    #[test]
    fn validate_ranges_valid() {
        let mut logger = OffsetLogger::default();
        logger.log_range(0, 4, "a");
        logger.log_range(8, 12, "b");
        logger.log_range(12, 16, "c");

        let bytes = [1, 1, 1, 1, 0, 0, 0, 0, 1, 1, 1, 1, 1, 1, 1, 1];
        assert!(logger.validate_ranges(&bytes).is_empty());
    }

    #[test]
    fn validate_ranges_gap_overlap() {
        let mut logger = OffsetLogger::default();
        logger.log_range(0, 4, "a");
        logger.log_range(3, 5, "b");
        logger.log_range(8, 12, "c");

        let bytes = [1, 1, 1, 2, 2, 3, 3, 3, 1, 1, 1, 1];
        assert_eq!(
            vec![
                OffsetValidationError::OverlappingRange {
                    current: OffsetRange {
                        start: 0,
                        end: 4,
                        name: "a"
                    },
                    next: OffsetRange {
                        start: 3,
                        end: 5,
                        name: "b"
                    },
                },
                OffsetValidationError::GapWithNonPaddingBytes {
                    before: OffsetRange {
                        start: 3,
                        end: 5,
                        name: "b"
                    },
                    after: OffsetRange {
                        start: 8,
                        end: 12,
                        name: "c"
                    },
                    gap_bytes: &[3, 3, 3]
                }
            ],
            logger.validate_ranges(&bytes)
        );
    }
}

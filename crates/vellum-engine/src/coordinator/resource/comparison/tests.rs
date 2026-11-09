use super::*;
#[test]
fn combined_comparison_arithmetic_never_wraps_or_uses_length_inequality_as_a_budget_bypass() {
    let budget = SourceComparisonBudgetV1 {
        max_encoded_bytes: usize::MAX,
        max_combined_bytes: usize::MAX,
    };
    assert_eq!(preflight(usize::MAX, 1, budget), Err(ResourceError::Limit));
    assert_eq!(preflight(1, usize::MAX, budget), Err(ResourceError::Limit));
    assert_eq!(preflight(usize::MAX, 0, budget), Ok(()));
    assert_eq!(
        preflight(
            2,
            1,
            SourceComparisonBudgetV1 {
                max_combined_bytes: 2,
                ..budget
            }
        ),
        Err(ResourceError::Limit)
    );
    assert_eq!(
        preflight(
            2,
            1,
            SourceComparisonBudgetV1 {
                max_encoded_bytes: 1,
                ..budget
            }
        ),
        Err(ResourceError::Limit)
    );
    assert_eq!(
        preflight(
            0,
            0,
            SourceComparisonBudgetV1 {
                max_encoded_bytes: 0,
                max_combined_bytes: 0
            }
        ),
        Ok(())
    );
}

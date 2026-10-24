use hydrus_core::hydrus_constants::LOCAL_RATINGS_SERVICES;
use hydrus_core::hydrus_exceptions::HydrusResult;

use super::services::parse_service_specifier;
use crate::system_predicate_parser::state::ParseState;
use crate::system_predicate_parser::types::{LogicalOperator, ServiceSpecifier, ValueData};

pub(crate) fn parse_rating_advanced(state: &mut ParseState) -> HydrusResult<()> {
    let mut workspace = state.subtag_text_lower.trim().to_string();

    let mut logical_operator = LogicalOperator::All;

    for (label, operator) in [
        ("all", LogicalOperator::All),
        ("any", LogicalOperator::Any),
        ("only", LogicalOperator::Only),
    ] {
        if let Some(rest) = workspace.strip_prefix(label) {
            workspace = rest.trim().to_string();
            logical_operator = operator;
            break;
        }
    }

    let mut rated = true;

    if let Some(rest) = workspace.strip_suffix("not rated") {
        workspace = rest.trim().to_string();
        rated = false;
    } else if let Some(rest) = workspace.strip_suffix("rated") {
        workspace = rest.trim().to_string();
        rated = true;
    }

    let secondary = if logical_operator == LogicalOperator::Only {
        let workspace_copy = workspace.clone();

        if let Some((primary, secondary)) = workspace_copy.split_once("(amongst") {
            workspace = primary.trim().to_string();

            let mut secondary = secondary.trim().to_string();

            if let Some(stripped) = secondary.strip_suffix(')') {
                secondary = stripped.trim().to_string();
            }

            parse_service_specifier(&secondary)
        } else {
            ServiceSpecifier {
                service_types: LOCAL_RATINGS_SERVICES.iter().copied().collect(),
                service_names: Vec::new(),
            }
        }
    } else {
        ServiceSpecifier::default()
    };

    let primary = parse_service_specifier(&workspace);

    state.text_remainder = String::new();
    state.value = Some(ValueData::RatingAdvanced {
        logical_operator,
        primary,
        secondary,
        rated,
    });

    Ok(())
}

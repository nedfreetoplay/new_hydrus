//! File-service status operator parsing.

use fancy_regex::Regex;
use hydrus_core::hydrus_exceptions::HydrusResult;

use crate::system_predicate_parser::parse_error;
use crate::system_predicate_parser::state::ParseState;
use crate::system_predicate_parser::types::{FileServiceStatus, Operator};

pub(super) fn parse_file_service_status(
    state: &mut ParseState,
    string: &str,
) -> HydrusResult<()> {
    for (pattern, status) in [
        (r"(is )?currently in", FileServiceStatus::IsCurrentlyIn),
        (
            r"((is )?not currently in)|isn't currently in",
            FileServiceStatus::IsNotCurrentlyIn,
        ),
        (r"(is )?pending to", FileServiceStatus::IsPendingTo),
        (
            r"((is )?not pending to)|isn't pending to",
            FileServiceStatus::IsNotPendingTo,
        ),
    ] {
        let regex = Regex::new(&format!("^(?:{pattern})")).expect("valid file status regex");

        if let Some(found) = regex
            .find(string)
            .map_err(|error| parse_error(error.to_string()))?
        {
            state.text_remainder = string[found.end()..].to_string();
            state.operator = Some(Operator::FileService(status));
            return Ok(());
        }
    }

    Err(parse_error(
        "Invalid operator, expected a file service relationship",
    ))
}

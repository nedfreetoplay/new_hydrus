use std::collections::{BTreeMap, BTreeSet};

use hydrus_core::hydrus_constants::{ServiceType, LOCAL_RATINGS_SERVICES};

use crate::system_predicate_parser::parse_error;
use crate::system_predicate_parser::types::ServiceSpecifier;
use hydrus_core::hydrus_exceptions::HydrusResult;

/// `parse_service_specifier( text )`
pub fn parse_service_specifier(text: &str) -> HydrusResult<ServiceSpecifier> {
    let text = text.trim().to_lowercase();
    let text = text.as_str();

    if text == "rating" || text == "ratings" {
        return Ok(ServiceSpecifier {
            service_types: LOCAL_RATINGS_SERVICES.iter().copied().collect(),
            service_names: Vec::new(),
        });
    }

    let names: Vec<String> = text
        .split(',')
        .map(|name| name.trim().to_string())
        .collect();

    let short_names: BTreeMap<&str, ServiceType> = ServiceType::ALL
        .iter()
        .map(|service_type| (service_type.short_str(), *service_type))
        .collect();

    if names.iter().any(|name| short_names.contains_key(name.as_str())) {
        let mut service_types = BTreeSet::new();

        for name in &names {
            let Some(service_type) = short_names.get(name.as_str()) else {
                return Err(parse_error("Unknown service type!"));
            };

            service_types.insert(*service_type);
        }

        Ok(ServiceSpecifier {
            service_types,
            service_names: Vec::new(),
        })
    } else {
        Ok(ServiceSpecifier {
            service_types: BTreeSet::new(),
            service_names: names.into_iter().filter(|name| !name.is_empty()).collect(),
        })
    }
}

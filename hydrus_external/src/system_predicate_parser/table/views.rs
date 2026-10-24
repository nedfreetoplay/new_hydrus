//! System predicate table section.

use crate::system_predicate_parser::types::{OperatorKind as O, Predicate as P, ValueKind as V};

use super::spec;

pub(super) fn specs() -> Vec<super::PredicateSpec> {
    vec![
        spec(
            "media views",
            P::MediaViews,
            Some(O::Relational),
            Some(V::Natural),
            None,
        ),
        spec(
            "preview views",
            P::PreviewViews,
            Some(O::Relational),
            Some(V::Natural),
            None,
        ),
        spec(
            "all views",
            P::AllViews,
            Some(O::Relational),
            Some(V::Natural),
            None,
        ),
        spec(
            "^views (in )?",
            P::NewViews,
            Some(O::ViewsRelational),
            Some(V::Natural),
            None,
        ),
        spec(
            "media viewtime",
            P::MediaViewtime,
            Some(O::Relational),
            Some(V::TimeInterval),
            None,
        ),
        spec(
            "preview viewtime",
            P::PreviewViewtime,
            Some(O::Relational),
            Some(V::TimeInterval),
            None,
        ),
        spec(
            "all viewtime",
            P::AllViewtime,
            Some(O::Relational),
            Some(V::TimeInterval),
            None,
        ),
        spec(
            "^viewtime (in )?",
            P::NewViewtime,
            Some(O::ViewsRelational),
            Some(V::TimeInterval),
            None,
        ),
        spec(
            "has (a )?url matching regex",
            P::UrlRegex,
            None,
            Some(V::AnyString),
            None,
        ),
        spec(
            "(does not|doesn't) have (a )?url matching regex",
            P::NoUrlRegex,
            None,
            Some(V::AnyString),
            None,
        ),
        spec("has url:? (?=http)", P::Url, None, Some(V::AnyString), None),
        spec(
            "(does not|doesn't) have url:? (?=http)",
            P::NoUrl,
            None,
            Some(V::AnyString),
            None,
        ),
        spec(
            "has (an? )?(url with )?domain",
            P::Domain,
            None,
            Some(V::AnyString),
            None,
        ),
        spec(
            "(does not|doesn't) have (an? )?(url with )?domain",
            P::NoDomain,
            None,
            Some(V::AnyString),
            None,
        ),
        spec(
            "has (an? )?url with (url )?class",
            P::UrlClass,
            None,
            Some(V::AnyString),
            None,
        ),
        spec(
            "(does not|doesn't) have (an? )?url with (url )?class",
            P::NoUrlClass,
            None,
            Some(V::AnyString),
            None,
        ),
        spec(
            "tag as number",
            P::TagAsNumber,
            Some(O::TagRelational),
            Some(V::Integer),
            None,
        ),
    ]
}

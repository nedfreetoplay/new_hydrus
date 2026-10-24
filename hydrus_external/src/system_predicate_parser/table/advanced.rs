//! System predicate table section.

use crate::system_predicate_parser::types::{OperatorKind as O, Predicate as P, ValueKind as V};

use super::spec;

pub(super) fn specs() -> Vec<super::PredicateSpec> {
    vec![
        spec("has notes?$", P::HasNotes, None, None, None),
        spec(
            "((has )?no|does not have( a)?|doesn't have) notes?$",
            P::NoNotes,
            None,
            None,
            None,
        ),
        spec(
            "num(ber)?( of)? notes?",
            P::NumNotes,
            Some(O::RelationalExact),
            Some(V::Natural),
            None,
        ),
        spec(
            "(has (a )?)?note (with name|named)",
            P::HasNoteName,
            None,
            Some(V::AnyString),
            None,
        ),
        spec(
            "((has )?no|does not have( a)?|doesn't have( a)?) note (with name|named)",
            P::NoNoteName,
            None,
            Some(V::AnyString),
            None,
        ),
        spec(
            "has( a)? (rating|count)( for)?",
            P::HasRating,
            None,
            Some(V::AnyString),
            None,
        ),
        spec(
            "((has )?no|does not have( a)?|doesn't have( a)?) (rating|count)( for)?",
            P::NoRating,
            None,
            Some(V::AnyString),
            None,
        ),
        spec(
            r"(rating|count)( for)?(?=.+?\d+/\d+$)",
            P::RatingSpecificNumerical,
            Some(O::RelationalForRatingService),
            Some(V::RatingServiceNameAndNumericalValue),
            None,
        ),
        spec(
            "(rating|count)( for)?(?=.+?(like|dislike)$)",
            P::RatingSpecificLikeDislike,
            None,
            Some(V::RatingServiceNameAndLikeDislike),
            None,
        ),
        spec(
            r"(rating|count)( for)?(?=.+?[^/]\d+$)",
            P::RatingSpecificIncDec,
            Some(O::RelationalForRatingService),
            Some(V::RatingServiceNameAndIncDec),
            None,
        ),
        spec(
            "has tag",
            P::TagAdvancedInclusive,
            Some(O::TagAdvancedGubbins),
            Some(V::TagAdvancedTag),
            None,
        ),
        spec(
            "does not have tag",
            P::TagAdvancedExclusive,
            Some(O::TagAdvancedGubbins),
            Some(V::TagAdvancedTag),
            None,
        ),
        spec(
            "(all|any|only).+rated",
            P::RatingAdvanced,
            None,
            Some(V::RatingAdvanced),
            None,
        ),
    ]
}

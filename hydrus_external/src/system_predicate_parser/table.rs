//! The `SYSTEM_PREDICATES` table.

use std::sync::LazyLock;

use fancy_regex::Regex;

use super::types::{
    OperatorKind, Predicate, UnitKind, ValueKind,
};

pub(crate) struct PredicateSpec {
    pub regex: Regex,
    pub predicate: Predicate,
    pub operator: Option<OperatorKind>,
    pub value: Option<ValueKind>,
    pub unit: Option<UnitKind>,
}

fn spec(
    pattern: &str,
    predicate: Predicate,
    operator: Option<OperatorKind>,
    value: Option<ValueKind>,
    unit: Option<UnitKind>,
) -> PredicateSpec {
    let compiled = format!("^(?:{}):?", pattern.replace(' ', "([_ ]+)"));

    PredicateSpec {
        regex: Regex::new(&compiled).expect("valid system predicate regex"),
        predicate,
        operator,
        value,
        unit,
    }
}

pub(crate) fn predicate_specs() -> &'static Vec<PredicateSpec> {
    static SPECS: LazyLock<Vec<PredicateSpec>> = LazyLock::new(|| {
        use OperatorKind as O;
        use Predicate as P;
        use UnitKind as U;
        use ValueKind as V;

        vec![
            spec("everything", P::Everything, None, None, None),
            spec("inbox", P::Inbox, None, None, None),
            spec("archived?$", P::Archive, None, None, None),
            spec("has duration", P::HasDuration, None, None, None),
            spec("no duration", P::NoDuration, None, None, None),
            spec("has framerate", P::HasFramerate, None, None, None),
            spec("no framerate", P::NoFramerate, None, None, None),
            spec("has frames", P::HasFrames, None, None, None),
            spec("no frames", P::NoFrames, None, None, None),
            spec("has width", P::HasWidth, None, None, None),
            spec("no width", P::NoWidth, None, None, None),
            spec("has height", P::HasHeight, None, None, None),
            spec("no height", P::NoHeight, None, None, None),
            spec("has notes", P::HasNotes, None, None, None),
            spec("no notes", P::NoNotes, None, None, None),
            spec("has urls", P::HasUrls, None, None, None),
            spec("no urls", P::NoUrls, None, None, None),
            spec("has words", P::HasWords, None, None, None),
            spec("no words", P::NoWords, None, None, None),
            spec(
                "(is the )?best quality( file)? of( its)?( duplicate)? group",
                P::BestQualityOfGroup,
                None,
                None,
                None,
            ),
            spec(
                "(((is )?not)|(isn't))( the)? best quality( file)? of( its)?( duplicate)? group",
                P::NotBestQualityOfGroup,
                None,
                None,
                None,
            ),
            spec("has audio", P::HasAudio, None, None, None),
            spec("no audio", P::NoAudio, None, None, None),
            spec("has (transparency|alpha)", P::HasTransparency, None, None, None),
            spec("no (transparency|alpha)", P::NoTransparency, None, None, None),
            spec("has exif", P::HasExif, None, None, None),
            spec("no exif", P::NoExif, None, None, None),
            spec("has xmp", P::HasXmp, None, None, None),
            spec("no xmp", P::NoXmp, None, None, None),
            spec("has iptc", P::HasIptc, None, None, None),
            spec("no iptc", P::NoIptc, None, None, None),
            spec("has.*embedded.*metadata", P::HasHumanReadableEmbeddedMetadata, None, None, None),
            spec("no.*embedded.*metadata", P::NoHumanReadableEmbeddedMetadata, None, None, None),
            spec("has.*human-readable.*metadata", P::HasHumanReadableEmbeddedMetadata, None, None, None),
            spec("no.*human-readable.*metadata", P::NoHumanReadableEmbeddedMetadata, None, None, None),
            spec("has software/source metadata", P::HasSoftwareSource, None, None, None),
            spec("no software/source metadata", P::NoSoftwareSource, None, None, None),
            spec("has icc profile", P::HasIccProfile, None, None, None),
            spec("no icc profile", P::NoIccProfile, None, None, None),
            spec("has forced filetype", P::HasForcedFiletype, None, None, None),
            spec("no forced filetype", P::NoForcedFiletype, None, None, None),
            spec("has tags", P::HasTags, None, None, None),
            spec("untagged|no tags", P::Untagged, None, None, None),
            spec("num(ber)?( of)? tags", P::NumOfTags, Some(O::Relational), Some(V::Natural), None),
            spec(r"num(ber)?( of)? (?=[^\s].* tags)", P::NumOfTagsWithNamespace, None, Some(V::NamespaceAndNumTags), None),
            spec("num(ber)?( of)? urls", P::NumOfUrls, Some(O::Relational), Some(V::Natural), None),
            spec("num(ber)?( of)? words", P::NumOfWords, Some(O::RelationalNumberTest), Some(V::Natural), None),
            spec("height", P::Height, Some(O::RelationalNumberTest), Some(V::Natural), Some(U::PixelsOrNone)),
            spec("width", P::Width, Some(O::RelationalNumberTest), Some(V::Natural), Some(U::PixelsOrNone)),
            spec("file ?size", P::Filesize, Some(O::Relational), Some(V::Natural), Some(U::Filesize)),
            spec("similar to(?! data)( files)?", P::SimilarToFiles, None, Some(V::Sha256HashListWithDistance), None),
            spec("similar to data", P::SimilarToData, None, Some(V::SimilarToHashListWithDistance), None),
            spec("limit", P::Limit, Some(O::OnlyEqual), Some(V::Natural), None),
            spec("file ?type", P::Filetype, Some(O::Equal), Some(V::FiletypeList), None),
            spec(r"hash( \(?(md5|sha1|sha512)\)?)?", P::Hash, Some(O::Equal), Some(V::HashListWithAlgorithm), None),
            spec("archived? (date|time)|(date|time) archived|archived.", P::ArchivedDate, Some(O::RelationalTime), Some(V::DateOrTimeInterval), None),
            spec("modified (date|time)|(date|time) modified|modified", P::ModDate, Some(O::RelationalTime), Some(V::DateOrTimeInterval), None),
            spec("last view(ed)? (date|time)|(date|time) last viewed|last viewed", P::LastViewedTime, Some(O::RelationalTime), Some(V::DateOrTimeInterval), None),
            spec("import(ed)? (date|time)|(date|time) imported|imported", P::TimeImported, Some(O::RelationalTime), Some(V::DateOrTimeInterval), None),
            spec("duration", P::Duration, Some(O::RelationalNumberTest), Some(V::TimeToMsec), None),
            spec("framerate", P::Framerate, Some(O::Relational), Some(V::Natural), Some(U::FpsOrNone)),
            spec("num(ber)?( of)? frames", P::NumOfFrames, Some(O::RelationalNumberTest), Some(V::Natural), None),
            spec("file service", P::FileService, Some(O::FileServiceStatus), Some(V::AnyString), None),
            spec("num(ber)?( of)? file relationships", P::NumFileRels, Some(O::Relational), Some(V::Natural), Some(U::FileRelationshipType)),
            spec(r"ratio(?=.*\d)", P::Ratio, Some(O::RatioOperators), Some(V::Ratio), None),
            spec(r"ratio(?!.*\d)", P::RatioSpecial, Some(O::RatioOperatorsSpecial), None, None),
            spec("num(ber)?( of)? pixels", P::NumPixels, Some(O::Relational), Some(V::Natural), Some(U::Pixels)),
            spec("media views", P::MediaViews, Some(O::Relational), Some(V::Natural), None),
            spec("preview views", P::PreviewViews, Some(O::Relational), Some(V::Natural), None),
            spec("all views", P::AllViews, Some(O::Relational), Some(V::Natural), None),
            spec("^views (in )?", P::NewViews, Some(O::ViewsRelational), Some(V::Natural), None),
            spec("media viewtime", P::MediaViewtime, Some(O::Relational), Some(V::TimeInterval), None),
            spec("preview viewtime", P::PreviewViewtime, Some(O::Relational), Some(V::TimeInterval), None),
            spec("all viewtime", P::AllViewtime, Some(O::Relational), Some(V::TimeInterval), None),
            spec("^viewtime (in )?", P::NewViewtime, Some(O::ViewsRelational), Some(V::TimeInterval), None),
            spec("has (a )?url matching regex", P::UrlRegex, None, Some(V::AnyString), None),
            spec("(does not|doesn't) have (a )?url matching regex", P::NoUrlRegex, None, Some(V::AnyString), None),
            spec("has url:? (?=http)", P::Url, None, Some(V::AnyString), None),
            spec("(does not|doesn't) have url:? (?=http)", P::NoUrl, None, Some(V::AnyString), None),
            spec("has (an? )?(url with )?domain", P::Domain, None, Some(V::AnyString), None),
            spec("(does not|doesn't) have (an? )?(url with )?domain", P::NoDomain, None, Some(V::AnyString), None),
            spec("has (an? )?url with (url )?class", P::UrlClass, None, Some(V::AnyString), None),
            spec("(does not|doesn't) have (an? )?url with (url )?class", P::NoUrlClass, None, Some(V::AnyString), None),
            spec("tag as number", P::TagAsNumber, Some(O::TagRelational), Some(V::Integer), None),
            spec("has notes?$", P::HasNotes, None, None, None),
            spec("((has )?no|does not have( a)?|doesn't have) notes?$", P::NoNotes, None, None, None),
            spec("num(ber)?( of)? notes?", P::NumNotes, Some(O::RelationalExact), Some(V::Natural), None),
            spec("(has (a )?)?note (with name|named)", P::HasNoteName, None, Some(V::AnyString), None),
            spec("((has )?no|does not have( a)?|doesn't have( a)?) note (with name|named)", P::NoNoteName, None, Some(V::AnyString), None),
            spec("has( a)? (rating|count)( for)?", P::HasRating, None, Some(V::AnyString), None),
            spec("((has )?no|does not have( a)?|doesn't have( a)?) (rating|count)( for)?", P::NoRating, None, Some(V::AnyString), None),
            spec(r"(rating|count)( for)?(?=.+?\d+/\d+$)", P::RatingSpecificNumerical, Some(O::RelationalForRatingService), Some(V::RatingServiceNameAndNumericalValue), None),
            spec("(rating|count)( for)?(?=.+?(like|dislike)$)", P::RatingSpecificLikeDislike, None, Some(V::RatingServiceNameAndLikeDislike), None),
            spec(r"(rating|count)( for)?(?=.+?[^/]\d+$)", P::RatingSpecificIncDec, Some(O::RelationalForRatingService), Some(V::RatingServiceNameAndIncDec), None),
            spec("has tag", P::TagAdvancedInclusive, Some(O::TagAdvancedGubbins), Some(V::TagAdvancedTag), None),
            spec("does not have tag", P::TagAdvancedExclusive, Some(O::TagAdvancedGubbins), Some(V::TagAdvancedTag), None),
            spec("(all|any|only).+rated", P::RatingAdvanced, None, Some(V::RatingAdvanced), None),
        ]
    });

    &SPECS
}

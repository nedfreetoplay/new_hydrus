use hydrus_core::hydrus_constants::ContentStatus;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RelationalOperator {
    Equal,
    NotEqual,
    Less,
    Greater,
    LessEqual,
    GreaterEqual,
    Approx,
}

impl RelationalOperator {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Equal => "=",
            Self::NotEqual => "\u{2260}",
            Self::Less => "<",
            Self::Greater => ">",
            Self::LessEqual => "\u{2264}",
            Self::GreaterEqual => "\u{2265}",
            Self::Approx => "\u{2248}",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CanvasType {
    Media,
    Preview,
    ClientApi,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FileServiceStatus {
    IsCurrentlyIn,
    IsNotCurrentlyIn,
    IsPendingTo,
    IsNotPendingTo,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RatioOperator {
    Equal,
    WiderThan,
    TallerThan,
    Approx,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TagDisplayType {
    DisplayActual,
    Storage,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum LogicalOperator {
    All,
    Any,
    Only,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Unit {
    Bytes,
    Kilobytes,
    Megabytes,
    Gigabytes,
    Duplicates,
    Alternates,
    NotRelatedOrFalsePositive,
    PotentialDuplicates,
    Pixels,
    Kilopixels,
    Megapixels,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Operator {
    None,
    Relational(RelationalOperator),
    Views {
        canvas_types: Vec<CanvasType>,
        relational: RelationalOperator,
    },
    FileService(FileServiceStatus),
    TagRelational {
        namespace: String,
        relational: RelationalOperator,
    },
    Ratio(RatioOperator),
    TagAdvanced {
        service_name: Option<String>,
        display_type: TagDisplayType,
        statuses: Vec<ContentStatus>,
    },
}

//! Rust port of `hydrus/core/HydrusExceptions.py`.
//!
//! Python has one class per failure mode and catches them by class. Rust does
//! not have exception hierarchies, so all hydrus failures are carried by
//! [`HydrusException`], whose [`ErrorKind`] names the Python class. The
//! hierarchy is still queryable via [`ErrorKind::is_a`], which keeps ported
//! `except FooException` logic readable:
//!
//! ```
//! use hydrus_core::hydrus_exceptions::{ErrorKind, HydrusException};
//!
//! let e = HydrusException::new(ErrorKind::Shutdown, "closing");
//! assert!(e.is_a(ErrorKind::Shutdown));
//! assert!(!e.is_a(ErrorKind::Veto));
//! ```

use std::error::Error;
use std::fmt;

/// Every exception class defined by `HydrusExceptions.py`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ErrorKind {
    // base
    Hydrus,
    Unknown,
    CantRenderWithCV,
    DataMissing,
    TooComplicatedM8,

    // db
    Db,
    DbAccess,
    DbCredentials,
    DbVersion,

    // files / disk
    FileMissing,
    DirectoryMissing,
    Serialisation,
    Name,
    Shutdown,
    QtDeadWindow,

    // subprocess
    Executable,
    SubprocessTimedOut,
    BadReturnCode,

    // unsupported files
    UnsupportedFile,
    ZeroSizeFile,
    DamagedOrUnusualFile,

    // limited support
    LimitedSupportFile,
    EncryptedFile,
    NoThumbnailFile,
    NoRenderFile,
    NoResolutionFile,

    // vetoes
    Veto,
    Cancelled,
    FileImportBlock,
    FileImportRules,
    DecompressionBomb,
    TagSize,

    // parsing
    Parse,
    StringConvert,
    StringJoiner,
    StringMatch,
    StringSplitter,
    StringSort,
    UrlClass,
    Gug,

    // network
    Network,
    NetworkInfrastructure,
    Connection,
    Firewall,
    Router,
    CloudFlare,
    Bandwidth,
    Censorship,
    Server,
    ServerBusy,
    StreamTimeout,
    NetworkVersion,
    NoContent,
    NotFound,
    NotAcceptable,
    NotModified,
    BadRequest,
    Conflict,
    UnprocessableEntity,
    RangeNotSatisfiable,
    MissingCredentials,
    DoesNotSupportCors,
    InsufficientCredentials,
    Redirection,
    Session,
    WrongServiceType,
    Validation,
    ShouldReattemptNetwork,
}

impl ErrorKind {
    /// The Python class name, e.g. `ShutdownException`.
    pub const fn name(self) -> &'static str {
        match self {
            ErrorKind::Hydrus => "HydrusException",
            ErrorKind::Unknown => "UnknownException",
            ErrorKind::CantRenderWithCV => "CantRenderWithCVException",
            ErrorKind::DataMissing => "DataMissing",
            ErrorKind::TooComplicatedM8 => "TooComplicatedM8",
            ErrorKind::Db => "DBException",
            ErrorKind::DbAccess => "DBAccessException",
            ErrorKind::DbCredentials => "DBCredentialsException",
            ErrorKind::DbVersion => "DBVersionException",
            ErrorKind::FileMissing => "FileMissingException",
            ErrorKind::DirectoryMissing => "DirectoryMissingException",
            ErrorKind::Serialisation => "SerialisationException",
            ErrorKind::Name => "NameException",
            ErrorKind::Shutdown => "ShutdownException",
            ErrorKind::QtDeadWindow => "QtDeadWindowException",
            ErrorKind::Executable => "ExecutableException",
            ErrorKind::SubprocessTimedOut => "SubprocessTimedOut",
            ErrorKind::BadReturnCode => "BadReturnCodeException",
            ErrorKind::UnsupportedFile => "UnsupportedFileException",
            ErrorKind::ZeroSizeFile => "ZeroSizeFileException",
            ErrorKind::DamagedOrUnusualFile => "DamagedOrUnusualFileException",
            ErrorKind::LimitedSupportFile => "LimitedSupportFileException",
            ErrorKind::EncryptedFile => "EncryptedFileException",
            ErrorKind::NoThumbnailFile => "NoThumbnailFileException",
            ErrorKind::NoRenderFile => "NoRenderFileException",
            ErrorKind::NoResolutionFile => "NoResolutionFileException",
            ErrorKind::Veto => "VetoException",
            ErrorKind::Cancelled => "CancelledException",
            ErrorKind::FileImportBlock => "FileImportBlockException",
            ErrorKind::FileImportRules => "FileImportRulesException",
            ErrorKind::DecompressionBomb => "DecompressionBombException",
            ErrorKind::TagSize => "TagSizeException",
            ErrorKind::Parse => "ParseException",
            ErrorKind::StringConvert => "StringConvertException",
            ErrorKind::StringJoiner => "StringJoinerException",
            ErrorKind::StringMatch => "StringMatchException",
            ErrorKind::StringSplitter => "StringSplitterException",
            ErrorKind::StringSort => "StringSortException",
            ErrorKind::UrlClass => "URLClassException",
            ErrorKind::Gug => "GUGException",
            ErrorKind::Network => "NetworkException",
            ErrorKind::NetworkInfrastructure => "NetworkInfrastructureException",
            ErrorKind::Connection => "ConnectionException",
            ErrorKind::Firewall => "FirewallException",
            ErrorKind::Router => "RouterException",
            ErrorKind::CloudFlare => "CloudFlareException",
            ErrorKind::Bandwidth => "BandwidthException",
            ErrorKind::Censorship => "CensorshipException",
            ErrorKind::Server => "ServerException",
            ErrorKind::ServerBusy => "ServerBusyException",
            ErrorKind::StreamTimeout => "StreamTimeoutException",
            ErrorKind::NetworkVersion => "NetworkVersionException",
            ErrorKind::NoContent => "NoContentException",
            ErrorKind::NotFound => "NotFoundException",
            ErrorKind::NotAcceptable => "NotAcceptable",
            ErrorKind::NotModified => "NotModifiedException",
            ErrorKind::BadRequest => "BadRequestException",
            ErrorKind::Conflict => "ConflictException",
            ErrorKind::UnprocessableEntity => "UnprocessableEntity",
            ErrorKind::RangeNotSatisfiable => "RangeNotSatisfiableException",
            ErrorKind::MissingCredentials => "MissingCredentialsException",
            ErrorKind::DoesNotSupportCors => "DoesNotSupportCORSException",
            ErrorKind::InsufficientCredentials => "InsufficientCredentialsException",
            ErrorKind::Redirection => "RedirectionException",
            ErrorKind::Session => "SessionException",
            ErrorKind::WrongServiceType => "WrongServiceTypeException",
            ErrorKind::Validation => "ValidationException",
            ErrorKind::ShouldReattemptNetwork => "ShouldReattemptNetworkException",
        }
    }

    /// The direct Python superclass, or `None` for `HydrusException`.
    pub const fn parent(self) -> Option<ErrorKind> {
        use ErrorKind::*;

        match self {
            Hydrus => None,

            Unknown | CantRenderWithCV | DataMissing | TooComplicatedM8 | DbAccess
            | DbCredentials | DbVersion | FileMissing | DirectoryMissing | Serialisation
            | Name | Shutdown | QtDeadWindow | Executable | UnsupportedFile
            | LimitedSupportFile | Veto | Parse | Network => Some(Hydrus),

            Db => Some(Hydrus),

            SubprocessTimedOut | BadReturnCode => Some(Executable),

            ZeroSizeFile | DamagedOrUnusualFile => Some(UnsupportedFile),

            EncryptedFile | NoThumbnailFile | NoRenderFile | NoResolutionFile => {
                Some(LimitedSupportFile)
            }

            Cancelled | FileImportBlock | FileImportRules | TagSize => Some(Veto),
            DecompressionBomb => Some(FileImportRules),

            StringConvert | StringJoiner | StringMatch | StringSplitter | StringSort
            | UrlClass | Gug => Some(Parse),

            NetworkInfrastructure => Some(Network),
            StreamTimeout | NetworkVersion | NoContent | NotFound | NotAcceptable
            | NotModified | BadRequest | Conflict | UnprocessableEntity
            | RangeNotSatisfiable | MissingCredentials | DoesNotSupportCors
            | InsufficientCredentials | Redirection | Session | WrongServiceType
            | Validation | ShouldReattemptNetwork => Some(Network),

            Connection | Firewall | Router | CloudFlare | Bandwidth | Censorship | Server
            | ServerBusy => Some(NetworkInfrastructure),
        }
    }

    /// `isinstance( e, kind )`: exact kind or any ancestor.
    pub fn is_a(self, kind: ErrorKind) -> bool {
        let mut current = Some(self);

        while let Some(k) = current {
            if k == kind {
                return true;
            }

            current = k.parent();
        }

        false
    }
}

impl fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// A hydrus exception: a kind plus the Python `args` tuple rendered as text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HydrusException {
    pub kind: ErrorKind,
    /// Python's `self.args`. `Display` joins these with newlines, matching
    /// `HydrusException.__str__`.
    pub args: Vec<String>,
}

impl HydrusException {
    pub fn new(kind: ErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            args: vec![message.into()],
        }
    }

    pub fn with_args<I, S>(kind: ErrorKind, args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self {
            kind,
            args: args.into_iter().map(Into::into).collect(),
        }
    }

    pub fn kind(&self) -> ErrorKind {
        self.kind
    }

    pub fn is_a(&self, kind: ErrorKind) -> bool {
        self.kind.is_a(kind)
    }

    /// Build a `DBException` the way Python does: `( e, first_line, db_traceback )`.
    pub fn db(first_line: impl Into<String>, db_traceback: impl Into<String>) -> Self {
        Self::with_args(ErrorKind::Db, [first_line.into(), db_traceback.into()])
    }

    pub fn message(&self) -> String {
        self.to_string()
    }
}

impl fmt::Display for HydrusException {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut first = true;

        for arg in &self.args {
            if !first {
                f.write_str("\n")?;
            }

            f.write_str(arg)?;
            first = false;
        }

        Ok(())
    }
}

impl Error for HydrusException {}

pub type HydrusResult<T> = Result<T, HydrusException>;

// Handy constructors, one per frequently-thrown type.

macro_rules! error_ctor {
    ($func:ident, $kind:ident) => {
        pub fn $func(message: impl Into<String>) -> HydrusException {
            HydrusException::new(ErrorKind::$kind, message)
        }
    };
}

error_ctor!(unknown, Unknown);
error_ctor!(data_missing, DataMissing);
error_ctor!(file_missing, FileMissing);
error_ctor!(directory_missing, DirectoryMissing);
error_ctor!(serialisation, Serialisation);
error_ctor!(shutdown, Shutdown);
error_ctor!(veto, Veto);
error_ctor!(cancelled, Cancelled);
error_ctor!(tag_size, TagSize);
error_ctor!(parse, Parse);
error_ctor!(not_found, NotFound);
error_ctor!(no_content, NoContent);
error_ctor!(conflict, Conflict);
error_ctor!(validation, Validation);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hierarchy_behaves_like_python_isinstance() {
        let cancelled = HydrusException::new(ErrorKind::Cancelled, "user cancelled");

        assert!(cancelled.is_a(ErrorKind::Cancelled));
        assert!(cancelled.is_a(ErrorKind::Veto));
        assert!(cancelled.is_a(ErrorKind::Hydrus));
        assert!(!cancelled.is_a(ErrorKind::Shutdown));
        assert!(!cancelled.is_a(ErrorKind::Network));
    }

    #[test]
    fn network_hierarchy() {
        assert!(ErrorKind::Connection.is_a(ErrorKind::NetworkInfrastructure));
        assert!(ErrorKind::Connection.is_a(ErrorKind::Network));
        assert!(!ErrorKind::Network.is_a(ErrorKind::Connection));
    }

    #[test]
    fn display_joins_args_with_newlines() {
        let e = HydrusException::with_args(ErrorKind::Db, ["first line", "traceback"]);

        assert_eq!(e.to_string(), "first line\ntraceback");
        assert_eq!(e.kind().name(), "DBException");
    }
}

//! Regeneration job ids from `hydrus/client/files/ClientFilesMaintenance.py`.

pub const REGENERATE_FILE_DATA_JOB_FILE_METADATA: i32 = 0;
pub const REGENERATE_FILE_DATA_JOB_FORCE_THUMBNAIL: i32 = 1;
pub const REGENERATE_FILE_DATA_JOB_REFIT_THUMBNAIL: i32 = 2;
pub const REGENERATE_FILE_DATA_JOB_OTHER_HASHES: i32 = 3;
pub const REGENERATE_FILE_DATA_JOB_DELETE_NEIGHBOUR_DUPES: i32 = 4;
pub const REGENERATE_FILE_DATA_JOB_FILE_INTEGRITY_PRESENCE_REMOVE_RECORD: i32 = 5;
pub const REGENERATE_FILE_DATA_JOB_FILE_INTEGRITY_DATA_REMOVE_RECORD: i32 = 6;
pub const REGENERATE_FILE_DATA_JOB_FIX_PERMISSIONS: i32 = 7;
pub const REGENERATE_FILE_DATA_JOB_CHECK_POTENTIAL_DUPLICATE_PAIR_SEARCH_MEMBERSHIP: i32 = 8;
pub const REGENERATE_FILE_DATA_JOB_PERCEPTUAL_HASHES: i32 = 9;
pub const REGENERATE_FILE_DATA_JOB_FILE_MODIFIED_TIMESTAMP: i32 = 10;
pub const REGENERATE_FILE_DATA_JOB_FILE_INTEGRITY_PRESENCE_TRY_URL: i32 = 11;
pub const REGENERATE_FILE_DATA_JOB_FILE_INTEGRITY_DATA_TRY_URL: i32 = 12;
pub const REGENERATE_FILE_DATA_JOB_FILE_INTEGRITY_DATA_SILENT_DELETE: i32 = 13;
pub const REGENERATE_FILE_DATA_JOB_FILE_INTEGRITY_PRESENCE_TRY_URL_ELSE_REMOVE_RECORD: i32 = 14;
pub const REGENERATE_FILE_DATA_JOB_FILE_INTEGRITY_DATA_TRY_URL_ELSE_REMOVE_RECORD: i32 = 15;
pub const REGENERATE_FILE_DATA_JOB_FILE_HAS_ICC_PROFILE: i32 = 16;
pub const REGENERATE_FILE_DATA_JOB_PIXEL_HASH: i32 = 17;
pub const REGENERATE_FILE_DATA_JOB_FILE_INTEGRITY_PRESENCE_LOG_ONLY: i32 = 18;
pub const REGENERATE_FILE_DATA_JOB_FILE_HAS_HUMAN_READABLE_EMBEDDED_METADATA: i32 = 19;
pub const REGENERATE_FILE_DATA_JOB_FILE_HAS_EXIF: i32 = 20;
pub const REGENERATE_FILE_DATA_JOB_FILE_INTEGRITY_PRESENCE_DELETE_RECORD: i32 = 21;
pub const REGENERATE_FILE_DATA_JOB_BLURHASH: i32 = 22;
pub const REGENERATE_FILE_DATA_JOB_FILE_HAS_TRANSPARENCY: i32 = 23;
pub const REGENERATE_FILE_DATA_JOB_FILE_HAS_XMP: i32 = 24;
pub const REGENERATE_FILE_DATA_JOB_FILE_HAS_IPTC: i32 = 25;
pub const REGENERATE_FILE_DATA_JOB_FILE_HAS_SOFTWARE_SOURCE: i32 = 26;

/// Python weights big jobs as 4 normal jobs.
pub const NORMALISED_BIG_JOB_WEIGHT: i64 = 4;

pub fn regen_file_enum_to_str(job_type: i32) -> &'static str {
    match job_type {
        REGENERATE_FILE_DATA_JOB_FILE_METADATA => "regenerate file metadata",
        REGENERATE_FILE_DATA_JOB_FORCE_THUMBNAIL => "regenerate thumbnail",
        REGENERATE_FILE_DATA_JOB_REFIT_THUMBNAIL => "regenerate thumbnail if incorrect size",
        REGENERATE_FILE_DATA_JOB_OTHER_HASHES => "regenerate non-standard hashes",
        REGENERATE_FILE_DATA_JOB_DELETE_NEIGHBOUR_DUPES => {
            "delete duplicate neighbours with incorrect file extension"
        }
        REGENERATE_FILE_DATA_JOB_FILE_INTEGRITY_PRESENCE_REMOVE_RECORD => {
            "if file is missing, remove record (leave no delete record)"
        }
        REGENERATE_FILE_DATA_JOB_FILE_INTEGRITY_PRESENCE_DELETE_RECORD => {
            "if file is missing, remove record (leave a delete record)"
        }
        REGENERATE_FILE_DATA_JOB_FILE_INTEGRITY_PRESENCE_TRY_URL => {
            "if file is missing, then if has URL try to redownload"
        }
        REGENERATE_FILE_DATA_JOB_FILE_INTEGRITY_PRESENCE_TRY_URL_ELSE_REMOVE_RECORD => {
            "if file is missing, then if has URL try to redownload, else remove record"
        }
        REGENERATE_FILE_DATA_JOB_FILE_INTEGRITY_PRESENCE_LOG_ONLY => {
            "if file is missing, note it in log"
        }
        REGENERATE_FILE_DATA_JOB_FILE_INTEGRITY_DATA_REMOVE_RECORD => {
            "if file is missing/incorrect, move file out and remove record"
        }
        REGENERATE_FILE_DATA_JOB_FILE_INTEGRITY_DATA_TRY_URL => {
            "if file is missing/incorrect, then if has URL try to redownload"
        }
        REGENERATE_FILE_DATA_JOB_FILE_INTEGRITY_DATA_SILENT_DELETE => {
            "if file is missing/incorrect, quietly delete record"
        }
        REGENERATE_FILE_DATA_JOB_FILE_INTEGRITY_DATA_TRY_URL_ELSE_REMOVE_RECORD => {
            "if file is missing/incorrect, try URL, else remove record"
        }
        REGENERATE_FILE_DATA_JOB_FIX_PERMISSIONS => "fix file permissions",
        REGENERATE_FILE_DATA_JOB_FILE_MODIFIED_TIMESTAMP => {
            "regenerate file modified time"
        }
        REGENERATE_FILE_DATA_JOB_CHECK_POTENTIAL_DUPLICATE_PAIR_SEARCH_MEMBERSHIP => {
            "check potential duplicate pair search membership"
        }
        REGENERATE_FILE_DATA_JOB_PERCEPTUAL_HASHES => "regenerate perceptual hashes",
        REGENERATE_FILE_DATA_JOB_FILE_HAS_ICC_PROFILE => "check for icc profile",
        REGENERATE_FILE_DATA_JOB_PIXEL_HASH => "regenerate pixel hash",
        REGENERATE_FILE_DATA_JOB_FILE_HAS_HUMAN_READABLE_EMBEDDED_METADATA => {
            "check for human-readable embedded metadata"
        }
        REGENERATE_FILE_DATA_JOB_FILE_HAS_EXIF => "check for exif",
        REGENERATE_FILE_DATA_JOB_BLURHASH => "regenerate blurhash",
        REGENERATE_FILE_DATA_JOB_FILE_HAS_TRANSPARENCY => "check for transparency",
        REGENERATE_FILE_DATA_JOB_FILE_HAS_XMP => "check for xmp",
        REGENERATE_FILE_DATA_JOB_FILE_HAS_IPTC => "check for iptc",
        REGENERATE_FILE_DATA_JOB_FILE_HAS_SOFTWARE_SOURCE => "check for software source",
        _ => "unknown job",
    }
}

pub fn is_big_job(job_type: i32) -> bool {
    matches!(
        job_type,
        REGENERATE_FILE_DATA_JOB_PERCEPTUAL_HASHES
            | REGENERATE_FILE_DATA_JOB_PIXEL_HASH
            | REGENERATE_FILE_DATA_JOB_BLURHASH
            | REGENERATE_FILE_DATA_JOB_FILE_METADATA
    )
}

pub fn normalised_weight(job_type: i32) -> i64 {
    if is_big_job(job_type) {
        NORMALISED_BIG_JOB_WEIGHT
    } else {
        1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_known_job_has_a_description() {
        for job_type in 0..=26 {
            assert_ne!(regen_file_enum_to_str(job_type), "unknown job", "{job_type}");
        }
    }

    #[test]
    fn big_jobs_have_higher_weight() {
        assert_eq!(
            normalised_weight(REGENERATE_FILE_DATA_JOB_BLURHASH),
            NORMALISED_BIG_JOB_WEIGHT
        );
        assert_eq!(normalised_weight(REGENERATE_FILE_DATA_JOB_FIX_PERMISSIONS), 1);
    }
}

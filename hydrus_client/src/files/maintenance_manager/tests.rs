use super::*;
use crate::files::maintenance::{
    REGENERATE_FILE_DATA_JOB_BLURHASH, REGENERATE_FILE_DATA_JOB_FIX_PERMISSIONS,
};

#[test]
fn jobs_are_counted_per_type() {
    let manager = FilesMaintenanceManager::new();
    manager.schedule_job_now(vec![1], REGENERATE_FILE_DATA_JOB_FIX_PERMISSIONS);
    manager.schedule_job_now(vec![2], REGENERATE_FILE_DATA_JOB_FIX_PERMISSIONS);
    manager.schedule_job_now(vec![1], REGENERATE_FILE_DATA_JOB_BLURHASH);

    let counts = manager.get_job_counts();

    assert_eq!(counts[&REGENERATE_FILE_DATA_JOB_FIX_PERMISSIONS], 2);
    assert_eq!(counts[&REGENERATE_FILE_DATA_JOB_BLURHASH], 1);
}

#[test]
fn scheduled_for_future_is_not_due() {
    let manager = FilesMaintenanceManager::new();
    manager.schedule_job(vec![1], REGENERATE_FILE_DATA_JOB_FIX_PERMISSIONS, i64::MAX);

    assert!(manager.get_jobs(None).is_empty());
}

#[test]
fn running_a_job_removes_it_on_success() {
    let manager = FilesMaintenanceManager::new();
    manager.schedule_job_now(vec![1], REGENERATE_FILE_DATA_JOB_FIX_PERMISSIONS);

    let job = manager.run_next(|_| true).unwrap();

    assert_eq!(job.job_type, REGENERATE_FILE_DATA_JOB_FIX_PERMISSIONS);
    assert!(manager.is_empty());
}

#[test]
fn failed_jobs_stay_queued() {
    let manager = FilesMaintenanceManager::new();
    manager.schedule_job_now(vec![1], REGENERATE_FILE_DATA_JOB_FIX_PERMISSIONS);

    manager.run_next(|_| false).unwrap();

    assert!(!manager.is_empty());
}

#[test]
fn cancel_jobs_removes_only_that_type() {
    let manager = FilesMaintenanceManager::new();
    manager.schedule_job_now(vec![1], REGENERATE_FILE_DATA_JOB_FIX_PERMISSIONS);
    manager.schedule_job_now(vec![1], REGENERATE_FILE_DATA_JOB_BLURHASH);

    manager.cancel_jobs(REGENERATE_FILE_DATA_JOB_FIX_PERMISSIONS);

    assert_eq!(
        manager.get_job_counts(),
        BTreeMap::from([(REGENERATE_FILE_DATA_JOB_BLURHASH, 1)])
    );
}

#[test]
fn normalised_weight_uses_job_weights() {
    let manager = FilesMaintenanceManager::new();
    manager.schedule_job_now(vec![1], REGENERATE_FILE_DATA_JOB_BLURHASH);
    manager.schedule_job_now(vec![2], REGENERATE_FILE_DATA_JOB_FIX_PERMISSIONS);

    assert_eq!(manager.get_normalised_weight(), 5);
}

use foundry_records_draft::{ActionEnvelope, RecordsLog};
use foundry_records_sqlite_draft::SqliteRecordsLog;

/// One entry for `tenant_id`, carrying bytes the fold cannot decode — so it
/// counts as lag while unfolded, and as poison once a boot fold consumes it.
pub fn append_for(action_log: &std::path::Path, tenant_id: &str, key: &str) {
    let mut log = SqliteRecordsLog::open(action_log).expect("open the log");
    log.append(
        ActionEnvelope::new(
            tenant_id,
            "ent_alpha",
            "aty_record_write",
            key,
            1,
            b"these bytes are not a canonical action record".to_vec(),
            1_700_000_000_000,
        )
        .expect("a well-formed envelope"),
    )
    .expect("the append succeeds");
}

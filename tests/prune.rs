mod common;

use common::TestEnv;

#[test]
fn prune_succeeds_on_clean_repo() {
    let env = TestEnv::new();
    let output = env.arbor(&["prune"]).output().unwrap();
    assert!(output.status.success());
}

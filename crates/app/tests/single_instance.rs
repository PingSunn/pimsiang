use app::single_instance::InstanceLock;

#[test]
fn a_second_lock_on_the_same_file_is_refused_until_the_first_is_dropped() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("PimSiang/instance.lock");

    let first = InstanceLock::acquire(&path).unwrap();
    assert!(first.is_some());
    assert!(InstanceLock::acquire(&path).unwrap().is_none());

    drop(first);
    assert!(InstanceLock::acquire(&path).unwrap().is_some());
}

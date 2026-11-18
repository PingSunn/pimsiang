use std::cell::RefCell;
use std::collections::HashMap;

use app::model_download::ModelStatus;
use app::model_switch::{Command, Presence, Switcher, delete_build, presence};
use engine::Precision::{self, Fp32, Int8};

#[derive(Default)]
struct Disk(RefCell<HashMap<Precision, Presence>>);

impl Disk {
    fn with(builds: &[(Precision, Presence)]) -> Self {
        Self(RefCell::new(builds.iter().copied().collect()))
    }

    fn set(&self, precision: Precision, presence: Presence) {
        self.0.borrow_mut().insert(precision, presence);
    }

    fn remove(&self, precision: Precision) {
        self.0.borrow_mut().remove(&precision);
    }

    fn look(&self) -> impl Fn(Precision) -> Presence + '_ {
        |precision| {
            self.0
                .borrow()
                .get(&precision)
                .copied()
                .unwrap_or(Presence::Absent)
        }
    }
}

fn downloading() -> ModelStatus {
    ModelStatus::Downloading { done: 1, total: 2 }
}

#[test]
fn a_fresh_install_downloads_the_wanted_build_and_has_nothing_to_use_yet() {
    let disk = Disk::default();

    let (switcher, commands) = Switcher::start(Fp32, &disk.look());

    assert_eq!(
        commands,
        [Command::Download {
            id: 1,
            precision: Fp32
        }]
    );
    assert!(!switcher.usable());
    assert_eq!(switcher.engine_files(), Fp32);
}

#[test]
fn the_first_install_becomes_ready_when_the_download_is_complete() {
    let disk = Disk::default();
    let (mut switcher, _) = Switcher::start(Fp32, &disk.look());
    assert_eq!(
        switcher.download_event(1, downloading(), &disk.look()),
        [Command::Report(downloading())]
    );

    disk.set(Fp32, Presence::Complete);
    let commands = switcher.download_event(1, ModelStatus::Ready, &disk.look());

    assert_eq!(
        commands,
        [Command::Ready, Command::Report(ModelStatus::Ready)]
    );
    assert!(switcher.usable());
}

#[test]
fn an_existing_light_install_keeps_working_while_accurate_downloads() {
    let disk = Disk::with(&[(Int8, Presence::Complete)]);

    let (switcher, commands) = Switcher::start(Fp32, &disk.look());

    assert_eq!(
        commands,
        [Command::Download {
            id: 1,
            precision: Fp32
        }]
    );
    assert!(switcher.usable());
    assert_eq!(switcher.engine_files(), Int8);
}

#[test]
fn the_engine_switches_only_after_the_download_is_complete_and_the_old_build_goes_after_the_switch()
{
    let disk = Disk::with(&[(Int8, Presence::Complete)]);
    let (mut switcher, _) = Switcher::start(Fp32, &disk.look());

    disk.set(Fp32, Presence::Partial);
    assert_eq!(
        switcher.download_event(1, downloading(), &disk.look()),
        [Command::Report(downloading())]
    );
    assert_eq!(switcher.engine_files(), Int8);

    disk.set(Fp32, Presence::Complete);
    let commands = switcher.download_event(1, ModelStatus::Ready, &disk.look());
    assert_eq!(
        commands,
        [Command::Use(Fp32), Command::Report(ModelStatus::Ready)]
    );
    assert_eq!(switcher.engine_files(), Fp32);

    assert_eq!(
        switcher.switched(Fp32, &disk.look()),
        [Command::Delete(Int8)]
    );
}

#[test]
fn a_download_that_ends_without_every_file_is_a_failure_and_changes_nothing() {
    let disk = Disk::with(&[(Int8, Presence::Complete), (Fp32, Presence::Partial)]);
    let (mut switcher, _) = Switcher::start(Fp32, &disk.look());

    let commands = switcher.download_event(1, ModelStatus::Ready, &disk.look());

    assert!(matches!(
        commands.as_slice(),
        [Command::Report(ModelStatus::Failed(_))]
    ));
    assert_eq!(switcher.engine_files(), Int8);
}

#[test]
fn a_failed_download_keeps_the_old_build_and_surfaces_the_error() {
    let disk = Disk::with(&[(Int8, Presence::Complete)]);
    let (mut switcher, _) = Switcher::start(Fp32, &disk.look());

    let failed = ModelStatus::Failed("the server answered 500".to_owned());
    let commands = switcher.download_event(1, failed.clone(), &disk.look());

    assert_eq!(commands, [Command::Report(failed)]);
    assert!(switcher.usable());
    assert_eq!(switcher.engine_files(), Int8);
    assert!(!commands.iter().any(|c| matches!(c, Command::Delete(_))));
}

#[test]
fn retrying_after_a_failure_downloads_again() {
    let disk = Disk::with(&[(Int8, Presence::Complete)]);
    let (mut switcher, _) = Switcher::start(Fp32, &disk.look());
    switcher.download_event(1, ModelStatus::Failed("x".to_owned()), &disk.look());

    let commands = switcher.retry(&disk.look());

    assert_eq!(
        commands,
        [Command::Download {
            id: 2,
            precision: Fp32
        }]
    );
}

#[test]
fn events_from_a_cancelled_download_are_ignored() {
    let disk = Disk::with(&[(Int8, Presence::Complete)]);
    let (mut switcher, _) = Switcher::start(Fp32, &disk.look());

    let commands = switcher.want(Int8, &disk.look());
    assert_eq!(
        commands,
        [
            Command::Cancel { id: 1 },
            Command::Report(ModelStatus::Ready)
        ]
    );

    disk.set(Fp32, Presence::Complete);
    assert!(
        switcher
            .download_event(1, ModelStatus::Ready, &disk.look())
            .is_empty()
    );
    assert_eq!(switcher.engine_files(), Int8);
}

#[test]
fn switching_back_while_downloading_keeps_the_partial_download() {
    let disk = Disk::with(&[(Int8, Presence::Complete), (Fp32, Presence::Partial)]);
    let (mut switcher, _) = Switcher::start(Fp32, &disk.look());

    let commands = switcher.want(Int8, &disk.look());

    assert!(!commands.iter().any(|c| matches!(c, Command::Delete(_))));
}

#[test]
fn choosing_light_from_a_complete_accurate_install_downloads_light_and_then_removes_accurate() {
    let disk = Disk::with(&[(Fp32, Presence::Complete)]);
    let (mut switcher, commands) = Switcher::start(Fp32, &disk.look());
    assert_eq!(commands, [Command::Report(ModelStatus::Ready)]);

    let commands = switcher.want(Int8, &disk.look());
    assert_eq!(
        commands,
        [Command::Download {
            id: 1,
            precision: Int8
        }]
    );
    assert_eq!(switcher.engine_files(), Fp32);

    disk.set(Int8, Presence::Complete);
    let commands = switcher.download_event(1, ModelStatus::Ready, &disk.look());
    assert_eq!(
        commands,
        [Command::Use(Int8), Command::Report(ModelStatus::Ready)]
    );

    assert_eq!(
        switcher.switched(Int8, &disk.look()),
        [Command::Delete(Fp32)]
    );
    disk.remove(Fp32);
    assert!(switcher.switched(Int8, &disk.look()).is_empty());
}

#[test]
fn the_unused_build_is_deleted_at_start_when_the_wanted_one_is_complete() {
    let disk = Disk::with(&[(Int8, Presence::Complete), (Fp32, Presence::Complete)]);

    let (switcher, commands) = Switcher::start(Fp32, &disk.look());

    assert_eq!(
        commands,
        [Command::Delete(Int8), Command::Report(ModelStatus::Ready)]
    );
    assert_eq!(switcher.engine_files(), Fp32);
}

#[test]
fn the_old_build_is_kept_while_another_switch_is_still_waiting() {
    let disk = Disk::with(&[(Int8, Presence::Complete)]);
    let (mut switcher, _) = Switcher::start(Fp32, &disk.look());
    disk.set(Fp32, Presence::Complete);
    switcher.download_event(1, ModelStatus::Ready, &disk.look());

    let commands = switcher.want(Int8, &disk.look());
    assert_eq!(
        commands,
        [Command::Use(Int8), Command::Report(ModelStatus::Ready)]
    );

    assert_eq!(
        switcher.switched(Int8, &disk.look()),
        [Command::Delete(Fp32)]
    );
}

#[test]
fn changing_the_choice_during_the_first_install_downloads_the_other_build() {
    let disk = Disk::default();
    let (mut switcher, _) = Switcher::start(Fp32, &disk.look());

    let commands = switcher.want(Int8, &disk.look());
    assert_eq!(
        commands,
        [
            Command::Cancel { id: 1 },
            Command::Download {
                id: 2,
                precision: Int8
            }
        ]
    );

    disk.set(Int8, Presence::Complete);
    let commands = switcher.download_event(2, ModelStatus::Ready, &disk.look());
    assert_eq!(
        commands,
        [
            Command::Use(Int8),
            Command::Ready,
            Command::Report(ModelStatus::Ready)
        ]
    );
}

#[test]
fn presence_reads_the_disk() {
    let root = tempfile::tempdir().unwrap();
    assert_eq!(presence(root.path(), Fp32), Presence::Absent);

    let dir = app::model_download::build_dir(root.path(), Fp32);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("tokens.txt"), b"x").unwrap();
    assert_eq!(presence(root.path(), Fp32), Presence::Partial);
    assert_eq!(presence(root.path(), Int8), Presence::Absent);
}

#[test]
fn deleting_a_build_removes_only_its_folder() {
    let root = tempfile::tempdir().unwrap();
    for precision in [Fp32, Int8] {
        let dir = app::model_download::build_dir(root.path(), precision);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("tokens.txt"), b"x").unwrap();
    }

    delete_build(root.path(), Int8).unwrap();
    delete_build(root.path(), Int8).unwrap();

    assert_eq!(presence(root.path(), Int8), Presence::Absent);
    assert_eq!(presence(root.path(), Fp32), Presence::Partial);
}

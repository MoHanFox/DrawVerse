use paint_storage::{Config, ScratchSpace};
use std::{
    fs,
    io::{BufRead, BufReader, Read, Seek, SeekFrom, Write},
    path::PathBuf,
    process::{Child, Command, Stdio},
};
fn space(path: &std::path::Path) -> std::sync::Arc<ScratchSpace> {
    ScratchSpace::new(Config {
        directory: path.into(),
        min_free_bytes: 0,
    })
}
#[test]
fn child_holder() {
    let Some(path) = std::env::var_os("DRAWVERSE_TEST_SCRATCH_CHILD") else {
        return;
    };
    let manager = space(&PathBuf::from(path));
    let mut file = manager.tempfile("pages").unwrap();
    file.write_all(b"retained pixels").unwrap();
    file.flush().unwrap();
    // Parent terminates this helper without destructors; OS releases the lease.
    println!("LEASE_READY={}", file.path().display());
    std::io::stdout().flush().unwrap();
    let mut byte = [0];
    let _ = std::io::stdin().read(&mut byte);
}
struct Helper(Child);
impl Drop for Helper {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}
fn helper(path: &std::path::Path) -> (Helper, PathBuf) {
    let mut command = Command::new(std::env::current_exe().unwrap());
    command
        .args(["--exact", "child_holder", "--nocapture"])
        .env("DRAWVERSE_TEST_SCRATCH_CHILD", path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
    let mut child = Helper(command.spawn().unwrap());
    let mut reader = BufReader::new(child.0.stdout.take().unwrap());
    loop {
        let mut line = String::new();
        assert!(
            reader.read_line(&mut line).unwrap() > 0,
            "helper failed before readiness"
        );
        if let Some(path) = line.trim().strip_prefix("LEASE_READY=") {
            return (child, PathBuf::from(path));
        }
    }
}
#[test]
fn active_process_is_preserved_and_terminated_process_is_cleaned() {
    let base = tempfile::Builder::new()
        .prefix("绘画-暂存-")
        .tempdir()
        .unwrap();
    let (mut child, path) = helper(base.path());
    let run = path.parent().unwrap().to_path_buf();
    let active = space(base.path());
    let report = active.inspect().unwrap();
    assert_eq!(report.removed_runs, 0);
    assert_eq!(report.skipped_runs, 1);
    assert!(path.is_file());
    drop(active);
    child.0.kill().unwrap();
    child.0.wait().unwrap();
    let fresh = space(base.path());
    let report = fresh.inspect().unwrap();
    assert_eq!(report.removed_runs, 1);
    assert_eq!(report.removed_bytes, 15);
    assert!(!run.exists());
}
#[test]
fn files_keep_manager_alive_and_normal_drop_removes_only_own_files() {
    let base = tempfile::tempdir().unwrap();
    let manager = space(base.path());
    let file = manager.tempfile("history").unwrap();
    let path = file.path().to_path_buf();
    let run = path.parent().unwrap().to_path_buf();
    drop(manager);
    assert!(path.is_file());
    drop(file);
    assert!(!run.exists());
    assert!(base
        .path()
        .join(".drawverse-scratch-v1/.maintenance")
        .is_file());
}
#[test]
fn unknown_contents_and_invalid_lease_are_not_removed() {
    let base = tempfile::tempdir().unwrap();
    let (_child, path) = helper(base.path());
    fs::write(
        path.parent().unwrap().join("my-picture.png"),
        b"user material",
    )
    .unwrap();
    drop(_child);
    let manager = space(base.path());
    assert_eq!(manager.inspect().unwrap().removed_runs, 0);
    assert!(path.is_file());
    assert_eq!(
        fs::read(path.parent().unwrap().join("my-picture.png")).unwrap(),
        b"user material"
    );
    let root = path.parent().unwrap().parent().unwrap();
    let unknown = root.join("run-aaaaaaaaaaaaaaaa");
    fs::create_dir(&unknown).unwrap();
    fs::write(unknown.join(".lease"), b"not DrawVerse").unwrap();
    fs::write(unknown.join("pages-aaaaaaaaaaaaaaaa"), b"other").unwrap();
    let other = space(base.path());
    assert_eq!(other.inspect().unwrap().removed_runs, 0);
    assert!(unknown.join("pages-aaaaaaaaaaaaaaaa").exists());
}
#[test]
fn unmarked_namespace_and_empty_marker_are_not_adopted() {
    for empty_marker in [false, true] {
        let base = tempfile::tempdir().unwrap();
        let root = base.path().join(".drawverse-scratch-v1");
        fs::create_dir(&root).unwrap();
        let target = root.join(if empty_marker {
            ".maintenance"
        } else {
            "user-file"
        });
        fs::write(&target, b"").unwrap();
        assert!(space(base.path()).inspect().is_err());
        assert_eq!(fs::read(target).unwrap(), b"");
    }
}
#[test]
fn reserve_and_growth_fail_before_creating_or_writing_data() {
    let base = tempfile::tempdir().unwrap();
    let manager = ScratchSpace::new(Config {
        directory: base.path().into(),
        min_free_bytes: u64::MAX,
    });
    assert!(manager.inspect().is_err());
    assert!(!base.path().join(".drawverse-scratch-v1").exists());
    let mut file = space(base.path()).tempfile("pages").unwrap();
    file.write_all(b"original").unwrap();
    file.flush().unwrap();
    // Request a sparse extent larger than the entire available volume. No huge allocation.
    let available = fs2::available_space(base.path()).unwrap();
    file.seek(SeekFrom::Start(available + 1024 * 1024)).unwrap();
    assert!(file.write_all(b"x").is_err());
    assert_eq!(fs::read(file.path()).unwrap(), b"original");
}
#[test]
fn directory_links_and_linked_namespace_are_skipped() {
    let base = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    let manager = space(base.path());
    manager.inspect().unwrap();
    let link = base
        .path()
        .join(".drawverse-scratch-v1/run-aaaaaaaaaaaaaaaa");
    #[cfg(unix)]
    std::os::unix::fs::symlink(outside.path(), &link).unwrap();
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        let status = Command::new("cmd")
            .args(["/d", "/c", "mklink", "/J"])
            .arg(link.to_string_lossy().replace('/', "\\"))
            .arg(outside.path().to_string_lossy().replace('/', "\\"))
            .creation_flags(0x08000000)
            .stdout(Stdio::null())
            .status()
            .unwrap();
        assert!(status.success());
    }
    fs::write(outside.path().join("pages-aaaaaaaaaaaaaaaa"), b"outside").unwrap();
    fs::write(outside.path().join(".lease"), b"DrawVerse scratch run v1\n").unwrap();
    let other = space(base.path());
    assert_eq!(other.inspect().unwrap().removed_runs, 0);
    assert!(outside.path().join("pages-aaaaaaaaaaaaaaaa").is_file());
    fs::remove_dir(&link)
        .or_else(|_| fs::remove_file(&link))
        .unwrap();
    drop(other);
    drop(manager);
    let root = base.path().join(".drawverse-scratch-v1");
    fs::remove_file(root.join(".maintenance")).unwrap();
    fs::remove_dir(&root).unwrap();
    #[cfg(unix)]
    std::os::unix::fs::symlink(outside.path(), &root).unwrap();
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        assert!(Command::new("cmd")
            .args(["/d", "/c", "mklink", "/J"])
            .arg(root.to_string_lossy().replace('/', "\\"))
            .arg(outside.path().to_string_lossy().replace('/', "\\"))
            .creation_flags(0x08000000)
            .stdout(Stdio::null())
            .status()
            .unwrap()
            .success());
    }
    assert!(space(base.path()).inspect().is_err());
    assert!(outside.path().join("pages-aaaaaaaaaaaaaaaa").is_file());
    fs::remove_dir(&root)
        .or_else(|_| fs::remove_file(&root))
        .unwrap();
}

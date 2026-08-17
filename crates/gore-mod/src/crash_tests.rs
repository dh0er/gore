use super::{
    bak_path, manager_transaction_root, probe_manager_install_recovery, read_record, record_path,
    recover_manager_install, DeployPhase, ManagerInstallRecoveryOutcome,
    ManagerInstallRecoveryReadiness, ManagerMutationOperation, RecoveryTransactionStep,
    MANAGER_CRASH_TEST_NONCE_ENV, MANAGER_CRASH_TEST_POINT_ENV, MANAGER_CRASH_TEST_READY_ENV,
    MANAGER_CRASH_TEST_ROOT_ENV,
};
use crate::mgr::apply::{apply_loadout, undeploy_all};
use crate::mgr::model::{ComponentInfo, ModEntryMeta, ModKind, META_FILE};
use crate::mgr::{Loadout, LoadoutEntry};
use std::collections::BTreeMap;
use std::ffi::OsStr;
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const CHILD_FIXTURE_ENV: &str = "GORE_MOD_CRASH_TEST_FIXTURE";
const CHILD_MODE_ENV: &str = "GORE_MOD_CRASH_TEST_CHILD_MODE";
const CHILD_EXPECTED_GUARD_ENV: &str = "GORE_MOD_CRASH_TEST_EXPECTED_GUARD";
const CHILD_GUARD_OWNER_ENV: &str = "GORE_MOD_CRASH_TEST_GUARD_OWNER";
const CHILD_DROP_WITNESS_ENV: &str = "GORE_MOD_CRASH_TEST_DROP_WITNESS";
const CHILD_ARMED_ENV: &str = "GORE_MOD_CRASH_TEST_ARMED";
const CHILD_GATE_ENV: &str = "GORE_MOD_CRASH_TEST_GATE";
const CHILD_OUTCOME_ENV: &str = "GORE_MOD_CRASH_TEST_OUTCOME";

const MODE_APPLY: &str = "apply";
const MODE_REAPPLY: &str = "reapply";
const MODE_UNDEPLOY: &str = "undeploy";
const MODE_RECOVER: &str = "recover";
const MODE_RECOVER_ONCE: &str = "recover_once";
const MODE_HOLD_GUARD: &str = "hold_guard";

const OLD_ID: &str = "crash-old";
const NEW_ID: &str = "crash-new";
const UE4SS_NAME: &str = "CrashHarness";
const LIVE_A_REL: &str = "G1R/Content/Movies/CrashHarnessA.bk2";
const LIVE_B_REL: &str = "G1R/Content/Movies/CrashHarnessB.bk2";
const PAK_REL: &str = "crash_P.pak";
const PAK_NAME: &str = "zzz_gm000_crash_P.pak";
const PRISTINE_A: &[u8] = b"pristine-live-a";
const PRISTINE_B: &[u8] = b"pristine-live-b";
const OLD_A: &[u8] = b"old-manager-live-a";
const OLD_B: &[u8] = b"old-manager-live-b";
const OLD_UE4SS: &[u8] = b"old-manager-ue4ss";
const OLD_PAK: &[u8] = b"old-manager-pak";
const NEW_A: &[u8] = b"new-manager-live-a";
const NEW_B: &[u8] = b"new-manager-live-b";
const NEW_UE4SS: &[u8] = b"new-manager-ue4ss";
const NEW_PAK: &[u8] = b"new-manager-pak";

const CHILD_TIMEOUT: Duration = Duration::from_secs(20);
const POLL_INTERVAL: Duration = Duration::from_millis(10);

#[derive(Clone, Copy, Debug)]
enum ApplyScenario {
    Initial,
    Reapply,
}

#[derive(Clone, Copy, Debug)]
enum UndeployCrashStage {
    PreMutation,
    EarlyRecord,
    FirstRestore,
    RecordRemoved,
}

struct CrashFixture {
    _temp: tempfile::TempDir,
    base: PathBuf,
    game: PathBuf,
    library: PathBuf,
    control: PathBuf,
    initial_game_tree: BTreeMap<PathBuf, GameTreeEntry>,
}

#[derive(Debug, PartialEq, Eq)]
enum GameTreeEntry {
    Directory,
    File(Vec<u8>),
}

impl CrashFixture {
    fn new() -> Self {
        let temp = crate::canonical_tempfile::tempdir().expect("create crash-test temp directory");
        let base = fs::canonicalize(temp.path()).expect("canonicalize crash-test temp directory");
        let game = base.join("game");
        let library = base.join("library");
        let control = base.join("control");
        for directory in [
            game.join("G1R/Binaries/Win64/ue4ss/Mods"),
            game.join("G1R/Content/Movies"),
            game.join("G1R/Content/Paks/~mods"),
            library.clone(),
            control.clone(),
        ] {
            fs::create_dir_all(directory).expect("create crash-test fixture directory");
        }
        fs::write(game.join(LIVE_A_REL), PRISTINE_A).expect("write first pristine live file");
        fs::write(game.join(LIVE_B_REL), PRISTINE_B).expect("write second pristine live file");
        write_mod(
            &library,
            OLD_ID,
            "Old crash fixture",
            OLD_A,
            OLD_B,
            OLD_UE4SS,
            OLD_PAK,
            "2026-08-17T00:00:00Z",
        );
        write_mod(
            &library,
            NEW_ID,
            "New crash fixture",
            NEW_A,
            NEW_B,
            NEW_UE4SS,
            NEW_PAK,
            "2026-08-17T00:00:01Z",
        );
        let game = fs::canonicalize(game).expect("canonicalize crash-test game root");
        let library = fs::canonicalize(library).expect("canonicalize crash-test library");
        let initial_game_tree = snapshot_game_tree(&game);
        Self {
            _temp: temp,
            base,
            game,
            library,
            control,
            initial_game_tree,
        }
    }

    fn live_a(&self) -> PathBuf {
        self.game.join(LIVE_A_REL)
    }

    fn live_b(&self) -> PathBuf {
        self.game.join(LIVE_B_REL)
    }

    fn ue4ss(&self) -> PathBuf {
        self.game
            .join("G1R/Binaries/Win64/ue4ss/Mods")
            .join(format!("gm000_{UE4SS_NAME}"))
    }

    fn pak(&self) -> PathBuf {
        self.game.join("G1R/Content/Paks/~mods").join(PAK_NAME)
    }

    fn lock(&self) -> PathBuf {
        self.game.join(".gore-install-mutation.lock")
    }

    fn control_path(&self, kind: &str, nonce: &str) -> PathBuf {
        self.control.join(format!("{kind}-{nonce}"))
    }
}

struct ChildFixture {
    game: PathBuf,
    library: PathBuf,
}

impl ChildFixture {
    fn from_env() -> Self {
        let base = PathBuf::from(
            std::env::var_os(CHILD_FIXTURE_ENV).expect("child fixture root environment"),
        );
        Self {
            game: base.join("game"),
            library: base.join("library"),
        }
    }
}

fn write_mod(
    library: &Path,
    id: &str,
    name: &str,
    live_a: &[u8],
    live_b: &[u8],
    ue4ss: &[u8],
    pak: &[u8],
    imported_at: &str,
) {
    let directory = library.join(id);
    fs::create_dir_all(directory.join("files")).expect("create loose-file payload directory");
    fs::create_dir_all(directory.join("ue4ss/crash")).expect("create UE4SS payload directory");
    let manifest = BTreeMap::from([
        (LIVE_A_REL.to_owned(), "files/a.bin".to_owned()),
        (LIVE_B_REL.to_owned(), "files/b.bin".to_owned()),
    ]);
    fs::write(
        directory.join("files/manifest.json"),
        serde_json::to_vec_pretty(&manifest).expect("serialize loose-file manifest"),
    )
    .expect("write loose-file manifest");
    fs::write(directory.join("files/a.bin"), live_a).expect("write first loose-file payload");
    fs::write(directory.join("files/b.bin"), live_b).expect("write second loose-file payload");
    fs::write(directory.join("ue4ss/crash/main.lua"), ue4ss).expect("write UE4SS payload");
    fs::write(directory.join(PAK_REL), pak).expect("write additive pak payload");
    let metadata = ModEntryMeta {
        id: id.to_owned(),
        kind: ModKind::Goremod,
        name: name.to_owned(),
        version: String::new(),
        author: String::new(),
        imported_at: imported_at.to_owned(),
        source: String::new(),
        components: vec![
            ComponentInfo::Ue4ssLua {
                name: UE4SS_NAME.to_owned(),
                rel: "ue4ss/crash".to_owned(),
                targets: Vec::new(),
                opaque: false,
            },
            ComponentInfo::FilePatch {
                rel: "files".to_owned(),
                targets: vec![LIVE_A_REL.to_owned(), LIVE_B_REL.to_owned()],
            },
            ComponentInfo::LoosePak {
                rel: PAK_REL.to_owned(),
                targets: Vec::new(),
            },
        ],
    };
    fs::write(
        directory.join(META_FILE),
        serde_json::to_vec_pretty(&metadata).expect("serialize crash-test sidecar"),
    )
    .expect("write crash-test sidecar");
}

fn loadout(id: &str) -> Loadout {
    Loadout {
        format: 1,
        entries: vec![LoadoutEntry {
            id: id.to_owned(),
            enabled: true,
        }],
    }
}

struct DropWitness {
    path: PathBuf,
    nonce: String,
}

impl DropWitness {
    fn from_env() -> Option<Self> {
        Some(Self {
            path: PathBuf::from(std::env::var_os(CHILD_DROP_WITNESS_ENV)?),
            nonce: std::env::var(MANAGER_CRASH_TEST_NONCE_ENV)
                .expect("drop witness requires crash-test nonce"),
        })
    }
}

impl Drop for DropWitness {
    fn drop(&mut self) {
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&self.path)
            .expect("create child drop witness");
        writeln!(file, "nonce={}", self.nonce).expect("write child drop witness");
        file.sync_all().expect("sync child drop witness");
        super::sync_parent_directory(self.path.parent().expect("child drop witness has parent"))
            .expect("sync child drop-witness directory");
    }
}

/// One ignored worker is reused for every subprocess role. Parent tests always select it with
/// `--ignored --exact`, so a hard-kill checkpoint cannot accidentally run unrelated unit tests.
#[test]
#[ignore = "subprocess worker; parent hard-kill tests invoke it explicitly"]
fn manager_crash_child() {
    let Some(mode) = std::env::var_os(CHILD_MODE_ENV) else {
        return;
    };
    let mode = mode.to_string_lossy();
    let _drop_witness = DropWitness::from_env();
    let fixture = ChildFixture::from_env();
    match mode.as_ref() {
        MODE_APPLY | MODE_REAPPLY => {
            let result = apply_loadout(&fixture.game, &fixture.library, &loadout(NEW_ID));
            panic!(
                "targeted {mode} returned without reaching its hard-kill checkpoint: {result:?}"
            );
        }
        MODE_UNDEPLOY => {
            let result = undeploy_all(&fixture.game);
            panic!(
                "targeted Manager undeploy returned without reaching its hard-kill checkpoint: \
                 {result:?}"
            );
        }
        MODE_RECOVER => {
            let expected =
                std::env::var(CHILD_EXPECTED_GUARD_ENV).expect("recovery child expected guard id");
            let result = recover_manager_install(&fixture.game, &expected);
            panic!(
                "targeted recovery returned without reaching its hard-kill checkpoint: {result:?}"
            );
        }
        MODE_HOLD_GUARD => {
            let owner = std::env::var(CHILD_GUARD_OWNER_ENV).expect("guard child owner");
            let _guard = gore_as::compile::InstallMutationGuard::acquire(&fixture.game, &owner)
                .expect("acquire child install-mutation guard");
            publish_child_ready("guard.active");
            loop {
                std::thread::park();
            }
        }
        MODE_RECOVER_ONCE => recover_once_child(&fixture),
        other => panic!("unknown crash-test child mode {other:?}"),
    }
}

fn recover_once_child(fixture: &ChildFixture) {
    let expected =
        std::env::var(CHILD_EXPECTED_GUARD_ENV).expect("one-shot recovery expected guard id");
    let armed = PathBuf::from(
        std::env::var_os(CHILD_ARMED_ENV).expect("one-shot recovery armed witness path"),
    );
    let gate =
        PathBuf::from(std::env::var_os(CHILD_GATE_ENV).expect("one-shot recovery start-gate path"));
    let outcome =
        PathBuf::from(std::env::var_os(CHILD_OUTCOME_ENV).expect("one-shot recovery outcome path"));
    durable_create_new(&armed, b"armed\n");
    let deadline = Instant::now() + CHILD_TIMEOUT;
    while !gate.exists() {
        assert!(
            Instant::now() < deadline,
            "timed out waiting for one-shot recovery start gate {}",
            gate.display()
        );
        std::thread::sleep(POLL_INTERVAL);
    }
    let result = recover_manager_install(&fixture.game, &expected)
        .expect("one-shot Manager recovery call failed");
    durable_create_new(
        &outcome,
        &serde_json::to_vec(&result).expect("serialize one-shot recovery result"),
    );
}

fn publish_child_ready(point: &str) {
    let ready = PathBuf::from(
        std::env::var_os(MANAGER_CRASH_TEST_READY_ENV).expect("child ready witness path"),
    );
    let nonce = std::env::var(MANAGER_CRASH_TEST_NONCE_ENV).expect("child ready witness nonce");
    durable_create_new(&ready, format!("point={point}\nnonce={nonce}\n").as_bytes());
}

fn durable_create_new(path: &Path, bytes: &[u8]) {
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .unwrap_or_else(|error| panic!("creating {}: {error}", path.display()));
    file.write_all(bytes)
        .unwrap_or_else(|error| panic!("writing {}: {error}", path.display()));
    file.sync_all()
        .unwrap_or_else(|error| panic!("syncing {}: {error}", path.display()));
    super::sync_parent_directory(path.parent().expect("durable child file has parent"))
        .unwrap_or_else(|error| panic!("syncing parent of {}: {error}", path.display()));
}

fn child_command(fixture: &CrashFixture, mode: &str) -> Command {
    let mut command = Command::new(std::env::current_exe().expect("locate gore-mod test binary"));
    command
        .arg("--ignored")
        .arg("--exact")
        .arg("crash_tests::manager_crash_child")
        .arg("--test-threads=1")
        .env_remove(MANAGER_CRASH_TEST_ROOT_ENV)
        .env_remove(MANAGER_CRASH_TEST_POINT_ENV)
        .env_remove(MANAGER_CRASH_TEST_READY_ENV)
        .env_remove(MANAGER_CRASH_TEST_NONCE_ENV)
        .env_remove(CHILD_EXPECTED_GUARD_ENV)
        .env_remove(CHILD_GUARD_OWNER_ENV)
        .env_remove(CHILD_DROP_WITNESS_ENV)
        .env_remove(CHILD_ARMED_ENV)
        .env_remove(CHILD_GATE_ENV)
        .env_remove(CHILD_OUTCOME_ENV)
        .env(CHILD_FIXTURE_ENV, &fixture.base)
        .env(CHILD_MODE_ENV, mode)
        .stdout(Stdio::null())
        .stderr(Stdio::piped());
    command
}

fn hard_kill_at_point(
    fixture: &CrashFixture,
    mode: &str,
    point: &str,
    expected_guard: Option<&str>,
    guard_owner: Option<&str>,
) {
    let nonce = unique_nonce();
    let ready = fixture.control_path("ready", &nonce);
    let drop_witness = fixture.control_path("drop", &nonce);
    let mut command = child_command(fixture, mode);
    command
        .env(MANAGER_CRASH_TEST_ROOT_ENV, &fixture.game)
        .env(MANAGER_CRASH_TEST_POINT_ENV, point)
        .env(MANAGER_CRASH_TEST_READY_ENV, &ready)
        .env(MANAGER_CRASH_TEST_NONCE_ENV, &nonce)
        .env(CHILD_DROP_WITNESS_ENV, &drop_witness);
    if let Some(expected_guard) = expected_guard {
        command.env(CHILD_EXPECTED_GUARD_ENV, expected_guard);
    }
    if let Some(owner) = guard_owner {
        command.env(CHILD_GUARD_OWNER_ENV, owner);
    }
    let mut child = command.spawn().expect("spawn crash-test child");
    wait_for_ready(&mut child, &ready, point, &nonce);
    assert!(
        child.try_wait().expect("probe parked child").is_none(),
        "child exited after publishing its ready witness"
    );
    assert!(
        fixture.lock().is_file(),
        "parked child did not retain the install-mutation record"
    );
    assert_eq!(
        probe_manager_install_recovery(&fixture.game),
        ManagerInstallRecoveryReadiness::Active,
        "the retained OS lock must classify the child as active before termination"
    );
    assert_eq!(
        recover_manager_install(&fixture.game, "active-manager-probe")
            .expect("probe active Manager recovery"),
        ManagerInstallRecoveryOutcome::Busy,
        "recovery must not adopt a live child guard"
    );
    child.kill().expect("hard-kill parked crash-test child");
    let status = child.wait().expect("wait for hard-killed crash-test child");
    let stderr = take_child_stderr(&mut child);
    assert!(
        !status.success(),
        "hard-killed child unexpectedly exited successfully; stderr: {stderr}"
    );
    assert!(
        !drop_witness.exists(),
        "child drop witness exists after hard kill; a destructor or unwind path ran"
    );
    assert!(
        fixture.lock().is_file(),
        "hard kill removed the persistent install-mutation record"
    );
}

fn wait_for_ready(child: &mut Child, ready: &Path, point: &str, nonce: &str) {
    let expected = format!("point={point}\nnonce={nonce}\n");
    let deadline = Instant::now() + CHILD_TIMEOUT;
    loop {
        if let Ok(contents) = fs::read_to_string(ready) {
            if contents == expected {
                return;
            }
        }
        if let Some(status) = child.try_wait().expect("poll crash-test child") {
            let stderr = take_child_stderr(child);
            panic!("child exited before ready witness for {point} ({status}); stderr: {stderr}");
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let status = child.wait().expect("wait for timed-out crash-test child");
            let stderr = take_child_stderr(child);
            let observed = fs::read_to_string(ready).unwrap_or_else(|error| format!("<{error}>"));
            panic!(
                "timed out waiting for {point}; child={status}, ready={observed:?}, stderr={stderr}"
            );
        }
        std::thread::sleep(POLL_INTERVAL);
    }
}

fn wait_for_path(child: &mut Child, path: &Path, label: &str) {
    let deadline = Instant::now() + CHILD_TIMEOUT;
    loop {
        if path.exists() {
            return;
        }
        if let Some(status) = child.try_wait().expect("poll child readiness") {
            let stderr = take_child_stderr(child);
            panic!("{label} child exited before readiness ({status}); stderr: {stderr}");
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let status = child.wait().expect("wait for timed-out child");
            let stderr = take_child_stderr(child);
            panic!("timed out waiting for {label}; child={status}, stderr={stderr}");
        }
        std::thread::sleep(POLL_INTERVAL);
    }
}

fn wait_for_success(child: &mut Child, label: &str) {
    let deadline = Instant::now() + CHILD_TIMEOUT;
    loop {
        if let Some(status) = child.try_wait().expect("poll one-shot child") {
            let stderr = take_child_stderr(child);
            assert!(
                status.success(),
                "{label} failed ({status}); stderr: {stderr}"
            );
            return;
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let status = child.wait().expect("wait for timed-out one-shot child");
            let stderr = take_child_stderr(child);
            panic!("{label} timed out ({status}); stderr: {stderr}");
        }
        std::thread::sleep(POLL_INTERVAL);
    }
}

fn take_child_stderr(child: &mut Child) -> String {
    let mut stderr = String::new();
    if let Some(mut pipe) = child.stderr.take() {
        pipe.read_to_string(&mut stderr)
            .expect("read crash-test child stderr");
    }
    stderr
}

fn unique_nonce() -> String {
    static SEQUENCE: AtomicU64 = AtomicU64::new(1);
    let sequence = SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    format!("{}-{nanos}-{sequence}", std::process::id())
}

fn interrupt_manager_operation(
    fixture: &CrashFixture,
    mode: &str,
    point: &str,
    expected_guard: Option<&str>,
) -> String {
    hard_kill_at_point(fixture, mode, point, expected_guard, None);
    match probe_manager_install_recovery(&fixture.game) {
        ManagerInstallRecoveryReadiness::AbandonedManager { guard_id } => {
            if let Some(expected) = expected_guard {
                assert_eq!(guard_id, expected, "recovery changed the bound guard id");
            }
            guard_id
        }
        readiness => panic!("hard-killed Manager child was not recoverable: {readiness:?}"),
    }
}

fn run_apply_crash_case(
    point: &str,
    scenario: ApplyScenario,
    expected: ManagerInstallRecoveryOutcome,
) {
    let fixture = CrashFixture::new();
    let mode = match scenario {
        ApplyScenario::Initial => MODE_APPLY,
        ApplyScenario::Reapply => {
            let report = apply_loadout(&fixture.game, &fixture.library, &loadout(OLD_ID))
                .expect("prepare old Manager deployment");
            assert_eq!(report.applied, vec!["Old crash fixture"]);
            assert_applied(&fixture, OLD_A, OLD_B, OLD_UE4SS, OLD_PAK);
            MODE_REAPPLY
        }
    };
    let guard_id = interrupt_manager_operation(&fixture, mode, point, None);
    let recovered = recover_manager_install(&fixture.game, &guard_id)
        .expect("recover hard-killed Manager apply");
    assert_eq!(
        recovered, expected,
        "unexpected recovery outcome at {point}"
    );
    assert_eq!(
        recover_manager_install(&fixture.game, &guard_id)
            .expect("repeat Manager recovery for idempotence"),
        ManagerInstallRecoveryOutcome::AlreadyClean,
        "second recovery was not idempotent at {point}"
    );
    if expected == ManagerInstallRecoveryOutcome::CompletedApplyPreserved {
        assert_applied(&fixture, NEW_A, NEW_B, NEW_UE4SS, NEW_PAK);
        assert!(
            undeploy_all(&fixture.game).expect("reset completed crash-point deployment"),
            "completed crash-point deployment was not present for reset"
        );
    }
    assert_pristine(&fixture);
}

fn run_recovery_crash_case(point: &str, expected: ManagerInstallRecoveryOutcome) {
    let fixture = CrashFixture::new();
    let guard_id =
        interrupt_manager_operation(&fixture, MODE_APPLY, "apply.between_live_writes", None);
    let recovered_guard =
        interrupt_manager_operation(&fixture, MODE_RECOVER, point, Some(&guard_id));
    assert_eq!(recovered_guard, guard_id);
    assert_eq!(
        recover_manager_install(&fixture.game, &guard_id)
            .expect("resume hard-killed Manager recovery"),
        expected,
        "unexpected resumed recovery outcome at {point}"
    );
    assert_eq!(
        recover_manager_install(&fixture.game, &guard_id)
            .expect("repeat resumed recovery for idempotence"),
        ManagerInstallRecoveryOutcome::AlreadyClean
    );
    assert_pristine(&fixture);
}

fn assert_deployed_crash_payload(fixture: &CrashFixture) {
    assert_eq!(fs::read(fixture.live_a()).unwrap(), NEW_A);
    assert_eq!(fs::read(fixture.live_b()).unwrap(), NEW_B);
    assert_eq!(fs::read(fixture.ue4ss().join("main.lua")).unwrap(), NEW_UE4SS);
    assert_eq!(fs::read(fixture.pak()).unwrap(), NEW_PAK);
}

fn assert_pristine_crash_backups(fixture: &CrashFixture) {
    assert_eq!(fs::read(bak_path(&fixture.live_a())).unwrap(), PRISTINE_A);
    assert_eq!(fs::read(bak_path(&fixture.live_b())).unwrap(), PRISTINE_B);
}

fn assert_bound_undeploy_record(fixture: &CrashFixture, guard_id: &str) -> PathBuf {
    let stored = read_record(&fixture.game)
        .expect("read interrupted reset record")
        .expect("interrupted reset retained its record");
    assert_eq!(stored.record.phase, DeployPhase::RecoveryRequired);
    let transaction = stored
        .record
        .recovery_transaction
        .expect("interrupted reset record retained its transaction");
    assert_eq!(transaction.transaction_id, guard_id);
    assert_eq!(transaction.operation, ManagerMutationOperation::Undeploy);
    assert_eq!(transaction.step, RecoveryTransactionStep::Applying);
    let scratch = PathBuf::from(transaction.scratch_root);
    assert_eq!(
        scratch,
        manager_transaction_root(&fixture.game, guard_id)
            .expect("resolve interrupted reset scratch root")
    );
    assert!(scratch.is_dir(), "interrupted reset scratch root is missing");
    scratch
}

fn assert_interrupted_undeploy_state(
    fixture: &CrashFixture,
    guard_id: &str,
    stage: UndeployCrashStage,
) {
    match stage {
        UndeployCrashStage::PreMutation => {
            assert_deployed_crash_payload(fixture);
            assert_pristine_crash_backups(fixture);
            let stored = read_record(&fixture.game)
                .expect("read pre-mutation reset record")
                .expect("pre-mutation reset retained the applied record");
            assert_eq!(stored.record.phase, DeployPhase::Applied);
            assert_eq!(
                stored
                    .record
                    .recovery_transaction
                    .as_ref()
                    .map(|transaction| transaction.operation),
                Some(ManagerMutationOperation::Apply)
            );
            assert!(!manager_transaction_root(&fixture.game, guard_id)
                .expect("resolve unused reset scratch root")
                .exists());
        }
        UndeployCrashStage::EarlyRecord => {
            assert_deployed_crash_payload(fixture);
            assert_pristine_crash_backups(fixture);
            assert_bound_undeploy_record(fixture, guard_id);
        }
        UndeployCrashStage::FirstRestore => {
            let live_a = fs::read(fixture.live_a()).unwrap();
            let live_b = fs::read(fixture.live_b()).unwrap();
            assert!(
                (live_a == PRISTINE_A && live_b == NEW_B)
                    || (live_a == NEW_A && live_b == PRISTINE_B),
                "first reset restore did not leave exactly one pristine live target"
            );
            assert_eq!(fs::read(fixture.ue4ss().join("main.lua")).unwrap(), NEW_UE4SS);
            assert_eq!(fs::read(fixture.pak()).unwrap(), NEW_PAK);
            assert_pristine_crash_backups(fixture);
            let scratch = assert_bound_undeploy_record(fixture, guard_id);
            let holders: Vec<_> = fs::read_dir(&scratch)
                .expect("read interrupted reset scratch root")
                .map(|entry| entry.expect("read reset scratch entry").path())
                .filter(|path| {
                    path.file_name()
                        .and_then(OsStr::to_str)
                        .is_some_and(|name| {
                            name.starts_with(".gore-mod-cleanup-")
                                || name.starts_with(".gore-ue4ss-delete-")
                        })
                })
                .collect();
            assert!(holders.is_empty(), "durable restore retained holders: {holders:#?}");
        }
        UndeployCrashStage::RecordRemoved => {
            assert_eq!(fs::read(fixture.live_a()).unwrap(), PRISTINE_A);
            assert_eq!(fs::read(fixture.live_b()).unwrap(), PRISTINE_B);
            assert!(!fixture.ue4ss().exists());
            assert!(!fixture.pak().exists());
            assert!(!bak_path(&fixture.live_a()).exists());
            assert!(!bak_path(&fixture.live_b()).exists());
            assert!(!record_path(&fixture.game).exists());
            let scratch = manager_transaction_root(&fixture.game, guard_id)
                .expect("resolve final reset scratch root");
            assert!(scratch.is_dir(), "final reset scratch root was removed too early");
            assert_directory_empty(&scratch, "final reset scratch root");
        }
    }
}

fn run_undeploy_crash_case(
    point: &str,
    stage: UndeployCrashStage,
    expected: ManagerInstallRecoveryOutcome,
) {
    let fixture = CrashFixture::new();
    let report = apply_loadout(&fixture.game, &fixture.library, &loadout(NEW_ID))
        .expect("prepare Manager deployment for interrupted reset");
    assert_eq!(report.applied, vec!["New crash fixture"]);
    assert_applied(&fixture, NEW_A, NEW_B, NEW_UE4SS, NEW_PAK);

    let guard_id = interrupt_manager_operation(&fixture, MODE_UNDEPLOY, point, None);
    assert_interrupted_undeploy_state(&fixture, &guard_id, stage);
    assert_eq!(
        recover_manager_install(&fixture.game, &guard_id)
            .expect("recover hard-killed Manager reset"),
        expected,
        "unexpected reset recovery outcome at {point}"
    );
    assert_eq!(
        recover_manager_install(&fixture.game, &guard_id)
            .expect("repeat Manager reset recovery for idempotence"),
        ManagerInstallRecoveryOutcome::AlreadyClean,
        "second recovery was not idempotent at {point}"
    );

    if expected == ManagerInstallRecoveryOutcome::PreMutationLockCleared {
        assert_applied(&fixture, NEW_A, NEW_B, NEW_UE4SS, NEW_PAK);
        assert!(
            undeploy_all(&fixture.game).expect("retry reset after clearing its pre-mutation lock"),
            "retry did not find the still-applied Manager deployment at {point}"
        );
    }
    assert!(
        !undeploy_all(&fixture.game).expect("repeat completed Manager reset for idempotence"),
        "completed reset was not idempotent at {point}"
    );
    assert_pristine(&fixture);
}

fn assert_applied(
    fixture: &CrashFixture,
    expected_a: &[u8],
    expected_b: &[u8],
    expected_ue4ss: &[u8],
    expected_pak: &[u8],
) {
    assert_eq!(
        fs::read(fixture.live_a()).expect("read first live file"),
        expected_a
    );
    assert_eq!(
        fs::read(fixture.live_b()).expect("read second live file"),
        expected_b
    );
    assert_eq!(
        fs::read(fixture.ue4ss().join("main.lua")).expect("read deployed UE4SS file"),
        expected_ue4ss
    );
    assert_eq!(
        fs::read(fixture.pak()).expect("read deployed pak"),
        expected_pak
    );
    let stored = read_record(&fixture.game)
        .expect("read applied deploy record")
        .expect("applied deploy record is present");
    assert_eq!(stored.record.owner, "manager");
    assert_eq!(stored.record.phase, DeployPhase::Applied);
    assert!(
        !fixture.lock().exists(),
        "completed apply retained its live lock"
    );
    assert_no_transient_gore_paths(&fixture.game);
}

fn assert_pristine(fixture: &CrashFixture) {
    assert_eq!(
        fs::read(fixture.live_a()).expect("read first pristine live file"),
        PRISTINE_A
    );
    assert_eq!(
        fs::read(fixture.live_b()).expect("read second pristine live file"),
        PRISTINE_B
    );
    assert!(
        !fixture.ue4ss().exists(),
        "Manager UE4SS directory survived recovery"
    );
    assert!(!fixture.pak().exists(), "Manager pak survived recovery");
    assert_directory_empty(
        &fixture.game.join("G1R/Binaries/Win64/ue4ss/Mods"),
        "UE4SS Mods",
    );
    assert_directory_empty(&fixture.game.join("G1R/Content/Paks/~mods"), "~mods");
    assert!(
        !record_path(&fixture.game).exists(),
        "Manager deploy record survived recovery/reset"
    );
    let residues = owned_residues(&fixture.game);
    assert!(
        residues.is_empty(),
        "Manager recovery left owned residue: {residues:#?}"
    );
    let observed = snapshot_game_tree(&fixture.game);
    assert_eq!(
        observed, fixture.initial_game_tree,
        "recovered game tree differs from its exact pre-apply snapshot"
    );
}

fn assert_directory_empty(path: &Path, label: &str) {
    let entries: Vec<_> = fs::read_dir(path)
        .unwrap_or_else(|error| panic!("read {label} directory {}: {error}", path.display()))
        .map(|entry| entry.expect("read fixture directory entry").path())
        .collect();
    assert!(entries.is_empty(), "{label} retained entries: {entries:#?}");
}

fn assert_no_transient_gore_paths(game: &Path) {
    let mut paths = Vec::new();
    collect_paths(game, &mut paths);
    let transient: Vec<_> = paths
        .into_iter()
        .filter(|path| {
            path.file_name()
                .and_then(OsStr::to_str)
                .is_some_and(|name| name.starts_with(".gore-"))
        })
        .collect();
    assert!(
        transient.is_empty(),
        "completed apply retained transient Manager paths: {transient:#?}"
    );
}

fn owned_residues(game: &Path) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    collect_paths(game, &mut paths);
    paths
        .into_iter()
        .filter(|path| {
            path.file_name()
                .and_then(OsStr::to_str)
                .is_some_and(|name| {
                    name == "gore-mod.deployed.json"
                        || name.starts_with(".gore-")
                        || name.ends_with(".gore-bak")
                })
        })
        .collect()
}

fn collect_paths(directory: &Path, paths: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(directory)
        .unwrap_or_else(|error| panic!("enumerating {}: {error}", directory.display()))
    {
        let entry = entry.expect("read fixture tree entry");
        let path = entry.path();
        let metadata = fs::symlink_metadata(&path).expect("read fixture tree metadata");
        paths.push(path.clone());
        if metadata.is_dir() && !metadata.file_type().is_symlink() {
            collect_paths(&path, paths);
        }
    }
}

fn snapshot_game_tree(root: &Path) -> BTreeMap<PathBuf, GameTreeEntry> {
    fn visit(root: &Path, directory: &Path, snapshot: &mut BTreeMap<PathBuf, GameTreeEntry>) {
        for entry in fs::read_dir(directory)
            .unwrap_or_else(|error| panic!("snapshotting {}: {error}", directory.display()))
        {
            let entry = entry.expect("read game-tree snapshot entry");
            let path = entry.path();
            let relative = path
                .strip_prefix(root)
                .expect("game-tree snapshot stays below its root")
                .to_path_buf();
            let metadata =
                fs::symlink_metadata(&path).expect("read game-tree snapshot entry metadata");
            let value = if metadata.is_dir() && !metadata.file_type().is_symlink() {
                GameTreeEntry::Directory
            } else if metadata.is_file() && !metadata.file_type().is_symlink() {
                GameTreeEntry::File(
                    fs::read(&path).expect("read game-tree snapshot regular-file bytes"),
                )
            } else {
                panic!(
                    "crash-test game tree contains an unsupported link or special object: {}",
                    path.display()
                );
            };
            assert!(
                snapshot.insert(relative, value).is_none(),
                "duplicate game-tree snapshot path"
            );
            if metadata.is_dir() {
                visit(root, &path, snapshot);
            }
        }
    }

    let mut snapshot = BTreeMap::new();
    visit(root, root, &mut snapshot);
    snapshot
}

macro_rules! apply_crash_test {
    ($name:ident, $point:literal, $scenario:expr, $expected:expr) => {
        #[test]
        fn $name() {
            run_apply_crash_case($point, $scenario, $expected);
        }
    };
}

apply_crash_test!(
    active_manager_guard_is_busy_and_hard_kill_is_recoverable,
    "apply.lock_acquired",
    ApplyScenario::Initial,
    ManagerInstallRecoveryOutcome::PreMutationLockCleared
);
apply_crash_test!(
    hard_kill_after_plan_basis_revalidation_clears_pre_mutation_lock,
    "apply.plan_basis_revalidated",
    ApplyScenario::Initial,
    ManagerInstallRecoveryOutcome::PreMutationLockCleared
);
apply_crash_test!(
    hard_kill_after_early_record_temp_sync_recovers,
    "apply.early_record_temp_synced",
    ApplyScenario::Initial,
    ManagerInstallRecoveryOutcome::PreMutationLockCleared
);
apply_crash_test!(
    hard_kill_after_early_record_is_durable_recovers,
    "apply.early_record_durable",
    ApplyScenario::Initial,
    ManagerInstallRecoveryOutcome::RecoveredToPristine
);
apply_crash_test!(
    hard_kill_after_first_backup_is_durable_recovers,
    "apply.first_backup_durable",
    ApplyScenario::Initial,
    ManagerInstallRecoveryOutcome::RecoveredToPristine
);
apply_crash_test!(
    hard_kill_after_first_ue4ss_stage_is_durable_recovers,
    "apply.first_ue4ss_stage_durable",
    ApplyScenario::Initial,
    ManagerInstallRecoveryOutcome::RecoveredToPristine
);
apply_crash_test!(
    hard_kill_before_first_live_write_recovers,
    "apply.before_first_live_write",
    ApplyScenario::Initial,
    ManagerInstallRecoveryOutcome::RecoveredToPristine
);
apply_crash_test!(
    hard_kill_between_live_writes_recovers,
    "apply.between_live_writes",
    ApplyScenario::Initial,
    ManagerInstallRecoveryOutcome::RecoveredToPristine
);
apply_crash_test!(
    hard_kill_after_old_ue4ss_tree_moves_during_reapply_recovers,
    "apply.ue4ss_old_moved",
    ApplyScenario::Reapply,
    ManagerInstallRecoveryOutcome::RecoveredToPristine
);
apply_crash_test!(
    hard_kill_after_applied_record_is_durable_preserves_then_resets,
    "apply.applied_record_durable_before_unlock",
    ApplyScenario::Initial,
    ManagerInstallRecoveryOutcome::CompletedApplyPreserved
);
#[cfg(windows)]
apply_crash_test!(
    hard_kill_with_a_transaction_bound_windows_write_stage_leaves_no_parent_residue,
    "apply.windows_write_stage_ready",
    ApplyScenario::Initial,
    ManagerInstallRecoveryOutcome::PreMutationLockCleared
);
#[cfg(windows)]
apply_crash_test!(
    hard_kill_with_a_transaction_bound_windows_noclobber_stage_leaves_no_parent_residue,
    "apply.windows_noclobber_stage_ready",
    ApplyScenario::Initial,
    ManagerInstallRecoveryOutcome::RecoveredToPristine
);
#[cfg(windows)]
apply_crash_test!(
    hard_kill_with_a_transaction_bound_windows_copy_stage_leaves_no_parent_residue,
    "apply.windows_copy_stage_ready",
    ApplyScenario::Initial,
    ManagerInstallRecoveryOutcome::RecoveredToPristine
);

macro_rules! recovery_crash_test {
    ($name:ident, $point:literal, $expected:expr) => {
        #[test]
        fn $name() {
            run_recovery_crash_case($point, $expected);
        }
    };
}

macro_rules! undeploy_crash_test {
    ($name:ident, $point:literal, $stage:expr, $expected:expr) => {
        #[test]
        fn $name() {
            run_undeploy_crash_case($point, $stage, $expected);
        }
    };
}

undeploy_crash_test!(
    hard_kill_after_reset_lock_acquisition_retries_cleanly,
    "undeploy.lock_acquired",
    UndeployCrashStage::PreMutation,
    ManagerInstallRecoveryOutcome::PreMutationLockCleared
);
undeploy_crash_test!(
    hard_kill_after_reset_early_record_recovers_to_pristine,
    "undeploy.early_record_durable",
    UndeployCrashStage::EarlyRecord,
    ManagerInstallRecoveryOutcome::CompletedUndeployConfirmed
);
undeploy_crash_test!(
    hard_kill_between_reset_restore_and_backup_cleanup_recovers_to_pristine,
    "undeploy.after_first_restore_durable",
    UndeployCrashStage::FirstRestore,
    ManagerInstallRecoveryOutcome::CompletedUndeployConfirmed
);
undeploy_crash_test!(
    hard_kill_after_reset_record_removal_finishes_scratch_cleanup,
    "undeploy.record_removed_before_scratch_cleanup",
    UndeployCrashStage::RecordRemoved,
    ManagerInstallRecoveryOutcome::CompletedUndeployConfirmed
);

recovery_crash_test!(
    hard_kill_after_recovery_takes_over_lock_can_resume,
    "recovery.lock_taken_over",
    ManagerInstallRecoveryOutcome::RecoveredToPristine
);
recovery_crash_test!(
    hard_kill_after_first_durable_recovery_cleanup_can_resume,
    "recovery.after_first_cleanup_durable",
    ManagerInstallRecoveryOutcome::RecoveredToPristine
);
recovery_crash_test!(
    hard_kill_before_recovery_lock_release_can_resume,
    "recovery.before_lock_release",
    ManagerInstallRecoveryOutcome::PreMutationLockCleared
);

#[test]
fn active_and_hard_killed_gore_as_owner_is_never_manager_owned() {
    let fixture = CrashFixture::new();
    hard_kill_at_point(
        &fixture,
        MODE_HOLD_GUARD,
        "guard.active",
        None,
        Some("gore-as:compile"),
    );
    assert_eq!(
        probe_manager_install_recovery(&fixture.game),
        ManagerInstallRecoveryReadiness::CompileOrAmbiguous
    );
    assert_eq!(
        recover_manager_install(&fixture.game, "abandoned-compile-probe")
            .expect("classify abandoned gore-as owner"),
        ManagerInstallRecoveryOutcome::CompileRecoveryRequired
    );
    assert!(
        fixture.lock().exists(),
        "Manager removed a gore-as-owned lock"
    );
    fs::remove_file(fixture.lock()).expect("remove dead test-only gore-as lock");
    assert_pristine(&fixture);
}

#[test]
fn concurrent_recoveries_have_exactly_one_mutating_winner() {
    let fixture = CrashFixture::new();
    let guard_id =
        interrupt_manager_operation(&fixture, MODE_APPLY, "apply.between_live_writes", None);
    let nonce = unique_nonce();
    let gate = fixture.control_path("gate", &nonce);
    let armed_a = fixture.control_path("armed-a", &nonce);
    let armed_b = fixture.control_path("armed-b", &nonce);
    let outcome_a = fixture.control_path("outcome-a", &nonce);
    let outcome_b = fixture.control_path("outcome-b", &nonce);
    let mut command_a = child_command(&fixture, MODE_RECOVER_ONCE);
    command_a
        .env(CHILD_EXPECTED_GUARD_ENV, &guard_id)
        .env(CHILD_ARMED_ENV, &armed_a)
        .env(CHILD_GATE_ENV, &gate)
        .env(CHILD_OUTCOME_ENV, &outcome_a);
    let mut command_b = child_command(&fixture, MODE_RECOVER_ONCE);
    command_b
        .env(CHILD_EXPECTED_GUARD_ENV, &guard_id)
        .env(CHILD_ARMED_ENV, &armed_b)
        .env(CHILD_GATE_ENV, &gate)
        .env(CHILD_OUTCOME_ENV, &outcome_b);
    let mut child_a = command_a.spawn().expect("spawn first recovery contender");
    let mut child_b = command_b.spawn().expect("spawn second recovery contender");
    wait_for_path(&mut child_a, &armed_a, "first recovery contender");
    wait_for_path(&mut child_b, &armed_b, "second recovery contender");
    durable_create_new(&gate, b"go\n");
    wait_for_success(&mut child_a, "first recovery contender");
    wait_for_success(&mut child_b, "second recovery contender");
    let outcomes = [outcome_a, outcome_b].map(|path| {
        serde_json::from_slice::<ManagerInstallRecoveryOutcome>(
            &fs::read(&path).unwrap_or_else(|error| panic!("reading {}: {error}", path.display())),
        )
        .unwrap_or_else(|error| panic!("parsing {}: {error}", path.display()))
    });
    assert_eq!(
        outcomes
            .iter()
            .filter(|outcome| **outcome == ManagerInstallRecoveryOutcome::RecoveredToPristine)
            .count(),
        1,
        "concurrent recovery outcomes: {outcomes:?}"
    );
    assert!(
        outcomes.iter().all(|outcome| matches!(
            outcome,
            ManagerInstallRecoveryOutcome::RecoveredToPristine
                | ManagerInstallRecoveryOutcome::Busy
                | ManagerInstallRecoveryOutcome::AlreadyClean
        )),
        "unexpected concurrent recovery outcome: {outcomes:?}"
    );
    assert_eq!(
        recover_manager_install(&fixture.game, &guard_id)
            .expect("verify concurrent recovery idempotence"),
        ManagerInstallRecoveryOutcome::AlreadyClean
    );
    assert_pristine(&fixture);
}

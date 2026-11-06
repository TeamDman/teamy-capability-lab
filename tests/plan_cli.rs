use cloud_terrastodon_excalidraw::ExcalidrawScene;
use std::process::Command;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;
use teamy_capability_lab::capability::planner::PlanningReport;

#[test]
fn json_command_creates_an_editable_file_without_overwriting() {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path = std::env::temp_dir().join(format!(
        "capability-lab-{}-{nonce}.excalidraw",
        std::process::id()
    ));
    let run = || {
        Command::new(env!("CARGO_BIN_EXE_teamy-capability-lab"))
            .args(["--output-format", "json", "plan", "--excalidraw"])
            .arg(&path)
            .output()
            .unwrap()
    };
    let first = run();
    assert!(
        first.status.success(),
        "{}",
        String::from_utf8_lossy(&first.stderr)
    );
    let stdout = std::str::from_utf8(&first.stdout).unwrap();
    let report: PlanningReport = facet_json::from_str(stdout).unwrap();
    assert_eq!(report.candidates.len(), 3);
    let drawing = std::fs::read_to_string(&path).unwrap();
    let scene: ExcalidrawScene = facet_json::from_str(&drawing).unwrap();
    assert!(!scene.elements.is_empty());
    let second = run();
    assert!(!second.status.success());
    assert_eq!(std::fs::read_to_string(&path).unwrap(), drawing);
    std::fs::remove_file(path).unwrap();
}

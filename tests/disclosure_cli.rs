use std::process::Command;
use teamy_capability_lab::capability::identity::DisclosureAssessment;
use teamy_capability_lab::capability::identity::ReadPowerOutput;

#[test]
fn disclosure_cli_emits_only_the_synthetic_projected_property() {
    let output = Command::new(env!("CARGO_BIN_EXE_teamy-capability-lab"))
        .args(["--output-format", "json", "disclosure"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let json = std::str::from_utf8(&output.stdout).unwrap();
    let value: DisclosureAssessment = facet_json::from_str(json).unwrap();
    assert_eq!(
        value,
        DisclosureAssessment::Visible(ReadPowerOutput {
            powered_on: Some(true)
        })
    );
    for undisclosed in [
        "alice",
        "desktop",
        "jeff-on",
        "owner_tag",
        "Jeff",
        "os_version",
        "synthetic-os",
        "disk_label",
        "synthetic-disk",
    ] {
        assert!(!json.contains(undisclosed));
    }
}

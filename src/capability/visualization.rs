//! Convert actual validated alternatives into a disposable editable scene.
use super::planner::PlanningReport;
use cloud_terrastodon_excalidraw::ExcalidrawObjectField;
use cloud_terrastodon_excalidraw::ExcalidrawObjectSnapshot;
use cloud_terrastodon_excalidraw::ExcalidrawScene;

/// # Errors
/// Returns an error if the scene cannot be constructed or serialized.
pub fn diagram_json(report: &PlanningReport) -> eyre::Result<String> {
    let mut objects = vec![ExcalidrawObjectSnapshot {
        address: "search".into(),
        shape: "Synthetic planning result".into(),
        fields: vec![
            field("artifact", report.artifact.clone()),
            field("source", report.source.clone()),
            field("target", report.target.clone()),
            field("search completeness", format!("{:?}", report.completeness)),
            field("execution authorized", "no".into()),
        ],
        fields_complete: true,
        plan: None,
    }];
    for (index, candidate) in report.candidates.iter().enumerate() {
        objects.push(ExcalidrawObjectSnapshot {
            address: format!("candidate-{}", index + 1),
            shape: format!("Candidate {}", index + 1),
            fields: vec![
                field("steps", candidate.transfers.join(" -> ")),
                field(
                    "estimated cost",
                    candidate
                        .estimated_cost
                        .map_or_else(|| "overflow".into(), |cost| cost.to_string()),
                ),
                field("status and reasons", format!("{:?}", candidate.status)),
                field("approvals needed", candidate.approvals_needed.join(", ")),
            ],
            fields_complete: true,
            plan: None,
        });
    }
    ExcalidrawScene::from_objects("Synthetic transfer alternatives", &objects)?.to_json()
}

fn field(name: &str, value: String) -> ExcalidrawObjectField {
    ExcalidrawObjectField {
        name: name.into(),
        type_name: None,
        value,
    }
}

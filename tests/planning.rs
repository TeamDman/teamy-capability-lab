use cloud_terrastodon_excalidraw::ExcalidrawScene;
use teamy_capability_lab::capability::BlockedReason;
use teamy_capability_lab::capability::Capability;
use teamy_capability_lab::capability::EndpointId;
use teamy_capability_lab::capability::Permission;
use teamy_capability_lab::capability::RouteStatus;
use teamy_capability_lab::capability::SearchCompleteness;
use teamy_capability_lab::capability::planner::PlanningReport;
use teamy_capability_lab::capability::planner::SearchBudget;
use teamy_capability_lab::capability::planner::plan;
use teamy_capability_lab::capability::synthetic_fixture;
use teamy_capability_lab::capability::validate_route;
use teamy_capability_lab::capability::visualization::diagram_json;

#[test]
fn enumerates_and_ranks_the_actual_alternatives() {
    let fixture = synthetic_fixture();
    let report = plan(&fixture, SearchBudget::default(), || false).unwrap();
    assert_eq!(report.completeness, SearchCompleteness::Complete);
    assert!(!report.execution_authorized);
    assert_eq!(report.candidates.len(), 3);
    assert_eq!(report.candidates[0].transfers, ["upload", "download"]);
    assert_eq!(report.candidates[0].estimated_cost, Some(4));
    assert_eq!(report.candidates[0].status, RouteStatus::Ready);
    assert_eq!(report.candidates[1].transfers, ["direct"]);
    assert_eq!(report.candidates[1].estimated_cost, Some(3));
    assert!(matches!(
        report.candidates[1].status,
        RouteStatus::ApprovalNeeded { .. }
    ));
    assert_eq!(report.candidates[2].transfers, ["to-dot", "from-dot"]);
    assert!(matches!(
        report.candidates[2].status,
        RouteStatus::Blocked { .. }
    ));
    let second = plan(&fixture, SearchBudget::default(), || false).unwrap();
    assert_eq!(
        report
            .candidates
            .iter()
            .map(|candidate| &candidate.transfers)
            .collect::<Vec<_>>(),
        second
            .candidates
            .iter()
            .map(|candidate| &candidate.transfers)
            .collect::<Vec<_>>()
    );
}

#[test]
fn incomplete_search_never_claims_exhaustive_results() {
    let fixture = synthetic_fixture();
    let depth = SearchBudget {
        max_depth: 1,
        ..SearchBudget::default()
    };
    let report = plan(&fixture, depth, || false).unwrap();
    assert_eq!(report.completeness, SearchCompleteness::DepthLimit);
    let limit = SearchBudget {
        max_solutions: 1,
        ..SearchBudget::default()
    };
    let report = plan(&fixture, limit, || false).unwrap();
    assert_eq!(report.completeness, SearchCompleteness::SolutionLimit);
    let inference = SearchBudget {
        max_inferences_per_solution: 1,
        ..SearchBudget::default()
    };
    let report = plan(&fixture, inference, || false).unwrap();
    assert_eq!(report.completeness, SearchCompleteness::InferenceLimit);
    assert_eq!(
        plan(&fixture, SearchBudget::default(), || true)
            .unwrap()
            .completeness,
        SearchCompleteness::Cancelled
    );
}

#[test]
fn cycles_are_not_enumerated_and_direction_is_not_symmetric() {
    let mut fixture = synthetic_fixture();
    let mut back = fixture.transfers[1].clone();
    back.id = "cycle".into();
    std::mem::swap(&mut back.from, &mut back.to);
    fixture.transfers.push(back);
    let report = plan(&fixture, SearchBudget::default(), || false).unwrap();
    assert_eq!(report.candidates.len(), 3);
    assert!(
        report
            .candidates
            .iter()
            .all(|candidate| !candidate.transfers.contains(&"cycle".to_string()))
    );
    fixture.artifact.location = fixture.target.clone();
    fixture.target = EndpointId("desktop".into());

    let report = plan(&fixture, SearchBudget::default(), || false).unwrap();
    assert!(report.candidates.is_empty());
}

#[test]
fn wrong_artifact_and_permissions_have_separate_reasons() {
    let mut fixture = synthetic_fixture();
    let mut route = fixture.transfers[1..3].to_vec();
    route[0].artifact_id = "different-file".into();
    route[0].permission = Permission::Denied;
    route[1].permission = Permission::Unknown;
    let result = validate_route(
        &fixture.endpoints,
        &fixture.artifact,
        &fixture.target,
        &route,
    );
    assert_eq!(result.approvals_needed, ["download"]);
    assert!(
        matches!(result.status, RouteStatus::Blocked { reasons } if reasons.iter().any(|reason| matches!(reason, BlockedReason::WrongArtifact { .. })) && reasons.iter().any(|reason| matches!(reason, BlockedReason::PermissionDenied { .. })))
    );
    fixture.endpoints[3]
        .capabilities
        .retain(|c| *c != Capability::Receive);
    let result = validate_route(
        &fixture.endpoints,
        &fixture.artifact,
        &fixture.target,
        &fixture.transfers[..1],
    );
    assert!(
        matches!(result.status, RouteStatus::Blocked { reasons } if reasons.iter().any(|reason| matches!(reason, BlockedReason::MissingCapability { endpoint, capability: Capability::Receive } if endpoint == "codex-instance-a")))
    );
}

#[test]
fn actual_results_roundtrip_as_json_and_excalidraw() {
    let report = plan(&synthetic_fixture(), SearchBudget::default(), || false).unwrap();
    let json = facet_json::to_string(&report).unwrap();
    let restored: PlanningReport = facet_json::from_str(&json).unwrap();
    assert_eq!(restored.candidates.len(), 3);
    let drawing = diagram_json(&report).unwrap();
    let restored: ExcalidrawScene = facet_json::from_str(&drawing).unwrap();
    assert!(!restored.elements.is_empty());
    assert!(
        drawing.contains("direct")
            && drawing.contains("upload")
            && drawing.contains("ApprovalNeeded")
    );
}

#[test]
fn equal_status_and_cost_use_transfer_ids_as_tie_breaker() {
    let mut fixture = synthetic_fixture();
    fixture.transfers[0].permission = Permission::Granted;
    let mut second = fixture.transfers[0].clone();
    second.id = "aaa-direct".into();
    fixture.transfers.push(second);
    let report = plan(&fixture, SearchBudget::default(), || false).unwrap();
    assert_eq!(report.candidates[0].transfers, ["aaa-direct"]);
    assert_eq!(report.candidates[1].transfers, ["direct"]);
}

#[test]
fn cancellation_between_answers_retains_incomplete_status() {
    let mut checks = 0;
    let report = plan(&synthetic_fixture(), SearchBudget::default(), || {
        checks += 1;
        checks >= 3
    })
    .unwrap();
    assert_eq!(report.completeness, SearchCompleteness::Cancelled);
    assert_eq!(report.candidates.len(), 1);
}

//! Typed synthetic state, bounded route enumeration and validation.

pub mod identity;
pub mod planner;
pub mod visualization;

use facet::Facet;

/// A concrete endpoint identity; definitions are deliberately a different type.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Facet)]
pub struct EndpointId(pub String);

#[derive(Clone, Debug, PartialEq, Eq, Hash, Facet)]
pub struct DefinitionId(pub String);

#[derive(Clone, Debug, PartialEq, Eq, Facet)]
#[repr(u8)]
pub enum EndpointKind {
    Desktop,
    DotVm,
    CodexInstance { definition: DefinitionId },
    IntermediateStore,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Facet)]
#[repr(u8)]
pub enum Capability {
    Send,
    Receive,
    ProcessArtifact,
}

#[derive(Clone, Debug, Facet)]
pub struct Endpoint {
    pub id: EndpointId,
    pub kind: EndpointKind,
    pub capabilities: Vec<Capability>,
}

#[derive(Clone, Debug, Facet)]
pub struct EnvironmentDefinition {
    pub id: DefinitionId,
    pub description: String,
}

#[derive(Clone, Debug, Facet)]
pub struct Artifact {
    pub id: String,
    pub location: EndpointId,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Facet)]
#[repr(u8)]
pub enum Permission {
    Granted,
    Denied,
    Unknown,
}

/// A directed, synthetic transfer contract; no transfer implementation exists.
#[derive(Clone, Debug, Facet)]
pub struct Transfer {
    pub id: String,
    pub artifact_id: String,
    pub from: EndpointId,
    pub to: EndpointId,
    pub required_source: Capability,
    pub required_destination: Capability,
    pub permission: Permission,
    pub estimated_cost: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Facet)]
#[repr(u8)]
pub enum BlockedReason {
    EmptyRoute,
    WrongArtifact {
        expected: String,
        actual: String,
    },
    MissingEndpoint {
        endpoint: String,
    },
    MissingInput {
        expected_at: String,
        actual_at: String,
    },
    MissingCapability {
        endpoint: String,
        capability: Capability,
    },
    PermissionDenied {
        transfer: String,
    },
    WrongDestination {
        expected: String,
        actual: String,
    },
    CostOverflow,
}

#[derive(Clone, Debug, PartialEq, Eq, Facet)]
#[repr(u8)]
pub enum RouteStatus {
    Ready,
    ApprovalNeeded { transfers: Vec<String> },
    Blocked { reasons: Vec<BlockedReason> },
}

/// Exhaustion is distinct from an exhaustive search with no results.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Facet)]
#[repr(u8)]
pub enum SearchCompleteness {
    Complete,
    DepthLimit,
    SolutionLimit,
    InferenceLimit,
    Cancelled,
}

#[derive(Clone, Debug, Facet)]
pub struct ValidatedRoute {
    pub transfers: Vec<String>,
    pub estimated_cost: Option<u64>,
    pub approvals_needed: Vec<String>,
    pub status: RouteStatus,
}

/// Validate an enumerated route without performing it.
#[must_use]
pub fn validate_route(
    endpoints: &[Endpoint],
    artifact: &Artifact,
    target: &EndpointId,
    route: &[Transfer],
) -> ValidatedRoute {
    let mut location = &artifact.location;
    let mut reasons = Vec::new();
    let mut approvals = Vec::new();
    let mut cost = Some(0_u64);
    if route.is_empty() {
        reasons.push(BlockedReason::EmptyRoute);
    }
    for transfer in route {
        if transfer.artifact_id != artifact.id {
            reasons.push(BlockedReason::WrongArtifact {
                expected: transfer.artifact_id.clone(),
                actual: artifact.id.clone(),
            });
        }
        if &transfer.from != location {
            reasons.push(BlockedReason::MissingInput {
                expected_at: transfer.from.0.clone(),
                actual_at: location.0.clone(),
            });
        }
        for (id, capability) in [
            (&transfer.from, transfer.required_source),
            (&transfer.to, transfer.required_destination),
        ] {
            match endpoints.iter().find(|endpoint| &endpoint.id == id) {
                None => reasons.push(BlockedReason::MissingEndpoint {
                    endpoint: id.0.clone(),
                }),
                Some(endpoint) if !endpoint.capabilities.contains(&capability) => {
                    reasons.push(BlockedReason::MissingCapability {
                        endpoint: id.0.clone(),
                        capability,
                    });
                }
                Some(_) => {}
            }
        }
        match transfer.permission {
            Permission::Granted => {}
            Permission::Denied => reasons.push(BlockedReason::PermissionDenied {
                transfer: transfer.id.clone(),
            }),
            Permission::Unknown => approvals.push(transfer.id.clone()),
        }
        cost = cost.and_then(|total| total.checked_add(transfer.estimated_cost));
        location = &transfer.to;
    }
    if cost.is_none() {
        reasons.push(BlockedReason::CostOverflow);
    }
    if location != target {
        reasons.push(BlockedReason::WrongDestination {
            expected: target.0.clone(),
            actual: location.0.clone(),
        });
    }
    match endpoints.iter().find(|endpoint| &endpoint.id == target) {
        None => reasons.push(BlockedReason::MissingEndpoint {
            endpoint: target.0.clone(),
        }),
        Some(endpoint) if !endpoint.capabilities.contains(&Capability::ProcessArtifact) => {
            reasons.push(BlockedReason::MissingCapability {
                endpoint: target.0.clone(),
                capability: Capability::ProcessArtifact,
            });
        }
        Some(_) => {}
    }
    let status = if !reasons.is_empty() {
        RouteStatus::Blocked { reasons }
    } else if approvals.is_empty() {
        RouteStatus::Ready
    } else {
        RouteStatus::ApprovalNeeded {
            transfers: approvals.clone(),
        }
    };
    ValidatedRoute {
        transfers: route.iter().map(|transfer| transfer.id.clone()).collect(),
        estimated_cost: cost,
        approvals_needed: approvals,
        status,
    }
}

/// Entirely fictional state, used only for offline demonstrations and tests.
#[derive(Clone, Debug, Facet)]
pub struct Fixture {
    pub definition: EnvironmentDefinition,
    pub endpoints: Vec<Endpoint>,
    pub artifact: Artifact,
    pub target: EndpointId,
    pub transfers: Vec<Transfer>,
}

#[must_use]
pub fn synthetic_fixture() -> Fixture {
    fn id(value: &str) -> EndpointId {
        EndpointId(value.to_owned())
    }
    fn endpoint(name: &str, kind: EndpointKind, capabilities: &[Capability]) -> Endpoint {
        Endpoint {
            id: id(name),
            kind,
            capabilities: capabilities.to_vec(),
        }
    }
    fn transfer(name: &str, from: &str, to: &str, permission: Permission, cost: u64) -> Transfer {
        Transfer {
            id: name.to_owned(),
            artifact_id: "synthetic-artifact".to_owned(),
            from: id(from),
            to: id(to),
            required_source: Capability::Send,
            required_destination: Capability::Receive,
            permission,
            estimated_cost: cost,
        }
    }
    let definition = DefinitionId("codex-definition-1".to_owned());
    Fixture {
        definition: EnvironmentDefinition {
            id: definition.clone(),
            description: "Synthetic Codex environment definition".to_owned(),
        },
        endpoints: vec![
            endpoint("desktop", EndpointKind::Desktop, &[Capability::Send]),
            endpoint(
                "dot-vm",
                EndpointKind::DotVm,
                &[Capability::Send, Capability::Receive],
            ),
            endpoint(
                "store",
                EndpointKind::IntermediateStore,
                &[Capability::Send, Capability::Receive],
            ),
            endpoint(
                "codex-instance-a",
                EndpointKind::CodexInstance {
                    definition: definition.clone(),
                },
                &[Capability::Receive, Capability::ProcessArtifact],
            ),
            endpoint(
                "codex-instance-b",
                EndpointKind::CodexInstance { definition },
                &[Capability::Receive],
            ),
        ],
        artifact: Artifact {
            id: "synthetic-artifact".to_owned(),
            location: id("desktop"),
        },
        target: id("codex-instance-a"),
        transfers: vec![
            transfer(
                "direct",
                "desktop",
                "codex-instance-a",
                Permission::Unknown,
                3,
            ),
            transfer("upload", "desktop", "store", Permission::Granted, 2),
            transfer(
                "download",
                "store",
                "codex-instance-a",
                Permission::Granted,
                2,
            ),
            transfer("to-dot", "desktop", "dot-vm", Permission::Denied, 1),
            transfer(
                "from-dot",
                "dot-vm",
                "codex-instance-a",
                Permission::Granted,
                1,
            ),
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn permission_remains_distinct_from_connectivity() {
        let fixture = synthetic_fixture();
        let direct = validate_route(
            &fixture.endpoints,
            &fixture.artifact,
            &fixture.target,
            &fixture.transfers[..1],
        );
        assert!(matches!(direct.status, RouteStatus::ApprovalNeeded { .. }));
        let via_store = validate_route(
            &fixture.endpoints,
            &fixture.artifact,
            &fixture.target,
            &fixture.transfers[1..3],
        );
        assert_eq!(via_store.status, RouteStatus::Ready);
        assert_eq!(via_store.estimated_cost, Some(4));
        let mut denied = fixture.transfers[0].clone();
        denied.permission = Permission::Denied;
        assert!(matches!(
            validate_route(
                &fixture.endpoints,
                &fixture.artifact,
                &fixture.target,
                &[denied]
            )
            .status,
            RouteStatus::Blocked { .. }
        ));
    }

    #[test]
    fn direction_and_input_location_are_enforced() {
        let fixture = synthetic_fixture();
        let mut reverse = fixture.transfers[0].clone();
        std::mem::swap(&mut reverse.from, &mut reverse.to);
        let result = validate_route(
            &fixture.endpoints,
            &fixture.artifact,
            &fixture.target,
            &[reverse],
        );
        assert!(
            matches!(result.status, RouteStatus::Blocked { reasons } if reasons.iter().any(|r| matches!(r, BlockedReason::MissingInput { .. })))
        );
    }

    #[test]
    fn capability_is_not_inherited_between_instances_or_from_definition() {
        let fixture = synthetic_fixture();
        let mut transfer = fixture.transfers[0].clone();
        transfer.to = EndpointId("codex-instance-b".into());
        let second = transfer.to.clone();
        assert!(
            matches!(validate_route(&fixture.endpoints, &fixture.artifact, &second, &[transfer.clone()]).status, RouteStatus::Blocked { reasons } if reasons.iter().any(|r| matches!(r, BlockedReason::MissingCapability { capability: Capability::ProcessArtifact, .. })))
        );
        transfer.to = EndpointId(fixture.definition.id.0.clone());
        let definition_as_target = transfer.to.clone();
        assert!(
            matches!(validate_route(&fixture.endpoints, &fixture.artifact, &definition_as_target, &[transfer]).status, RouteStatus::Blocked { reasons } if reasons.iter().any(|r| matches!(r, BlockedReason::MissingEndpoint { .. })))
        );
    }

    #[test]
    fn checked_cost_reports_overflow() {
        let fixture = synthetic_fixture();
        let mut route = fixture.transfers[1..3].to_vec();
        route[0].estimated_cost = u64::MAX;
        assert!(
            matches!(validate_route(&fixture.endpoints, &fixture.artifact, &fixture.target, &route).status, RouteStatus::Blocked { reasons } if reasons.contains(&BlockedReason::CostOverflow))
        );
    }
}

#[cfg(test)]
mod endpoint_tests {
    use super::*;

    #[test]
    fn source_and_destination_capabilities_belong_to_the_specific_endpoints() {
        let mut fixture = synthetic_fixture();
        fixture.transfers[0].permission = Permission::Granted;
        fixture.endpoints[0].capabilities.clear();
        let report = validate_route(
            &fixture.endpoints,
            &fixture.artifact,
            &fixture.target,
            &fixture.transfers[..1],
        );
        assert!(
            matches!(report.status, RouteStatus::Blocked { reasons } if reasons.iter().any(|reason| matches!(reason, BlockedReason::MissingCapability { endpoint, capability: Capability::Send } if endpoint == "desktop")))
        );

        let mut fixture = synthetic_fixture();
        fixture.transfers[0].permission = Permission::Granted;
        fixture.endpoints[3]
            .capabilities
            .retain(|capability| *capability != Capability::Receive);
        let report = validate_route(
            &fixture.endpoints,
            &fixture.artifact,
            &fixture.target,
            &fixture.transfers[..1],
        );
        assert!(
            matches!(report.status, RouteStatus::Blocked { reasons } if reasons.iter().any(|reason| matches!(reason, BlockedReason::MissingCapability { endpoint, capability: Capability::Receive } if endpoint == "codex-instance-a")))
        );
    }
}

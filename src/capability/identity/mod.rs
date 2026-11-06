//! Synthetic, in-memory identity and selective-disclosure experiment.
//!
//! This module does not authenticate, enforce access, discover resources or execute
//! operations. Its assessment is an observation over explicitly supplied facts.

use super::DefinitionId;
use super::EndpointId;
use super::Permission;
use cloud_terrastodon_registry::Effect;
use facet::Facet;
use std::future::Ready;
use std::future::ready;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;

#[derive(Clone, Debug, PartialEq, Eq, Hash, Facet)]
pub struct PrincipalId(pub String);

#[derive(Clone, Debug, PartialEq, Eq, Hash, Facet)]
pub struct DeviceId(pub String);

#[derive(Clone, Debug, PartialEq, Eq, Hash, Facet)]
pub struct VmId(pub String);

/// A client device is distinct from a concrete environment instance and definition.
#[derive(Clone, Debug, PartialEq, Eq, Facet)]
#[repr(u8)]
pub enum Origin {
    Desktop(DeviceId),
    Phone(DeviceId),
    Environment {
        definition: DefinitionId,
        instance: EndpointId,
    },
}

/// A fixture assertion, not proof that a principal authenticated on an origin.
#[derive(Clone, Debug, PartialEq, Eq, Facet)]
pub struct Session {
    pub principal: PrincipalId,
    pub origin: Origin,
}

/// A synthetic request for one already-selected VM, not a resource-listing API.
#[derive(Clone, Debug, Facet)]
pub struct ReadPowerRequest {
    pub session: Session,
    pub vm: VmId,
}

/// The only property returned by the fixture for a matching VM.
#[derive(Clone, Debug, PartialEq, Eq, Facet)]
pub struct ReadPowerOutput {
    /// `None` means the powered-on state is not known, not that the VM is off.
    pub powered_on: Option<bool>,
}

static EXECUTOR_CALLS: AtomicUsize = AtomicUsize::new(0);

impl IntoFuture for ReadPowerRequest {
    type Output = eyre::Result<ReadPowerOutput>;
    type IntoFuture = Ready<Self::Output>;

    fn into_future(self) -> Self::IntoFuture {
        EXECUTOR_CALLS.fetch_add(1, Ordering::SeqCst);
        ready(Err(eyre::eyre!(
            "synthetic registry entry has no executable operation"
        )))
    }
}

cloud_terrastodon_registry::register_thing!(ReadPowerRequest);
cloud_terrastodon_registry::register_thing!(ReadPowerOutput);
cloud_terrastodon_registry::register_into_future!(ReadPowerRequest => ReadPowerOutput, effects = [Read]);

/// The stub cannot access a VM. The count lets tests detect accidental invocation.
#[must_use]
pub fn executor_call_count() -> usize {
    EXECUTOR_CALLS.load(Ordering::SeqCst)
}

/// Synthetic records contain fields that are not included in the projection.
#[derive(Clone, Debug)]
pub struct SyntheticVm {
    pub id: VmId,
    pub owner_tag: Option<String>,
    pub powered_on: Option<bool>,
    pub os_version: String,
    pub disk_label: String,
}

/// One independently asserted invocation decision and one read-effect decision.
/// These values are fixture inputs, not permissions granted by the registry.
#[derive(Clone, Debug)]
pub struct PermissionObservation {
    pub principal: PrincipalId,
    pub vm: VmId,
    pub invoke: Permission,
    pub read_effect: Permission,
}

#[derive(Clone, Debug)]
pub struct IdentityFixture {
    pub sessions: Vec<Session>,
    pub vms: Vec<SyntheticVm>,
    pub permissions: Vec<PermissionObservation>,
}

#[derive(Clone, Debug, PartialEq, Eq, Facet)]
#[repr(u8)]
pub enum UnknownReason {
    UnrecognizedSession,
    MissingPermission,
    ConflictingPermissions,

    MissingVm,
    DuplicateVm,
    OwnerTag,
    UnsupportedRegistryEffects,
}

/// Internal fixture outcomes. This is not a policy for externally visible errors.
#[derive(Clone, Debug, PartialEq, Eq, Facet)]
#[repr(u8)]
pub enum DisclosureAssessment {
    Visible(ReadPowerOutput),
    Denied {
        invoke: Permission,
        read_effect: Permission,
    },
    UnknownPermissions {
        invoke: Permission,
        read_effect: Permission,
    },
    NotApplicable,
    Unknown(UnknownReason),
}

/// Assess one synthetic request without calling the registered executor.
///
/// The registration supplies the declared operation/effect metadata only; both
/// access decisions come from fixture facts. Missing or duplicate facts remain
/// unknown. Reporting `NotApplicable` is only an internal test outcome and does
/// not prescribe how a real service should conceal resource existence.
#[must_use]
pub fn assess(fixture: &IdentityFixture, request: &ReadPowerRequest) -> DisclosureAssessment {
    if !fixture.sessions.contains(&request.session) {
        return DisclosureAssessment::Unknown(UnknownReason::UnrecognizedSession);
    }

    let operation = ReadPowerRequest::REGISTERED_INTO_FUTURE;
    if operation.effects.len() != 1 || operation.effects.first() != Some(&Effect::Read) {
        return DisclosureAssessment::Unknown(UnknownReason::UnsupportedRegistryEffects);
    }

    let mut matching = fixture
        .permissions
        .iter()
        .filter(|fact| fact.principal == request.session.principal && fact.vm == request.vm);
    let Some(permission) = matching.next() else {
        return DisclosureAssessment::Unknown(UnknownReason::MissingPermission);
    };
    if matching.next().is_some() {
        return DisclosureAssessment::Unknown(UnknownReason::ConflictingPermissions);
    }
    if permission.invoke == Permission::Denied || permission.read_effect == Permission::Denied {
        return DisclosureAssessment::Denied {
            invoke: permission.invoke,
            read_effect: permission.read_effect,
        };
    }
    if permission.invoke == Permission::Unknown || permission.read_effect == Permission::Unknown {
        return DisclosureAssessment::UnknownPermissions {
            invoke: permission.invoke,
            read_effect: permission.read_effect,
        };
    }
    let mut matches = fixture.vms.iter().filter(|vm| vm.id == request.vm);
    let Some(vm) = matches.next() else {
        return DisclosureAssessment::Unknown(UnknownReason::MissingVm);
    };
    if matches.next().is_some() {
        return DisclosureAssessment::Unknown(UnknownReason::DuplicateVm);
    }
    match vm.owner_tag.as_deref() {
        None => DisclosureAssessment::Unknown(UnknownReason::OwnerTag),
        Some("Jeff") => DisclosureAssessment::Visible(ReadPowerOutput {
            powered_on: vm.powered_on,
        }),
        Some(_) => DisclosureAssessment::NotApplicable,
    }
}

/// Construct fictional identities, sessions, VM records and explicit observations.
#[must_use]
pub fn synthetic_identity_fixture() -> IdentityFixture {
    let alice = PrincipalId("alice".into());
    let bob = PrincipalId("bob".into());
    let agent = PrincipalId("agent".into());
    let vm = |name: &str, owner_tag: Option<&str>, powered_on: Option<bool>| SyntheticVm {
        id: VmId(name.into()),
        owner_tag: owner_tag.map(str::to_owned),
        powered_on,
        os_version: "synthetic-os".into(),
        disk_label: "synthetic-disk".into(),
    };
    let vms = vec![
        vm("jeff-on", Some("Jeff"), Some(true)),
        vm("not-jeff", Some("Other"), Some(false)),
        vm("unknown-owner", None, Some(true)),
        vm("unknown-power", Some("Jeff"), None),
    ];
    let mut permissions = vms
        .iter()
        .map(|record| PermissionObservation {
            principal: alice.clone(),
            vm: record.id.clone(),
            invoke: Permission::Granted,
            read_effect: Permission::Granted,
        })
        .collect::<Vec<_>>();
    permissions.push(PermissionObservation {
        principal: bob.clone(),
        vm: VmId("jeff-on".into()),
        invoke: Permission::Denied,
        read_effect: Permission::Granted,
    });
    permissions.push(PermissionObservation {
        principal: agent.clone(),
        vm: VmId("jeff-on".into()),
        invoke: Permission::Granted,
        read_effect: Permission::Unknown,
    });
    IdentityFixture {
        sessions: vec![
            Session {
                principal: alice.clone(),
                origin: Origin::Desktop(DeviceId("desktop".into())),
            },
            Session {
                principal: alice,
                origin: Origin::Phone(DeviceId("alice-phone".into())),
            },
            Session {
                principal: bob,
                origin: Origin::Phone(DeviceId("bob-phone".into())),
            },
            Session {
                principal: agent,
                origin: Origin::Environment {
                    definition: DefinitionId("codex-definition-1".into()),
                    instance: EndpointId("codex-instance-a".into()),
                },
            },
        ],
        vms,
        permissions,
    }
}

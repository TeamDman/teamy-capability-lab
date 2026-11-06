use cloud_terrastodon_registry::Effect;
use cloud_terrastodon_registry::known_functions;
use cloud_terrastodon_registry::known_thing_for_shape;
use facet::Facet;
use teamy_capability_lab::capability::Permission;
use teamy_capability_lab::capability::identity::DeviceId;
use teamy_capability_lab::capability::identity::DisclosureAssessment;
use teamy_capability_lab::capability::identity::IdentityFixture;
use teamy_capability_lab::capability::identity::Origin;
use teamy_capability_lab::capability::identity::PrincipalId;
use teamy_capability_lab::capability::identity::ReadPowerOutput;
use teamy_capability_lab::capability::identity::ReadPowerRequest;
use teamy_capability_lab::capability::identity::Session;
use teamy_capability_lab::capability::identity::UnknownReason;
use teamy_capability_lab::capability::identity::VmId;
use teamy_capability_lab::capability::identity::assess;
use teamy_capability_lab::capability::identity::executor_call_count;
use teamy_capability_lab::capability::identity::synthetic_identity_fixture;

fn request(fixture: &IdentityFixture, index: usize, vm: &str) -> ReadPowerRequest {
    ReadPowerRequest {
        session: fixture.sessions[index].clone(),
        vm: VmId(vm.into()),
    }
}

#[test]
fn principal_is_distinct_from_device_and_environment() {
    let fixture = synthetic_identity_fixture();
    let desktop = assess(&fixture, &request(&fixture, 0, "jeff-on"));
    let phone = assess(&fixture, &request(&fixture, 1, "jeff-on"));
    assert_eq!(
        desktop,
        DisclosureAssessment::Visible(ReadPowerOutput {
            powered_on: Some(true)
        })
    );
    assert_eq!(phone, desktop);
    assert_eq!(
        assess(&fixture, &request(&fixture, 2, "jeff-on")),
        DisclosureAssessment::Denied {
            invoke: Permission::Denied,
            read_effect: Permission::Granted
        }
    );
    assert_eq!(
        assess(&fixture, &request(&fixture, 3, "jeff-on")),
        DisclosureAssessment::UnknownPermissions {
            invoke: Permission::Granted,
            read_effect: Permission::Unknown
        },
    );
    let mut unrecognized_instance = request(&fixture, 3, "jeff-on");
    if let Origin::Environment { instance, .. } = &mut unrecognized_instance.session.origin {
        instance.0 = "codex-instance-b".into();
    } else {
        panic!("fixture must use a concrete environment instance");
    }
    assert_eq!(
        assess(&fixture, &unrecognized_instance),
        DisclosureAssessment::Unknown(UnknownReason::UnrecognizedSession)
    );
    let forged_device = ReadPowerRequest {
        session: Session {
            principal: PrincipalId("alice".into()),
            origin: Origin::Phone(DeviceId("bob-phone".into())),
        },
        vm: VmId("jeff-on".into()),
    };
    assert_eq!(
        assess(&fixture, &forged_device),
        DisclosureAssessment::Unknown(UnknownReason::UnrecognizedSession)
    );
}

#[test]
fn selective_disclosure_preserves_unknowns_and_omits_other_properties() {
    let fixture = synthetic_identity_fixture();
    assert_eq!(
        assess(&fixture, &request(&fixture, 0, "not-jeff")),
        DisclosureAssessment::NotApplicable
    );
    assert_eq!(
        assess(&fixture, &request(&fixture, 0, "unknown-owner")),
        DisclosureAssessment::Unknown(UnknownReason::OwnerTag)
    );
    assert_eq!(
        assess(&fixture, &request(&fixture, 0, "unknown-power")),
        DisclosureAssessment::Visible(ReadPowerOutput { powered_on: None })
    );
    let output = assess(&fixture, &request(&fixture, 0, "jeff-on"));
    let json = facet_json::to_string(&output).unwrap();
    assert!(json.contains("powered_on"));
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

#[test]
fn independent_permissions_are_never_inferred_from_registration() {
    let mut fixture = synthetic_identity_fixture();
    let target = request(&fixture, 0, "jeff-on");
    fixture.permissions[0].invoke = Permission::Unknown;
    assert_eq!(
        assess(&fixture, &target),
        DisclosureAssessment::UnknownPermissions {
            invoke: Permission::Unknown,
            read_effect: Permission::Granted
        }
    );
    fixture.permissions[0].read_effect = Permission::Unknown;
    assert_eq!(
        assess(&fixture, &target),
        DisclosureAssessment::UnknownPermissions {
            invoke: Permission::Unknown,
            read_effect: Permission::Unknown,
        }
    );
    fixture.permissions[0].read_effect = Permission::Granted;
    fixture.permissions[0].invoke = Permission::Granted;
    fixture.permissions[0].read_effect = Permission::Unknown;
    assert_eq!(
        assess(&fixture, &target),
        DisclosureAssessment::UnknownPermissions {
            invoke: Permission::Granted,
            read_effect: Permission::Unknown
        }
    );
    fixture.permissions[0].read_effect = Permission::Denied;
    assert_eq!(
        assess(&fixture, &target),
        DisclosureAssessment::Denied {
            invoke: Permission::Granted,
            read_effect: Permission::Denied
        }
    );
    assert_eq!(
        assess(&fixture, &request(&fixture, 2, "not-jeff")),
        DisclosureAssessment::Unknown(UnknownReason::MissingPermission)
    );
    fixture.permissions[0].invoke = Permission::Unknown;
    assert_eq!(
        assess(&fixture, &target),
        DisclosureAssessment::Denied {
            invoke: Permission::Unknown,
            read_effect: Permission::Denied,
        }
    );
    fixture.permissions[0].invoke = Permission::Granted;
    fixture.permissions[0].read_effect = Permission::Granted;
    fixture.permissions.push(fixture.permissions[0].clone());
    assert_eq!(
        assess(&fixture, &target),
        DisclosureAssessment::Unknown(UnknownReason::ConflictingPermissions)
    );
}

#[test]
fn matching_registry_entry_is_metadata_and_assessment_never_invokes_executor() {
    let before = executor_call_count();
    assert!(known_thing_for_shape(ReadPowerRequest::SHAPE).is_some());
    assert!(known_thing_for_shape(ReadPowerOutput::SHAPE).is_some());
    assert!(
        known_functions()
            .iter()
            .any(|operation| std::ptr::eq(operation, ReadPowerRequest::REGISTERED_INTO_FUTURE))
    );
    assert_eq!(
        ReadPowerRequest::REGISTERED_INTO_FUTURE.effects,
        std::slice::from_ref(&Effect::Read)
    );
    let fixture = synthetic_identity_fixture();
    let result = assess(&fixture, &request(&fixture, 0, "jeff-on"));
    assert!(matches!(result, DisclosureAssessment::Visible(_)));
    assert_eq!(executor_call_count(), before);
}

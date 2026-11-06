use cloud_terrastodon_registry::known_functions;
use cloud_terrastodon_registry::known_thing_for_shape;
use facet::Facet;
use std::future::Ready;
use std::future::ready;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
use teamy_capability_lab::capability::planner::SearchBudget;

static INVOCATIONS: AtomicUsize = AtomicUsize::new(0);

#[derive(Facet)]
struct LocalInput;
#[derive(Facet)]
struct LocalOutput;

impl IntoFuture for LocalInput {
    type Output = eyre::Result<LocalOutput>;
    type IntoFuture = Ready<Self::Output>;
    fn into_future(self) -> Self::IntoFuture {
        INVOCATIONS.fetch_add(1, Ordering::SeqCst);
        ready(Ok(LocalOutput))
    }
}

cloud_terrastodon_registry::register_thing!(LocalInput);
cloud_terrastodon_registry::register_thing!(LocalOutput);
cloud_terrastodon_registry::register_into_future!(LocalInput => LocalOutput, effects = [Read]);

#[test]
fn downstream_registrations_are_linked_without_invoking_operation() {
    assert!(known_thing_for_shape(LocalInput::SHAPE).is_some());
    assert!(
        known_functions()
            .iter()
            .any(|entry| std::ptr::eq(entry, LocalInput::REGISTERED_INTO_FUTURE))
    );
    assert_eq!(INVOCATIONS.load(Ordering::SeqCst), 0);
}

#[test]
fn planning_does_not_execute_registered_requests() {
    let fixture = teamy_capability_lab::capability::synthetic_fixture();
    let _ =
        teamy_capability_lab::capability::planner::plan(&fixture, SearchBudget::default(), || {
            false
        })
        .unwrap();
    assert_eq!(INVOCATIONS.load(Ordering::SeqCst), 0);
}

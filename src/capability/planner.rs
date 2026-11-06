//! Bounded structural route enumeration with Scryer. No operation is invoked.
use super::EndpointId;
use super::Fixture;
use super::RouteStatus;
use super::SearchCompleteness;
use super::Transfer;
use super::ValidatedRoute;
use super::validate_route;
use facet::Facet;
use scryer_prolog::LeafAnswer;
use scryer_prolog::MachineBuilder;
use scryer_prolog::Term;
use std::collections::HashSet;
use std::fmt::Write;
use std::fmt::{self};

const RULES: &str = r"
:- module(capability_lab, [bounded_route/6]).
:- use_module(library(iso_ext), [call_with_inference_limit/3]).
:- use_module(library(dif), [dif/2]).
bounded_route(A, B, N, Path, Limit, Result) :- call_with_inference_limit(route(A, B, N, Path), Limit, Result).
route(A, B, N, Path) :- walk(A, B, N, [A], Path).
walk(A, B, N, Seen, [E|Rest]) :-
    N > 0, edge(E, A, Next), unseen(Next, Seen), N1 is N - 1,
    (Next = B, Rest = []; dif(Next, B), walk(Next, B, N1, [Next|Seen], Rest)).
unseen(_, []).
unseen(X, [H|T]) :- dif(X, H), unseen(X, T).
";

/// Hard bounds on each enumeration. Inferences are bounded per returned solution.
#[derive(Clone, Copy, Debug)]
pub struct SearchBudget {
    pub max_depth: usize,
    pub max_solutions: usize,
    pub max_inferences_per_solution: usize,
}

impl Default for SearchBudget {
    fn default() -> Self {
        Self {
            max_depth: 4,
            max_solutions: 32,
            max_inferences_per_solution: 100_000,
        }
    }
}

#[derive(Debug)]
pub enum PlanningError {
    DuplicateEndpoint(String),
    DuplicateTransfer(String),
    UnknownEndpoint(String),
    InvalidBudget,
    Prolog(String),
    UnexpectedAnswer(String),
}

impl fmt::Display for PlanningError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for PlanningError {}

#[derive(Debug, Facet)]
pub struct PlanningReport {
    pub artifact: String,
    pub source: String,
    pub target: String,
    pub completeness: SearchCompleteness,
    /// Ranked among returned candidates only; never evidence of global optimality if incomplete.
    pub candidates: Vec<ValidatedRoute>,
    pub execution_authorized: bool,
    pub diagram: Option<String>,
}

fn endpoint_index(fixture: &Fixture, id: &EndpointId) -> Result<usize, PlanningError> {
    fixture
        .endpoints
        .iter()
        .position(|endpoint| &endpoint.id == id)
        .ok_or_else(|| PlanningError::UnknownEndpoint(id.0.clone()))
}

/// Generate a Prolog program from opaque indices, never from user strings.
fn program(fixture: &Fixture) -> Result<String, PlanningError> {
    let mut ids = HashSet::new();
    for endpoint in &fixture.endpoints {
        if !ids.insert(&endpoint.id) {
            return Err(PlanningError::DuplicateEndpoint(endpoint.id.0.clone()));
        }
    }
    let mut transfer_ids = HashSet::new();
    let mut source = String::from(RULES);
    // Keep the predicate defined even when there are no transfers.
    source.push_str("\nedge(_, _, _) :- fail.\n");
    for (index, transfer) in fixture.transfers.iter().enumerate() {
        if !transfer_ids.insert(&transfer.id) {
            return Err(PlanningError::DuplicateTransfer(transfer.id.clone()));
        }
        let from = endpoint_index(fixture, &transfer.from)?;
        let to = endpoint_index(fixture, &transfer.to)?;
        // Writing to a String cannot fail; preserve a typed error if it ever does.
        writeln!(source, "edge(e{index}, n{from}, n{to}).")
            .map_err(|error| PlanningError::Prolog(error.to_string()))?;
    }
    Ok(source)
}

fn route_indices(term: &Term, count: usize) -> Result<Vec<usize>, PlanningError> {
    let Term::List(items) = term else {
        return Err(PlanningError::UnexpectedAnswer(format!("route: {term:?}")));
    };
    items
        .iter()
        .map(|item| {
            let Term::Atom(atom) = item else {
                return Err(PlanningError::UnexpectedAnswer(format!("edge: {item:?}")));
            };
            let index = atom
                .strip_prefix('e')
                .and_then(|text| text.parse::<usize>().ok())
                .filter(|&index| index < count)
                .ok_or_else(|| PlanningError::UnexpectedAnswer(format!("edge: {atom}")))?;
            Ok(index)
        })
        .collect()
}

/// Enumerate simple directed paths, then validate and rank them without invoking executors.
///
/// # Errors
/// Returns typed errors for invalid fixture identities, budget or Prolog answers.
pub fn plan(
    fixture: &Fixture,
    budget: SearchBudget,
    mut cancelled: impl FnMut() -> bool,
) -> Result<PlanningReport, PlanningError> {
    if budget.max_solutions == 0 || budget.max_inferences_per_solution == 0 {
        return Err(PlanningError::InvalidBudget);
    }
    let source_program = program(fixture)?;
    let start = endpoint_index(fixture, &fixture.artifact.location)?;
    let target = endpoint_index(fixture, &fixture.target)?;
    let mut report = PlanningReport {
        artifact: fixture.artifact.id.clone(),
        source: fixture.artifact.location.0.clone(),
        target: fixture.target.0.clone(),
        completeness: SearchCompleteness::Complete,
        candidates: Vec::new(),
        execution_authorized: false,
        diagram: None,
    };
    if cancelled() {
        report.completeness = SearchCompleteness::Cancelled;
        return Ok(report);
    }
    let mut machine = MachineBuilder::new()
        .with_toplevel(":- module('$toplevel', []).\n")
        .build();
    machine.load_module_string("capability_lab", source_program);
    let query = format!(
        "capability_lab:bounded_route(n{start}, n{target}, {}, Path, {}, Limit).",
        budget.max_depth, budget.max_inferences_per_solution
    );
    let mut answers = machine.run_query(query);
    loop {
        if cancelled() {
            report.completeness = SearchCompleteness::Cancelled;
            break;
        }
        let Some(answer) = answers.next() else {
            break;
        };
        match answer {
            Err(error) | Ok(LeafAnswer::Exception(error)) => {
                return Err(PlanningError::Prolog(format!("{error:?}")));
            }
            Ok(LeafAnswer::False) => break,
            Ok(LeafAnswer::LeafAnswer { bindings, .. }) => {
                if matches!(bindings.get("Limit"), Some(Term::Atom(value)) if value == "inference_limit_exceeded")
                {
                    report.completeness = SearchCompleteness::InferenceLimit;
                    break;
                }
                let path = bindings
                    .get("Path")
                    .ok_or_else(|| PlanningError::UnexpectedAnswer(format!("{bindings:?}")))?;
                let indices = route_indices(path, fixture.transfers.len())?;
                if report.candidates.len() == budget.max_solutions {
                    report.completeness = SearchCompleteness::SolutionLimit;
                    break;
                }
                let route: Vec<Transfer> = indices
                    .into_iter()
                    .map(|index| fixture.transfers[index].clone())
                    .collect();
                report.candidates.push(validate_route(
                    &fixture.endpoints,
                    &fixture.artifact,
                    &fixture.target,
                    &route,
                ));
            }
            Ok(other) => return Err(PlanningError::UnexpectedAnswer(format!("{other:?}"))),
        }
    }
    // A simple path cannot have more than |V|-1 edges. Below that bound we
    // conservatively report incomplete even if the current fixtures happen to fit.
    if report.completeness == SearchCompleteness::Complete
        && budget.max_depth < fixture.endpoints.len().saturating_sub(1)
    {
        report.completeness = SearchCompleteness::DepthLimit;
    }
    report.candidates.sort_by(|a, b| {
        fn rank(status: &RouteStatus) -> u8 {
            match status {
                RouteStatus::Ready => 0,
                RouteStatus::ApprovalNeeded { .. } => 1,
                RouteStatus::Blocked { .. } => 2,
            }
        }
        (rank(&a.status), a.estimated_cost, &a.transfers).cmp(&(
            rank(&b.status),
            b.estimated_cost,
            &b.transfers,
        ))
    });
    Ok(report)
}

use crate::capability::identity::ReadPowerRequest;
use crate::capability::identity::VmId;
use crate::capability::identity::assess;
use crate::capability::identity::synthetic_identity_fixture;
use crate::cli::output::CliOutput;
use arbitrary::Arbitrary;
use facet::Facet;

/// Assess the fixed synthetic principal and Jeff-tagged VM example.
#[derive(Facet, Arbitrary, Debug, PartialEq)]
pub struct DisclosureArgs;

impl DisclosureArgs {
    /// # Errors
    /// Returns an error if the fixed fixture has no sample session.
    pub fn invoke(self) -> eyre::Result<CliOutput> {
        let fixture = synthetic_identity_fixture();
        let session = fixture
            .sessions
            .first()
            .cloned()
            .ok_or_else(|| eyre::eyre!("synthetic identity fixture has no sample session"))?;
        let request = ReadPowerRequest {
            session,
            vm: VmId("jeff-on".into()),
        };
        Ok(CliOutput::facet(assess(&fixture, &request)))
    }
}

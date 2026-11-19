//! Bounded observational helpers; never used for the installation mutation worker.
use enzui_core::cancellation::{CancelCause, CancelToken};
use enzui_services::process_transport::{self, ProcessOutput, ProcessRequest, StdinPolicy};
use std::time::Duration;

pub(crate) fn request(program: &str, args: &[&str]) -> ProcessRequest {
    ProcessRequest::new(
        program,
        StdinPolicy::Null,
        512 * 1024,
        16 * 1024,
        Duration::from_secs(20),
    )
    .args(args.iter().copied())
    .inherit_environment()
}
pub(crate) fn run(request: ProcessRequest) -> Result<ProcessOutput, String> {
    process_transport::run(request, &CancelToken::new())
        .map_err(|_| "Inspection helper exceeded its bounds or is unavailable.".into())
}
struct CancelOnDrop(CancelToken);
impl Drop for CancelOnDrop {
    fn drop(&mut self) {
        self.0.cancel(CancelCause::Disconnected);
    }
}
pub(crate) async fn run_async(request: ProcessRequest) -> Result<ProcessOutput, String> {
    let guard = CancelOnDrop(CancelToken::new());
    let token = guard.0.share();
    tokio::task::spawn_blocking(move || process_transport::run(request, &token))
        .await
        .map_err(|_| "Inspection worker is unavailable.".to_owned())?
        .map_err(|_| "Inspection helper exceeded its bounds or is unavailable.".into())
}

use defguard_proto::proxy::{CoreResponse, core_response};

use crate::tests::common::HandlerTestContext;

pub(crate) fn assert_initial_info_received(response: &CoreResponse) {
    assert!(
        matches!(
            response.payload,
            Some(core_response::Payload::InitialInfo(_))
        ),
        "expected InitialInfo as first response from handler, got: {:?}",
        response.payload.as_ref().map(|p| std::mem::discriminant(p))
    );
}

/// Consume the `InitialInfo` message that the handler sends immediately after
/// establishing the bidi stream.  Most lifecycle tests call this before
/// injecting any business messages.
pub(crate) async fn complete_proxy_handshake(context: &mut HandlerTestContext) {
    let response = context.mock_proxy_mut().recv_outbound().await;
    assert_initial_info_received(&response);
}

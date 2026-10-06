use ace_core::io::NodeAddress;
use ace_doip::session::ActivationAuthProvider;
use ace_doip::session::ActivationDenialReason;
use ace_gateway::config::{CanNodeEntry, GatewayConfig};
use ace_gateway::gateway::DoipGateway;

const CAN_MAX_FRAME: usize = 8;
const ISOTP_MAX_NODES: usize = 4;
const TCP_MAX_FRAME: usize = 256;
const TCP_MAX_OUTBOX: usize = 4;
const CAN_MAX_OUTBOX: usize = 4;
const MAX_TESTERS: usize = 2;
const MAX_FRAME: usize = 256;
const UDS_MAX_FRAME: usize = 256;
const MAX_CONNECTION_ACTIONS: usize = 8;
const MAX_ACTIVATION_TYPES: usize = 2;
const MAX_NODES: usize = 4;

#[derive(Clone)]
struct AllowActivation;

impl ActivationAuthProvider for AllowActivation {
    fn authenticate(
        &mut self,
        _source_address: u16,
        _oem_data: &[u8],
    ) -> Result<(), ActivationDenialReason> {
        Ok(())
    }
}

type Gateway = DoipGateway<
    CAN_MAX_FRAME,
    ISOTP_MAX_NODES,
    TCP_MAX_FRAME,
    TCP_MAX_OUTBOX,
    CAN_MAX_OUTBOX,
    MAX_TESTERS,
    MAX_FRAME,
    UDS_MAX_FRAME,
    MAX_CONNECTION_ACTIONS,
    MAX_ACTIVATION_TYPES,
    MAX_NODES,
    AllowActivation,
>;

fn main() {
    let config: GatewayConfig<MAX_NODES, MAX_TESTERS, MAX_ACTIVATION_TYPES> =
        GatewayConfig::new(0x0E80)
            .with_tester(0x0E00)
            .with_node(CanNodeEntry {
                logical_address: 0x0001,
                request_can_id: 0x7E0,
                response_can_id: 0x7E8,
                functional_can_id: 0x7DF,
            });

    let gateway: Gateway = DoipGateway::new(config, AllowActivation, NodeAddress(0x0E80));

    assert_eq!(gateway.address(), &NodeAddress(0x0E80));
}

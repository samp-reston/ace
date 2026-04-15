use ace_macros::FrameCodec;
use ace_proto::doip::constants::{
    DOIP_ENTITY_STATUS_RESPONSE_MCTS_LEN, DOIP_ENTITY_STATUS_RESPONSE_MDS_LEN,
    DOIP_ENTITY_STATUS_RESPONSE_NCTS_LEN,
};

use crate::error::DoipError;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = DoipError)]
pub struct EntityStatusRequest {}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = DoipError)]
pub struct EntityStatusResponse {
    pub node_type: NodeType,
    pub max_concurrent_sockets: [u8; DOIP_ENTITY_STATUS_RESPONSE_MCTS_LEN],
    pub currently_open_sockets: [u8; DOIP_ENTITY_STATUS_RESPONSE_NCTS_LEN],
    pub max_data_size: [u8; DOIP_ENTITY_STATUS_RESPONSE_MDS_LEN],
}

#[repr(u8)]
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = DoipError)]
pub enum NodeType {
    #[frame(id = 0x00)]
    DoipGateway,
    #[frame(id = 0x01)]
    DoipNode,
    #[frame(id_pat = "0x02..=0xFF")]
    Reserved(u8),
}

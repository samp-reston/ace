use crate::UdsError;
use ace_macros::FrameCodec;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub struct RoutineControlRequest<'a> {
    pub routine_control_type: RoutineControlType,
    pub routine_identifier: [u8; 2],
    pub routine_control_option_record: &'a [u8],
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub struct RoutineControlResponse<'a> {
    pub routine_control_type: RoutineControlType,
    pub routine_identifier: [u8; 2],
    pub routine_info: Option<u8>,
    pub routine_status_record: &'a [u8],
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub enum RoutineControlType {
    #[frame(id = 0x01)]
    StartRoutine,
    #[frame(id = 0x02)]
    StopRoutine,
    #[frame(id = 0x03)]
    RequestRoutineResults,
}

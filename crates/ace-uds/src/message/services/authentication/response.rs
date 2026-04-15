use ace_macros::FrameCodec;

use crate::UdsError;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub enum AuthenticationResponse<'a> {
    #[frame(id = 0x00)]
    DeAuthenticateResponse(DeAuthenticateResponse),
    #[frame(id = 0x01)]
    VerifyCertificateUnidirectionalResponse(VerifyCertificateUnidirectionalResponse<'a>),
    #[frame(id = 0x02)]
    VerifyCertificateBidirectionalResponse(VerifyCertificateBidirectionalResponse<'a>),
    #[frame(id = 0x03)]
    ProofOfOwnershipResponse(ProofOfOwnershipResponse<'a>),
    #[frame(id = 0x04)]
    TransmitCertificateResponse(TransmitCertificateResponse),
    #[frame(id = 0x05)]
    RequestChallengeForAuthenticationResponse(RequestChallengeForAuthenticationResponse<'a>),
    #[frame(id = 0x06)]
    VerifyProofOfOwnershipUnidirectionalResponse(VerifyProofOfOwnershipUnidirectionalResponse<'a>),
    #[frame(id = 0x07)]
    VerifyProofOfOwnershipBidirectionalResponse(VerifyProofOfOwnershipBidirectionalResponse<'a>),
    #[frame(id = 0x08)]
    AuthenticationConfigurationResponse(AuthenticationConfigurationResponse),
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub enum AuthenticationReturnParameter {
    #[frame(id = 0x00)]
    RequestAccepted,
    #[frame(id = 0x01)]
    GeneralReject,
    #[frame(id = 0x02)]
    AuthenticationConfigurationAPCE,
    #[frame(id = 0x03)]
    AuthenticationConfigurationACRAsymmetric,
    #[frame(id = 0x04)]
    AuthenticationConfigurationACRSymmetric,
    #[frame(id_pat = "0x05..=0x0F | 0x14..=0x9F | 0xFF")]
    IsoSaeReserved(u8),
    #[frame(id = 0x10)]
    DeAuthenticationSuccessful,
    #[frame(id = 0x11)]
    CertificateVerifiedOwnershipVerificationNecessary,
    #[frame(id = 0x12)]
    OwnershipVerified,
    #[frame(id = 0x13)]
    CertificateVerified,
    #[frame(id_pat = "0xA0..=0xCF")]
    VehicleManufacturerSpecific(u8),
    #[frame(id_pat = "0xD0..=0xFE")]
    SystemSupplierSpecific(u8),
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub struct DeAuthenticateResponse {
    pub authentication_return_parameter: AuthenticationReturnParameter,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub struct VerifyCertificateUnidirectionalResponse<'a> {
    pub authentication_return_parameter: AuthenticationReturnParameter,
    pub length_of_challenge_server: u16,
    #[frame(length = "length_of_challenge_server as usize")]
    pub challenge_server: &'a [u8],
    pub length_of_ephemeral_public_key_server: u16,
    #[frame(length = "length_of_ephemeral_public_key_server as usize")]
    pub ephemeral_public_key_server: &'a [u8],
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub struct VerifyCertificateBidirectionalResponse<'a> {
    pub authentication_return_parameter: AuthenticationReturnParameter,
    pub length_of_challenge_server: u16,
    #[frame(length = "length_of_challenge_server as usize")]
    pub challenge_server: &'a [u8],
    pub length_of_certificate_server: u16,
    #[frame(length = "length_of_certificate_server as usize")]
    pub certificate_server: &'a [u8],
    pub length_of_proof_of_ownership_server: u16,
    #[frame(length = "length_of_proof_of_ownership_server as usize")]
    pub proof_of_ownership_server: &'a [u8],
    pub length_of_ephemeral_public_key_server: u16,
    #[frame(length = "length_of_ephemeral_public_key_server as usize")]
    pub ephemeral_public_key_server: &'a [u8],
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub struct ProofOfOwnershipResponse<'a> {
    pub authentication_return_parameter: AuthenticationReturnParameter,
    pub length_of_session_key_info: u16,
    #[frame(length = "length_of_session_key_info as usize")]
    pub session_key_info: &'a [u8],
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub struct TransmitCertificateResponse {
    pub authentication_return_parameter: AuthenticationReturnParameter,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub struct RequestChallengeForAuthenticationResponse<'a> {
    pub authentication_return_parameter: AuthenticationReturnParameter,
    pub algorithm_indicator: [u8; 16],
    pub length_of_challenge_server: u16,
    #[frame(length = "length_of_challenge_server as usize")]
    pub challenge_server: &'a [u8],
    pub length_of_needed_additional_parameter: u16,
    #[frame(length = "length_of_needed_additional_parameter as usize")]
    pub needed_additional_parameter: &'a [u8],
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub struct VerifyProofOfOwnershipUnidirectionalResponse<'a> {
    pub authentication_return_parameter: AuthenticationReturnParameter,
    pub algorithm_indicator: [u8; 16],
    pub length_of_session_key_info: u16,
    #[frame(length = "length_of_session_key_info as usize")]
    pub session_key_info: &'a [u8],
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub struct VerifyProofOfOwnershipBidirectionalResponse<'a> {
    pub authentication_return_parameter: AuthenticationReturnParameter,
    pub algorithm_indicator: [u8; 16],
    pub length_of_proof_of_ownership_server: u16,
    #[frame(length = "length_of_proof_of_ownership_server as usize")]
    pub proof_of_ownership_server: &'a [u8],
    pub length_of_session_key_info: u16,
    #[frame(length = "length_of_session_key_info as usize")]
    pub session_key_info: &'a [u8],
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub struct AuthenticationConfigurationResponse {
    pub authentication_return_parameter: AuthenticationReturnParameter,
}

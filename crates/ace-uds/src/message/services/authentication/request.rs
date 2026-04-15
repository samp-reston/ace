use crate::UdsError;
use ace_macros::FrameCodec;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub enum AuthenticationRequest<'a> {
    #[frame(id = 0x00)]
    DeAuthenticateRequest(DeAuthenticateRequest),
    #[frame(id = 0x01)]
    VerifyCertificateUnidirectionalRequest(VerifyCertificateUnidirectionalRequest<'a>),
    #[frame(id = 0x02)]
    VerifyCertificateBidirectionalRequest(VerifyCertificateBidirectionalRequest<'a>),
    #[frame(id = 0x03)]
    ProofOfOwnershipRequest(ProofOfOwnershipRequest<'a>),
    #[frame(id = 0x04)]
    TransmitCertificateRequest(TransmitCertificateRequest<'a>),
    #[frame(id = 0x05)]
    RequestChallengeForAuthenticationRequest(RequestChallengeForAuthenticationRequest),
    #[frame(id = 0x06)]
    VerifyProofOfOwnershipUnidirectionalRequest(VerifyProofOfOwnershipUnidirectionalRequest<'a>),
    #[frame(id = 0x07)]
    VerifyProofOfOwnershipBidirectionalRequest(VerifyProofOfOwnershipBidirectionalRequest<'a>),
    #[frame(id = 0x08)]
    AuthenticationConfigurationRequest(AuthenticationConfigurationRequest),
    #[frame(id_pat = "0x09..=0x7F")]
    IsoSaeReserved(u8),
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub struct DeAuthenticateRequest {}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub struct VerifyCertificateUnidirectionalRequest<'a> {
    pub communication_configuration: u8,
    pub length_of_certificate_client: u16,
    #[frame(length = "length_of_certificate_client as usize")]
    pub certificate_client: &'a [u8],
    pub length_of_challenge_client: u16,
    #[frame(length = "length_of_challenge_client as usize")]
    pub challenge_client: &'a [u8],
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub struct VerifyCertificateBidirectionalRequest<'a> {
    pub communication_configuration: u8,
    pub length_of_certificate_client: u16,
    #[frame(length = "length_of_certificate_client as usize")]
    pub certificate_client: &'a [u8],
    pub length_of_challenge_client: u16,
    #[frame(length = "length_of_challenge_client as usize")]
    pub challenge_client: &'a [u8],
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub struct ProofOfOwnershipRequest<'a> {
    pub length_of_proof_of_ownership_client: u16,
    #[frame(length = "length_of_proof_of_ownership_client as usize")]
    pub proof_of_ownership_client: &'a [u8],
    pub length_of_ephemeral_public_key_client: u16,
    #[frame(length = "length_of_ephemeral_public_key_client as usize")]
    pub ephemeral_public_key_client: &'a [u8],
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub struct TransmitCertificateRequest<'a> {
    pub certificate_evaluation_id: u16,
    pub length_of_certificate_data: u16,
    #[frame(length = "length_of_certificate_data as usize")]
    pub certificate_data: &'a [u8],
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub struct RequestChallengeForAuthenticationRequest {
    pub communication_configuration: u8,
    pub algorithm_indicator: [u8; 16],
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub struct VerifyProofOfOwnershipUnidirectionalRequest<'a> {
    pub algorithm_indicator: [u8; 16],
    pub length_of_proof_of_ownership_client: u16,
    #[frame(length = "length_of_proof_of_ownership_client as usize")]
    pub proof_of_ownership_client: &'a [u8],
    pub length_of_challenge_client: u16,
    #[frame(length = "length_of_challenge_client as usize")]
    pub challenge_client: &'a [u8],
    pub length_of_additional_parameter: u16,
    #[frame(length = "length_of_additional_parameter as usize")]
    pub additional_parameter: &'a [u8],
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub struct VerifyProofOfOwnershipBidirectionalRequest<'a> {
    pub algorithm_indicator: [u8; 16],
    pub length_of_proof_of_ownership_client: u16,
    #[frame(length = "length_of_proof_of_ownership_client as usize")]
    pub proof_of_ownership_client: &'a [u8],
    pub length_of_challenge_client: u16,
    #[frame(length = "length_of_challenge_client as usize")]
    pub challenge_client: &'a [u8],
    pub length_of_additional_parameter: u16,
    #[frame(length = "length_of_additional_parameter as usize")]
    pub additional_parameter: &'a [u8],
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, FrameCodec)]
#[frame(error = UdsError)]
pub struct AuthenticationConfigurationRequest {}

use anchor_lang::prelude::*;

#[error_code]
pub enum ReceiverError {
    #[msg("The tuple emitter chain, emitter doesn't match one of the valid data sources.")]
    InvalidDataSource,
    #[msg("The posted VAA account has the wrong owner.")]
    WrongVaaOwner,
    #[msg("The posted VAA has wrong magic number.")]
    PostedVaaHeaderWrongMagicNumber,
    #[msg("An error occurred when deserializing the VAA.")]
    DeserializeVaaFailed,
    #[msg("An error occurred when deserializing the updates.")]
    DeserializeUpdateFailed,
    #[msg("An error occurred when deserializing the message")]
    DeserializeMessageFailed,
    #[msg("Received an invalid wormhole message")]
    InvalidWormholeMessage,
    #[msg("Received an invalid price update")]
    InvalidPriceUpdate,
    #[msg("This type of message is not supported currently")]
    UnsupportedMessageType,
    #[msg("The signer is not authorized to perform this governance action")]
    GovernanceAuthorityMismatch,
    #[msg("The signer is not authorized to accept the governance authority")]
    TargetGovernanceAuthorityMismatch,
    #[msg("The governance authority needs to request a transfer first")]
    NonexistentGovernanceAuthorityTransferRequest,
    #[msg("Funds are insufficient to pay the receiving fee")]
    InsufficientFunds,
}

use anchor_lang::prelude::*;
use constant_product_curve::CurveError;
#[error_code]
pub enum AmmError {
    #[msg("Invalid fee percent, must be between 0 and 10000 (inclusive)")]
    FeePercentErr,
    #[msg("Default error")]
    DefaultError,
    #[msg("Pool is locked")]
    PoolLocked,
    #[msg("Invalid amount")]
    InvalidAmount,
    #[msg("Slippage exceeded")]
    SlippageExceeded,
    #[msg("Overflow")]
    Overflow,
    #[msg("Underflow")]
    Underflow,
    #[msg("Invalid token")]
    InvalidToken,
    #[msg("Liquidity less than minimum")]
    LiquidityLessThanMinimum,
    #[msg("No liquidity")]
    NoLiquidity,
    #[msg("Curve error")]
    CurveError,
    #[msg("Bump error")]
    BumpError,
    #[msg("Unauthorized")]
    Unauthorized,
    #[msg("No authority set for mint")]
    NoAuthoritySet,
    #[msg("Invalid precision, must be between 0 and 9 (inclusive)")]
    InvalidPrecision,
    #[msg("Insufficient funds")]
    InsufficientFunds,
    #[msg("Zero balance")]
    ZeroBalance,
    #[msg("Invalid fee")]
    InvalidFee,
}

impl From<CurveError> for AmmError {
    fn from(error: CurveError) -> Self {
        match error {
            CurveError::Overflow => AmmError::Overflow,
            CurveError::Underflow => AmmError::Underflow,
            CurveError::InvalidFeeAmount => AmmError::InvalidFee,
            CurveError::InvalidPrecision => AmmError::InvalidPrecision,
            CurveError::InsufficientBalance => AmmError::InsufficientFunds,
            CurveError::ZeroBalance => AmmError::ZeroBalance,
            CurveError::SlippageLimitExceeded => AmmError::SlippageExceeded,
        }
    }
}

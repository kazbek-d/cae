pub mod erc20;
pub mod lp;

use alloy::sol;

sol! {
    #[sol(rpc)]
    interface IERC20Metadata {
        function symbol() external view returns (string);
        function decimals() external view returns (uint8);
    }
}

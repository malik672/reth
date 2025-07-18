//! Reth RPC `eth_` API implementation
//!
//! ## Feature Flags
//!
//! - `client`: Enables JSON-RPC client support.

#![doc(
    html_logo_url = "https://raw.githubusercontent.com/paradigmxyz/reth/main/assets/reth-docs.png",
    html_favicon_url = "https://avatars0.githubusercontent.com/u/97369466?s=256",
    issue_tracker_base_url = "https://github.com/paradigmxyz/reth/issues/"
)]
#![cfg_attr(not(test), warn(unused_crate_dependencies))]
#![cfg_attr(docsrs, feature(doc_cfg, doc_auto_cfg))]

pub mod bundle;
pub mod core;
pub mod ext;
pub mod filter;
pub mod helpers;
pub mod node;
pub mod pubsub;
pub mod types;

pub use bundle::{EthBundleApiServer, EthCallBundleApiServer};
pub use core::{EthApiServer, FullEthApiServer};
pub use ext::L2EthApiExtServer;
pub use filter::{EngineEthFilter, EthFilterApiServer, QueryLimits};
pub use node::{RpcNodeCore, RpcNodeCoreExt};
pub use pubsub::EthPubSubApiServer;
pub use reth_rpc_convert::*;
pub use reth_rpc_eth_types::error::{
    AsEthApiError, FromEthApiError, FromEvmError, IntoEthApiError,
};
pub use types::{EthApiTypes, FullEthApiTypes, RpcBlock, RpcHeader, RpcReceipt, RpcTransaction};

#[cfg(feature = "client")]
pub use bundle::{EthBundleApiClient, EthCallBundleApiClient};
#[cfg(feature = "client")]
pub use core::EthApiClient;
#[cfg(feature = "client")]
pub use ext::L2EthApiExtClient;
#[cfg(feature = "client")]
pub use filter::EthFilterApiClient;

use reth_trie_common as _;

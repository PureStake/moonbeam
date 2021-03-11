// Copyright 2019-2020 PureStake Inc.
// This file is part of Moonbeam.

// Moonbeam is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Moonbeam is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Moonbeam.  If not, see <http://www.gnu.org/licenses/>.

use jsonrpc_core::futures::Future;

use ethereum_types::H160;
use futures::{compat::Compat, future::BoxFuture};
use jsonrpc_derive::rpc;
use serde::{Deserialize, Serialize};
#[rpc(server)]
pub trait Trace {
	#[rpc(name = "trace_filter")]
	fn filter(
		&self,
		filter: FilterRequest,
	) -> Compat<BoxFuture<'static, jsonrpc_core::Result<FilterResponse>>>;
}

#[derive(Clone, Eq, PartialEq, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FilterRequest {
	/// (optional?) From this block.
	pub from_block: Option<u32>,

	/// (optional?) To this block.
	pub to_block: Option<u32>,

	/// (optional) Sent from these addresses.
	pub from_address: Vec<H160>,

	/// (optional) Sent to these addresses.
	pub to_address: Vec<H160>,

	/// (optional) The offset trace number
	pub after: u32,

	/// (optional) Integer number of traces to display in a batch.
	pub count: u32,
}

#[derive(Clone, Eq, PartialEq, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FilterResponse {
	// TODO
}

// Copyright 2019-2021 PureStake Inc.
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
use futures::{
	compat::Compat,
	future::{BoxFuture, TryFutureExt},
	FutureExt, SinkExt, StreamExt,
};
use jsonrpc_core::Result as RpcResult;
pub use moonbeam_rpc_core_debug::{Debug as DebugT, DebugServer, TraceParams};

use tokio::{
	self,
	sync::{oneshot, Semaphore},
};

use ethereum_types::{H128, H256};
use fc_rpc::{frontier_backend_client, internal_err};
use fp_rpc::EthereumRuntimeRPCApi;
use moonbeam_rpc_primitives_debug::{single, DebugRuntimeApi};
use sc_client_api::backend::Backend;
use sp_api::{BlockId, HeaderT, ProvideRuntimeApi};
use sp_block_builder::BlockBuilder;
use sp_blockchain::{
	Backend as BlockchainBackend, Error as BlockChainError, HeaderBackend, HeaderMetadata,
};
use sp_runtime::traits::Block as BlockT;
use sp_utils::mpsc::TracingUnboundedSender;
use std::{future::Future, marker::PhantomData, str::FromStr, sync::Arc};

pub type Responder = oneshot::Sender<RpcResult<single::TransactionTrace>>;
pub type DebugRequester = TracingUnboundedSender<((H256, Option<TraceParams>, u32), Responder)>;

pub struct Debug {
	pub requester: DebugRequester,
}

impl Debug {
	pub fn new(requester: DebugRequester) -> Self {
		Self { requester }
	}
}

impl DebugT for Debug {
	/// Handler for `debug_traceTransaction` request. Communicates with the service-defined task
	/// using channels.
	fn trace_transaction(
		&self,
		transaction_hash: H256,
		params: Option<TraceParams>,
	) -> Compat<BoxFuture<'static, RpcResult<single::TransactionTrace>>> {
		let mut requester = self.requester.clone();
		let rnd: u32 = rand::random();
		async move {
			log::debug!(
				target: "debug-test",
				"-------> START {}",
				rnd,
			);
			let (tx, rx) = oneshot::channel();
			// Send a message from the rpc handler to the service level task.
			requester
				.send(((transaction_hash, params, rnd), tx))
				.await
				.map_err(|err| {
					internal_err(format!(
						"failed to send request to debug service : {:?}",
						err
					))
				})?;

			// Receive a message from the service level task and send the rpc response.
			let r = rx.await;
			log::debug!(
				target: "debug-test",
				"-------> END {}",
				rnd,
			);
			r.map_err(|err| internal_err(format!("debug service dropped the channel : {:?}", err)))?
		}
		.boxed()
		.compat()
	}
}

pub struct DebugHandler<B: BlockT, C, BE>(PhantomData<(B, C, BE)>);

impl<B, C, BE> DebugHandler<B, C, BE>
where
	BE: Backend<B> + 'static,
	C: ProvideRuntimeApi<B>,
	C: HeaderMetadata<B, Error = BlockChainError> + HeaderBackend<B>,
	C: Send + Sync + 'static,
	B: BlockT<Hash = H256> + Send + Sync + 'static,
	C::Api: BlockBuilder<B>,
	C::Api: DebugRuntimeApi<B>,
	C::Api: EthereumRuntimeRPCApi<B>,
{
	/// Task spawned at service level that listens for messages on the rpc channel and spawns
	/// blocking tasks using a permit pool.
	pub fn task(
		client: Arc<C>,
		backend: Arc<BE>,
		frontier_backend: Arc<fc_db::Backend<B>>,
		permit_pool: Arc<Semaphore>,
	) -> (impl Future<Output = ()>, DebugRequester) {
		let (tx, mut rx): (DebugRequester, _) =
			sp_utils::mpsc::tracing_unbounded("debug-requester");

		let fut = async move {
			loop {
				if let Some(((transaction_hash, params, rnd), response_tx)) = rx.next().await {
					let client = client.clone();
					let backend = backend.clone();
					let frontier_backend = frontier_backend.clone();
					let permit_pool = permit_pool.clone();
					// Note on spawned tasks https://tokio.rs/tokio/tutorial/spawning#tasks.
					//
					// Substrate uses the default value for `core_threads` (number of cores of the
					// machine running the node) and `max_threads` (512 total).
					//
					// Task below is spawned in the substrate's built tokio::Runtime, so they share
					// the same thread pool as the rest of the service-spawned tasks. Additionally,
					// blocking tasks use a more restrictive permit pool shared by trace modules.
					// https://docs.rs/tokio/0.2.23/tokio/sync/struct.Semaphore.html
					tokio::task::spawn(async move {
						let _ = response_tx.send(
							async {
								let _permit = permit_pool.acquire().await;
								tokio::task::spawn_blocking(move || {
									Self::handle_request(
										client.clone(),
										backend.clone(),
										frontier_backend.clone(),
										transaction_hash,
										params,
										rnd,
									)
								})
								.await
								.map_err(|e| {
									internal_err(format!(
										"Internal error on spawned task : {:?}",
										e
									))
								})?
							}
							.await,
						);
					});
				}
			}
		};
		(fut, tx)
	}
	/// Replays a transaction in the Runtime at a given block height.
	///
	/// In order to succesfully reproduce the result of the original transaction we need a correct
	/// state to replay over.
	///
	/// Substrate allows to apply extrinsics in the Runtime and thus creating an overlayed state.
	/// This overlayed changes will live in-memory for the lifetime of the ApiRef.
	fn handle_request(
		client: Arc<C>,
		backend: Arc<BE>,
		frontier_backend: Arc<fc_db::Backend<B>>,
		transaction_hash: H256,
		params: Option<TraceParams>,
		rnd: u32,
	) -> RpcResult<single::TransactionTrace> {
		let (hash, index) = match frontier_backend_client::load_transactions::<B, C>(
			client.as_ref(),
			frontier_backend.as_ref(),
			transaction_hash,
		) {
			Ok(Some((hash, index))) => (hash, index as usize),
			Ok(None) => return Err(internal_err("Transaction hash not found".to_string())),
			Err(e) => return Err(e),
		};

		let reference_id =
			match frontier_backend_client::load_hash::<B>(frontier_backend.as_ref(), hash) {
				Ok(Some(hash)) => hash,
				Ok(_) => return Err(internal_err("Block hash not found".to_string())),
				Err(e) => return Err(e),
			};
		// Get ApiRef. This handle allow to keep changes between txs in an internal buffer.
		let api = client.runtime_api();
		// Get Blockchain backend
		let blockchain = backend.blockchain();
		// Get the header I want to work with.
		let header = client.header(reference_id).unwrap().unwrap();
		// Get parent blockid.
		let parent_block_id = BlockId::Hash(*header.parent_hash());

		// Get the extrinsics.
		let ext = blockchain.body(reference_id).unwrap().unwrap();
		// Get the block that contains the requested transaction.
		let reference_block = match api.current_block(&reference_id) {
			Ok(block) => block,
			Err(e) => return Err(internal_err(format!("Runtime block call failed: {:?}", e))),
		};

		// Set trace type
		let trace_type = match params {
			Some(TraceParams {
				tracer: Some(tracer),
				..
			}) => {
				let hash: H128 = sp_io::hashing::twox_128(&tracer.as_bytes()).into();
				let blockscout_hash = H128::from_str("0x94d9f08796f91eb13a2e82a6066882f7").unwrap();
				if hash == blockscout_hash {
					single::TraceType::CallList
				} else {
					return Err(internal_err(format!(
						"javascript based tracing is not available (hash :{:?})",
						hash
					)));
				}
			}
			Some(params) => single::TraceType::Raw {
				disable_storage: params.disable_storage.unwrap_or(false),
				disable_memory: params.disable_memory.unwrap_or(false),
				disable_stack: params.disable_stack.unwrap_or(false),
			},
			_ => single::TraceType::Raw {
				disable_storage: false,
				disable_memory: false,
				disable_stack: false,
			},
		};

		// Get the actual ethereum transaction.
		if let Some(block) = reference_block {
			let transactions = block.transactions;
			if let Some(transaction) = transactions.get(index) {
				log::debug!(
					target: "debug-test",
					"-------> START RUNTIME {}",
					rnd,
				);
				let runtime_call = client.runtime_api().trace_transaction(
					&parent_block_id,
					ext,
					&transaction,
					trace_type,
				);

				log::debug!(
					target: "debug-test",
					"-------> END RUNTIME {}",
					rnd,
				);
				return runtime_call
					.map_err(|e| internal_err(format!("Runtime api access error: {:?}", e)))?
					.map_err(|e| internal_err(format!("DispatchError: {:?}", e)));
			}
		}
		Err(internal_err("Runtime block call failed".to_string()))
	}
}

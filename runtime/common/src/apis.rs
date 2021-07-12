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

#[macro_export]
macro_rules! impl_runtime_apis_plus_common {
	{$($custom:tt)*} => {
		impl_runtime_apis! {
			$($custom)*

			impl sp_api::Core<Block> for Runtime {
				fn version() -> RuntimeVersion {
					VERSION
				}

				fn execute_block(block: Block) {
					Executive::execute_block(block)
				}

				fn initialize_block(header: &<Block as BlockT>::Header) {
					Executive::initialize_block(header)
				}
			}

			impl sp_api::Metadata<Block> for Runtime {
				fn metadata() -> OpaqueMetadata {
					Runtime::metadata().into()
				}
			}

			impl sp_block_builder::BlockBuilder<Block> for Runtime {
				fn apply_extrinsic(extrinsic: <Block as BlockT>::Extrinsic) -> ApplyExtrinsicResult {
					Executive::apply_extrinsic(extrinsic)
				}

				fn finalize_block() -> <Block as BlockT>::Header {
					Executive::finalize_block()
				}

				fn inherent_extrinsics(
					data: sp_inherents::InherentData,
				) -> Vec<<Block as BlockT>::Extrinsic> {
					data.create_extrinsics()
				}

				fn check_inherents(
					block: Block,
					data: sp_inherents::InherentData,
				) -> sp_inherents::CheckInherentsResult {
					data.check_extrinsics(&block)
				}
			}

			impl sp_offchain::OffchainWorkerApi<Block> for Runtime {
				fn offchain_worker(header: &<Block as BlockT>::Header) {
					Executive::offchain_worker(header)
				}
			}

			impl sp_session::SessionKeys<Block> for Runtime {
				fn decode_session_keys(
					encoded: Vec<u8>,
				) -> Option<Vec<(Vec<u8>, sp_core::crypto::KeyTypeId)>> {
					opaque::SessionKeys::decode_into_raw_public_keys(&encoded)
				}

				fn generate_session_keys(seed: Option<Vec<u8>>) -> Vec<u8> {
					opaque::SessionKeys::generate(seed)
				}
			}

			impl frame_system_rpc_runtime_api::AccountNonceApi<Block, AccountId, Index> for Runtime {
				fn account_nonce(account: AccountId) -> Index {
					System::account_nonce(account)
				}
			}

			impl moonbeam_rpc_primitives_debug::DebugRuntimeApi<Block> for Runtime {
				fn trace_transaction(
					header: &<Block as BlockT>::Header,
					extrinsics: Vec<<Block as BlockT>::Extrinsic>,
					transaction: &EthereumTransaction,
					trace_type: moonbeam_rpc_primitives_debug::single::TraceType,
				) -> Result<
					moonbeam_rpc_primitives_debug::single::TransactionTrace,
					sp_runtime::DispatchError,
				> {
					use moonbeam_evm_tracer::{CallListTracer, RawTracer};
					use moonbeam_rpc_primitives_debug::single::TraceType;

					// Explicit initialize.
					// Needed because https://github.com/paritytech/substrate/pull/8953
					Executive::initialize_block(header);

					// Apply the a subset of extrinsics: all the substrate-specific or ethereum
					// transactions that preceded the requested transaction.
					for ext in extrinsics.into_iter() {
						let _ = match &ext.function {
							Call::Ethereum(transact(t)) => {
								if t == transaction {
									return match trace_type {
										TraceType::Raw {
											disable_storage,
											disable_memory,
											disable_stack,
										} => Ok(RawTracer::new(
											disable_storage,
											disable_memory,
											disable_stack,
										)
										.trace(|| Executive::apply_extrinsic(ext))
										.0
										.into_tx_trace()),
										TraceType::CallList => Ok(CallListTracer::default()
											.trace(|| Executive::apply_extrinsic(ext))
											.0
											.into_tx_trace()),
									};
								} else {
									Executive::apply_extrinsic(ext)
								}
							}
							_ => Executive::apply_extrinsic(ext),
						};
					}

					Err(sp_runtime::DispatchError::Other(
						"Failed to find Ethereum transaction among the extrinsics.",
					))
				}

				fn trace_block(
					header: &<Block as BlockT>::Header,
					extrinsics: Vec<<Block as BlockT>::Extrinsic>,
				) -> Result<
					Vec<moonbeam_rpc_primitives_debug::block::TransactionTrace>,
					sp_runtime::DispatchError,
				> {
					use moonbeam_evm_tracer::CallListTracer;
					use moonbeam_rpc_primitives_debug::{
						block, single, CallResult, CreateResult, CreateType,
					};

					// Explicit initialize.
					// Needed because https://github.com/paritytech/substrate/pull/8953
					Executive::initialize_block(header);

					let mut config = <Runtime as pallet_evm::Config>::config().clone();
					config.estimate = true;

					let mut traces = vec![];
					let mut eth_tx_index = 0;

					// Apply all extrinsics. Ethereum extrinsics are traced.
					for ext in extrinsics.into_iter() {
						match &ext.function {
							Call::Ethereum(transact(_transaction)) => {
								let tx_traces = CallListTracer::default()
									.trace(|| Executive::apply_extrinsic(ext))
									.0
									.into_tx_trace();

								let tx_traces = match tx_traces {
									single::TransactionTrace::CallList(t) => t,
									_ => return Err(
										sp_runtime::DispatchError::Other("Runtime API error")
									),
								};

								// Convert traces from "single" format to "block" format.
								let mut tx_traces: Vec<_> = tx_traces
									.into_iter()
									.map(|trace| match trace.inner {
										single::CallInner::Call {
											input,
											to,
											res,
											call_type,
										} => block::TransactionTrace {
											action: block::TransactionTraceAction::Call {
												call_type,
												from: trace.from,
												gas: trace.gas,
												input,
												to,
												value: trace.value,
											},
											// Can't be known here, must be inserted upstream.
											block_hash: H256::default(),
											// Can't be known here, must be inserted upstream.
											block_number: 0,
											output: match res {
												CallResult::Output(output) => {
													block::TransactionTraceOutput::Result(
														block::TransactionTraceResult::Call {
															gas_used: trace.gas_used,
															output,
														},
													)
												}
												CallResult::Error(error) => {
													block::TransactionTraceOutput::Error(error)
												}
											},
											subtraces: trace.subtraces,
											trace_address: trace.trace_address,
											// Can't be known here, must be inserted upstream.
											transaction_hash: H256::default(),
											transaction_position: eth_tx_index,
										},
										single::CallInner::Create { init, res } => {
											block::TransactionTrace {
												action: block::TransactionTraceAction::Create {
													creation_method: CreateType::Create,
													from: trace.from,
													gas: trace.gas,
													init,
													value: trace.value,
												},
												// Can't be known here, must be inserted upstream.
												block_hash: H256::default(),
												// Can't be known here, must be inserted upstream.
												block_number: 0,
												output: match res {
													CreateResult::Success {
														created_contract_address_hash,
														created_contract_code,
													} => block::TransactionTraceOutput::Result(
														block::TransactionTraceResult::Create {
															gas_used: trace.gas_used,
															code: created_contract_code,
															address: created_contract_address_hash,
														},
													),
													CreateResult::Error { error } => {
														block::TransactionTraceOutput::Error(error)
													}
												},
												subtraces: trace.subtraces,
												trace_address: trace.trace_address,
												// Can't be known here, must be inserted upstream.
												transaction_hash: H256::default(),
												transaction_position: eth_tx_index,
											}
										}
										single::CallInner::SelfDestruct {
											balance,
											refund_address,
										} => block::TransactionTrace {
											action: block::TransactionTraceAction::Suicide {
												address: trace.from,
												balance,
												refund_address,
											},
											// Can't be known here, must be inserted upstream.
											block_hash: H256::default(),
											// Can't be known here, must be inserted upstream.
											block_number: 0,
											output: block::TransactionTraceOutput::Result(
												block::TransactionTraceResult::Suicide,
											),
											subtraces: trace.subtraces,
											trace_address: trace.trace_address,
											// Can't be known here, must be inserted upstream.
											transaction_hash: H256::default(),
											transaction_position: eth_tx_index,
										},
									})
									.collect();

								traces.append(&mut tx_traces);

								eth_tx_index += 1;
							}
							_ => {
								let _ = Executive::apply_extrinsic(ext);
							}
						};
					}

					Ok(traces)
				}
			}

			impl moonbeam_rpc_primitives_txpool::TxPoolRuntimeApi<Block> for Runtime {
				fn extrinsic_filter(
					xts_ready: Vec<<Block as BlockT>::Extrinsic>,
					xts_future: Vec<<Block as BlockT>::Extrinsic>,
				) -> TxPoolResponse {
					TxPoolResponse {
						ready: xts_ready
							.into_iter()
							.filter_map(|xt| match xt.function {
								Call::Ethereum(transact(t)) => Some(t),
								_ => None,
							})
							.collect(),
						future: xts_future
							.into_iter()
							.filter_map(|xt| match xt.function {
								Call::Ethereum(transact(t)) => Some(t),
								_ => None,
							})
							.collect(),
					}
				}
			}

			impl fp_rpc::EthereumRuntimeRPCApi<Block> for Runtime {
				fn chain_id() -> u64 {
					<Runtime as pallet_evm::Config>::ChainId::get()
				}

				fn account_basic(address: H160) -> EVMAccount {
					EVM::account_basic(&address)
				}

				fn gas_price() -> U256 {
					<Runtime as pallet_evm::Config>::FeeCalculator::min_gas_price()
				}

				fn account_code_at(address: H160) -> Vec<u8> {
					EVM::account_codes(address)
				}

				fn author() -> H160 {
					Ethereum::find_author()
				}

				fn storage_at(address: H160, index: U256) -> H256 {
					let mut tmp = [0u8; 32];
					index.to_big_endian(&mut tmp);
					EVM::account_storages(address, H256::from_slice(&tmp[..]))
				}

				fn call(
					from: H160,
					to: H160,
					data: Vec<u8>,
					value: U256,
					gas_limit: U256,
					gas_price: Option<U256>,
					nonce: Option<U256>,
					estimate: bool,
				) -> Result<pallet_evm::CallInfo, sp_runtime::DispatchError> {
					let config = if estimate {
						let mut config = <Runtime as pallet_evm::Config>::config().clone();
						config.estimate = true;
						Some(config)
					} else {
						None
					};

					<Runtime as pallet_evm::Config>::Runner::call(
						from,
						to,
						data,
						value,
						gas_limit.low_u64(),
						gas_price,
						nonce,
						config
							.as_ref()
							.unwrap_or_else(|| <Runtime as pallet_evm::Config>::config()),
					)
					.map_err(|err| err.into())
				}

				fn create(
					from: H160,
					data: Vec<u8>,
					value: U256,
					gas_limit: U256,
					gas_price: Option<U256>,
					nonce: Option<U256>,
					estimate: bool,
				) -> Result<pallet_evm::CreateInfo, sp_runtime::DispatchError> {
					let config = if estimate {
						let mut config = <Runtime as pallet_evm::Config>::config().clone();
						config.estimate = true;
						Some(config)
					} else {
						None
					};

					#[allow(clippy::or_fun_call)] // suggestion not helpful here
					<Runtime as pallet_evm::Config>::Runner::create(
						from,
						data,
						value,
						gas_limit.low_u64(),
						gas_price,
						nonce,
						config
							.as_ref()
							.unwrap_or(<Runtime as pallet_evm::Config>::config()),
					)
					.map_err(|err| err.into())
				}

				fn current_transaction_statuses() -> Option<Vec<TransactionStatus>> {
					Ethereum::current_transaction_statuses()
				}

				fn current_block() -> Option<pallet_ethereum::Block> {
					Ethereum::current_block()
				}

				fn current_receipts() -> Option<Vec<pallet_ethereum::Receipt>> {
					Ethereum::current_receipts()
				}

				fn current_all() -> (
					Option<pallet_ethereum::Block>,
					Option<Vec<pallet_ethereum::Receipt>>,
					Option<Vec<TransactionStatus>>,
				) {
					(
						Ethereum::current_block(),
						Ethereum::current_receipts(),
						Ethereum::current_transaction_statuses(),
					)
				}
			}

			impl pallet_transaction_payment_rpc_runtime_api::TransactionPaymentApi<Block, Balance>
			for Runtime {
				fn query_info(
					uxt: <Block as BlockT>::Extrinsic,
					len: u32,
				) -> pallet_transaction_payment_rpc_runtime_api::RuntimeDispatchInfo<Balance> {
					TransactionPayment::query_info(uxt, len)
				}

				fn query_fee_details(
					uxt: <Block as BlockT>::Extrinsic,
					len: u32,
				) -> pallet_transaction_payment::FeeDetails<Balance> {
					TransactionPayment::query_fee_details(uxt, len)
				}
			}

			impl nimbus_primitives::AuthorFilterAPI<Block, nimbus_primitives::NimbusId> for Runtime {
				fn can_author(
					author: nimbus_primitives::NimbusId,
					slot: u32,
					parent_header: &<Block as BlockT>::Header
				) -> bool {
					// The Moonbeam runtimes use an entropy source that needs to do some accounting
					// work during block initialization. Therefore we initialize it here to match
					// the state it will be in when the next block is being executed.
					use frame_support::traits::OnInitialize;
					System::initialize(
						&(parent_header.number + 1),
						&parent_header.hash(),
						&parent_header.digest,
						frame_system::InitKind::Inspection
					);
					RandomnessCollectiveFlip::on_initialize(System::block_number());

					// And now the actual prediction call
					AuthorInherent::can_author(&author, &slot)
				}
			}

			impl cumulus_primitives_core::CollectCollationInfo<Block> for Runtime {
				fn collect_collation_info() -> cumulus_primitives_core::CollationInfo {
					ParachainSystem::collect_collation_info()
				}
			}

			#[cfg(feature = "runtime-benchmarks")]
			impl frame_benchmarking::Benchmark<Block> for Runtime {
				fn dispatch_benchmark(
					config: frame_benchmarking::BenchmarkConfig,
				) -> Result<Vec<frame_benchmarking::BenchmarkBatch>, sp_runtime::RuntimeString> {
					use frame_benchmarking::{
						add_benchmark, BenchmarkBatch, Benchmarking, TrackedStorageKey,
					};

					use frame_system_benchmarking::Pallet as SystemBench;
					impl frame_system_benchmarking::Config for Runtime {}

					use pallet_crowdloan_rewards::Pallet as PalletCrowdloanRewardsBench;
					use parachain_staking::Pallet as ParachainStakingBench;
					let whitelist: Vec<TrackedStorageKey> = vec![];

					let mut batches = Vec::<BenchmarkBatch>::new();
					let params = (&config, &whitelist);

					add_benchmark!(
						params,
						batches,
						parachain_staking,
						ParachainStakingBench::<Runtime>
					);
					add_benchmark!(
						params,
					 	batches,
					 	pallet_crowdloan_rewards,
					 	PalletCrowdloanRewardsBench::<Runtime>
					);
					add_benchmark!(params, batches, frame_system, SystemBench::<Runtime>);

					if batches.is_empty() {
						return Err("Benchmark not found for this pallet.".into());
					}
					Ok(batches)
				}
			}
		}
	};
}

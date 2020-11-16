// This file is part of Substrate.

// Copyright (C) 2020 Parity Technologies (UK) Ltd.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use crate::{
	Error,
	wasm::{Runtime, RuntimeToken},
};
use frame_support::weights::Weight;
use sp_runtime::DispatchError;
use sp_core::crypto::UncheckedFrom;
use sp_std::marker::PhantomData;

pub use frame_system::Trait as SysTrait;
pub use pallet_contracts_primitives::ReturnFlags;
pub use crate::exec::Ext;

pub type Result<T> = sp_std::result::Result<T, DispatchError>;

pub trait ChainExtension {
	fn call<E: Ext>(func_id: u32, env: Environment<E, state::Init>) -> Result<RetVal>
	where
		<E::T as SysTrait>::AccountId: UncheckedFrom<<E::T as SysTrait>::Hash> + AsRef<[u8]>;

	fn enabled() -> bool {
		true
	}
}

impl ChainExtension for () {
	fn call<E: Ext>(_func_id: u32, mut env: Environment<E, state::Init>) -> Result<RetVal>
	where
		<E::T as SysTrait>::AccountId: UncheckedFrom<<E::T as SysTrait>::Hash> + AsRef<[u8]>,
	{
		env.ext().caller();
		Err(Error::<E::T>::NoChainExtension.into())
	}

	fn enabled() -> bool {
		false
	}
}

pub enum RetVal {
	Converging(u32),
	Diverging{flags: ReturnFlags, data: Vec<u8>},
}

struct Inner<'a, 'b, E: Ext> {
	runtime: &'a mut Runtime::<'b, E>,
	input_ptr: u32,
	input_len: u32,
	output_ptr: u32,
	output_len_ptr: u32,
}

pub struct Environment<'a, 'b, E: Ext, S: state::State> {
	inner: Inner<'a, 'b, E>,
	state: PhantomData<S>,
}

pub(crate) fn environment<'a, 'b, E: Ext>(
	runtime: &'a mut Runtime::<'b, E>,
	input_ptr: u32,
	input_len: u32,
	output_ptr: u32,
	output_len_ptr: u32,
) -> Environment<'a, 'b, E, state::Init>
{
	Environment {
		inner: Inner {
			runtime,
			input_ptr,
			input_len,
			output_ptr,
			output_len_ptr,
		},
		state: PhantomData,
	}
}

impl<'a, 'b, E: Ext, S: state::State> Environment<'a, 'b, E, S>
where
	<E::T as SysTrait>::AccountId: UncheckedFrom<<E::T as SysTrait>::Hash> + AsRef<[u8]>,
{
	pub fn charge_weight(&mut self, amount: Weight) -> Result<()> {
		self.inner.runtime.charge_gas(RuntimeToken::ChainExtension(amount))
	}

	pub fn ext(&mut self) -> &mut E {
		self.inner.runtime.ext()
	}
}

impl<'a, 'b, E: Ext> Environment<'a, 'b, E, state::Init> {
	pub fn only_in(self) -> Environment<'a, 'b, E, state::OnlyIn> {
		Environment {
			inner: self.inner,
			state: PhantomData,
		}
	}

	pub fn prim_in_buf_out(self) -> Environment<'a, 'b, E, state::PrimInBufOut> {
		Environment {
			inner: self.inner,
			state: PhantomData,
		}
	}

	pub fn buf_in_buf_out(self) -> Environment<'a, 'b, E, state::BufInBufOut> {
		Environment {
			inner: self.inner,
			state: PhantomData,
		}
	}
}

impl<'a, 'b, E: Ext, S: state::PrimIn> Environment<'a, 'b, E, S> {
	pub fn val0(&self) -> u32 {
		self.inner.input_ptr
	}

	pub fn val1(&self) -> u32 {
		self.inner.input_len
	}
}

impl<'a, 'b, E: Ext, S: state::PrimOut> Environment<'a, 'b, E, S> {
	pub fn val2(&self) -> u32 {
		self.inner.output_ptr
	}

	pub fn val3(&self) -> u32 {
		self.inner.output_len_ptr
	}
}

impl<'a, 'b, E: Ext, S: state::BufIn> Environment<'a, 'b, E, S>
where
	<E::T as SysTrait>::AccountId: UncheckedFrom<<E::T as SysTrait>::Hash> + AsRef<[u8]>,
{
	pub fn read(&self) -> Result<Vec<u8>> {
		self.inner.runtime.read_sandbox_memory(self.inner.input_ptr, self.inner.input_len)
	}
}

impl<'a, 'b, E: Ext, S: state::BufOut> Environment<'a, 'b, E, S>
where
	<E::T as SysTrait>::AccountId: UncheckedFrom<<E::T as SysTrait>::Hash> + AsRef<[u8]>,
{
	pub fn write(
		&mut self,
		buf: &[u8],
		allow_skip: bool,
		weight_per_byte: Option<Weight>,
	) -> Result<()> {
		self.inner.runtime.write_sandbox_output(
			self.inner.output_ptr,
			self.inner.output_len_ptr,
			buf,
			allow_skip,
			|len| {
				weight_per_byte.map(|w| RuntimeToken::ChainExtension(w.saturating_mul(len.into())))
			},
		)
	}
}

mod state {
	pub trait State {}

	pub trait PrimIn: State {}
	pub trait PrimOut: State {}
	pub trait BufIn: State {}
	pub trait BufOut: State {}

	pub enum Init {}
	pub enum OnlyIn {}
	pub enum PrimInBufOut {}
	pub enum BufInBufOut {}

	impl State for Init {}
	impl State for OnlyIn {}
	impl State for PrimInBufOut {}
	impl State for BufInBufOut {}

	impl PrimIn for OnlyIn {}
	impl PrimOut for OnlyIn {}
	impl PrimIn for PrimInBufOut {}
	impl BufOut for PrimInBufOut {}
	impl BufIn for BufInBufOut {}
	impl BufOut for BufInBufOut {}
}

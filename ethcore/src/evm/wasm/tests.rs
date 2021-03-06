use std::path::PathBuf;
use std::fs::File;
use std::io::Read;
use std::sync::Arc;

use ethcore_logger::init_log;
use super::super::tests::{FakeExt, FakeCall, FakeCallType};
use super::WasmInterpreter;
use evm::{self, Evm, GasLeft};
use action_params::{ActionParams, ActionValue};
use util::{U256, H256, Address};

fn load_sample(name: &str) -> Vec<u8> {
	let mut path = PathBuf::from("./res/wasm-tests/compiled");
	path.push(name);
	let mut file = File::open(path).expect(&format!("File {} for test to exist", name));
	let mut data = vec![];
	file.read_to_end(&mut data).expect(&format!("Test {} to load ok", name));
	data
}

fn test_finalize(res: Result<GasLeft, evm::Error>) -> Result<U256, evm::Error> {
	match res {
		Ok(GasLeft::Known(gas)) => Ok(gas),
		Ok(GasLeft::NeedsReturn{..}) => unimplemented!(), // since ret is unimplemented.
		Err(e) => Err(e),
	}
}

fn wasm_interpreter() -> WasmInterpreter {
	WasmInterpreter::new().expect("wasm interpreter to create without errors")
}

/// Empty contract does almost nothing except producing 1 (one) local node debug log message
#[test]
fn empty() {
	init_log();

	let code = load_sample("empty.wasm");
	let address: Address = "0f572e5295c57f15886f9b263e2f6d2d6c7b5ec6".parse().unwrap();

	let mut params = ActionParams::default();
	params.address = address.clone();
	params.gas = U256::from(100_000);
	params.code = Some(Arc::new(code));
	let mut ext = FakeExt::new();

	let gas_left = {
		let mut interpreter = wasm_interpreter();
		test_finalize(interpreter.exec(params, &mut ext)).unwrap()
	};

	assert_eq!(gas_left, U256::from(99_996));
}

// This test checks if the contract deserializes payload header properly.
//   Contract is provided with receiver(address), sender, origin and transaction value
//   logger.wasm writes all these provided fixed header fields to some arbitrary storage keys.
#[test]
fn logger() {
	init_log();

	let code = load_sample("logger.wasm");
	let address: Address = "0f572e5295c57f15886f9b263e2f6d2d6c7b5ec6".parse().unwrap();
	let sender: Address = "0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d".parse().unwrap();
	let origin: Address = "0102030405060708090a0b0c0d0e0f1011121314".parse().unwrap();

	let mut params = ActionParams::default();
	params.address = address.clone();
	params.sender = sender.clone();
	params.origin = origin.clone();
	params.gas = U256::from(100_000);
	params.value = ActionValue::transfer(1_000_000_000);
	params.code = Some(Arc::new(code));
	let mut ext = FakeExt::new();

	let gas_left = {
		let mut interpreter = wasm_interpreter();
		test_finalize(interpreter.exec(params, &mut ext)).unwrap()
	};

	println!("ext.store: {:?}", ext.store);
	assert_eq!(gas_left, U256::from(99581));
	let address_val: H256 = address.into();
	assert_eq!(
		ext.store.get(&"0100000000000000000000000000000000000000000000000000000000000000".parse().unwrap()).expect("storage key to exist"),
		&address_val,
		"Logger sets 0x01 key to the provided address"
	);
	let sender_val: H256 = sender.into();
	assert_eq!(
		ext.store.get(&"0200000000000000000000000000000000000000000000000000000000000000".parse().unwrap()).expect("storage key to exist"),
		&sender_val,
		"Logger sets 0x02 key to the provided sender"
	);
	let origin_val: H256 = origin.into();
	assert_eq!(
		ext.store.get(&"0300000000000000000000000000000000000000000000000000000000000000".parse().unwrap()).expect("storage key to exist"),
		&origin_val,
		"Logger sets 0x03 key to the provided origin"
	);
	assert_eq!(
		U256::from(ext.store.get(&"0400000000000000000000000000000000000000000000000000000000000000".parse().unwrap()).expect("storage key to exist")),
		U256::from(1_000_000_000),
		"Logger sets 0x04 key to the trasferred value"
	);
}

// This test checks if the contract can allocate memory and pass pointer to the result stream properly.
//   1. Contract is being provided with the call descriptor ptr
//   2. Descriptor ptr is 16 byte length
//   3. The last 8 bytes of call descriptor is the space for the contract to fill [result_ptr[4], result_len[4]]
//      if it has any result.
#[test]
fn identity() {
	init_log();

	let code = load_sample("identity.wasm");
	let sender: Address = "01030507090b0d0f11131517191b1d1f21232527".parse().unwrap();

	let mut params = ActionParams::default();
	params.sender = sender.clone();
	params.gas = U256::from(100_000);
	params.code = Some(Arc::new(code));
	let mut ext = FakeExt::new();

	let (gas_left, result) = {
		let mut interpreter = wasm_interpreter();
		let result = interpreter.exec(params, &mut ext).expect("Interpreter to execute without any errors");
		match result {
			GasLeft::Known(_) => { panic!("Identity contract should return payload"); },
			GasLeft::NeedsReturn { gas_left: gas, data: result, apply_state: _apply } => (gas, result.to_vec()),
		}
	};

	assert_eq!(gas_left, U256::from(99_689));

	assert_eq!(
		Address::from_slice(&result),
		sender,
		"Idenity test contract does not return the sender passed"
	);
}

// Dispersion test sends byte array and expect the contract to 'disperse' the original elements with
// their modulo 19 dopant. 
// The result is always twice as long as the input.
// This also tests byte-perfect memory allocation and in/out ptr lifecycle. 
#[test]
fn dispersion() {
	init_log();

	let code = load_sample("dispersion.wasm");

	let mut params = ActionParams::default();
	params.gas = U256::from(100_000);
	params.code = Some(Arc::new(code));
	params.data = Some(vec![
		0u8, 125, 197, 255, 19
	]);
	let mut ext = FakeExt::new();

	let (gas_left, result) = {
		let mut interpreter = wasm_interpreter();
		let result = interpreter.exec(params, &mut ext).expect("Interpreter to execute without any errors");
		match result {
			GasLeft::Known(_) => { panic!("Dispersion routine should return payload"); },
			GasLeft::NeedsReturn { gas_left: gas, data: result, apply_state: _apply } => (gas, result.to_vec()),
		}
	};

	assert_eq!(gas_left, U256::from(99_402));

	assert_eq!(
		result,
		vec![0u8, 0, 125, 11, 197, 7, 255, 8, 19, 0]
	);
}

#[test]
fn suicide_not() {
	init_log();

	let code = load_sample("suicidal.wasm");

	let mut params = ActionParams::default();
	params.gas = U256::from(100_000);
	params.code = Some(Arc::new(code));
	params.data = Some(vec![
		0u8
	]);
	let mut ext = FakeExt::new();

	let (gas_left, result) = {
		let mut interpreter = wasm_interpreter();
		let result = interpreter.exec(params, &mut ext).expect("Interpreter to execute without any errors");
		match result {
			GasLeft::Known(_) => { panic!("Suicidal contract should return payload when had not actualy killed himself"); },
			GasLeft::NeedsReturn { gas_left: gas, data: result, apply_state: _apply } => (gas, result.to_vec()),
		}
	};

	assert_eq!(gas_left, U256::from(99_703));

	assert_eq!(
		result,
		vec![0u8]
	);	
}

#[test]
fn suicide() {
	init_log();

	let code = load_sample("suicidal.wasm");

	let refund: Address = "01030507090b0d0f11131517191b1d1f21232527".parse().unwrap();
	let mut params = ActionParams::default();
	params.gas = U256::from(100_000);
	params.code = Some(Arc::new(code));

	let mut args = vec![127u8];
	args.extend(refund.to_vec());
	params.data = Some(args);

	let mut ext = FakeExt::new();

	let gas_left = {
		let mut interpreter = wasm_interpreter();
		let result = interpreter.exec(params, &mut ext).expect("Interpreter to execute without any errors");
		match result {
			GasLeft::Known(gas) => gas,
			GasLeft::NeedsReturn { .. } => {
				panic!("Suicidal contract should not return anything when had killed itself");
			},
		}
	};

	assert_eq!(gas_left, U256::from(99_747));
	assert!(ext.suicides.contains(&refund));
}

#[test]
fn create() {
	init_log();

	let mut params = ActionParams::default();
	params.gas = U256::from(100_000);
	params.code = Some(Arc::new(load_sample("creator.wasm")));
	params.data = Some(vec![0u8, 2, 4, 8, 16, 32, 64, 128]);
	params.value = ActionValue::transfer(1_000_000_000);

	let mut ext = FakeExt::new();

	let gas_left = {
		let mut interpreter = wasm_interpreter();
		let result = interpreter.exec(params, &mut ext).expect("Interpreter to execute without any errors");
		match result {
			GasLeft::Known(gas) => gas,
			GasLeft::NeedsReturn { .. } => {
				panic!("Create contract should not return anthing because ext always fails on creation");
			},
		}
	};

	trace!(target: "wasm", "fake_calls: {:?}", &ext.calls);
	assert!(ext.calls.contains(
		&FakeCall {
			call_type: FakeCallType::Create,
			gas: U256::from(99_778),
			sender_address: None,
			receive_address: None,
			value: Some(1_000_000_000.into()),
			data: vec![0u8, 2, 4, 8, 16, 32, 64, 128],
			code_address: None,
		}
	));
	assert_eq!(gas_left, U256::from(99_768));
}
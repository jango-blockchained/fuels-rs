contract;

use std::storage::storage_api::{read, write};
use std::context::msg_amount;

struct MyType {
    x: u64,
    y: u64,
}

#[allow(dead_code)]
struct Person {
    name: str[4],
}

#[allow(dead_code)]
enum State {
    A: (),
    B: (),
    C: (),
}

abi TestContract {
    #[storage(write)]
    fn initialize_counter(value: u64) -> u64;
    #[storage(read, write)]
    fn increment_counter(amount: u64) -> u64;
    #[storage(read)]
    fn read_counter() -> u64;
    // ANCHOR: low_level_call
    #[storage(write)]
    fn set_value_multiple_complex(a: MyStruct, b: str[4]);
    // ANCHOR_END: low_level_call
    #[storage(read)]
    fn get_str_value() -> str[4];
    #[storage(read)]
    fn get_bool_value() -> bool;
    fn get(x: u64, y: u64) -> u64;
    fn get_alt(x: MyType) -> MyType;
    fn get_single(x: u64) -> u64;
    fn array_of_structs(p: [Person; 2]) -> [Person; 2];
    fn array_of_enums(p: [State; 2]) -> [State; 2];
    fn get_array(p: [u64; 2]) -> [u64; 2];
    #[payable]
    fn get_msg_amount() -> u64;
    fn new() -> u64;
}

storage {
    counter: u64 = 0,
    value_str: str[4] = __to_str_array("none"),
    value_bool: bool = false,
}

pub struct MyStruct {
    a: bool,
    b: [u64; 3],
}

impl TestContract for Contract {
    // ANCHOR: msg_amount
    #[payable]
    fn get_msg_amount() -> u64 {
        msg_amount()
    }
    // ANCHOR_END: msg_amount
    #[storage(write)]
    fn initialize_counter(value: u64) -> u64 {
        storage.counter.write(value);

        value
    }

    /// This method will read the counter from storage, increment it
    /// and write the incremented value to storage
    #[storage(read, write)]
    fn increment_counter(amount: u64) -> u64 {
        let incremented = storage.counter.read() + amount;
        storage.counter.write(incremented);

        incremented
    }

    #[storage(read)]
    fn read_counter() -> u64 {
        storage.counter.read()
    }

    #[storage(write)]
    fn set_value_multiple_complex(a: MyStruct, b: str[4]) {
        storage.counter.write(a.b[1]);
        storage.value_str.write(b);
        storage.value_bool.write(a.a);
    }

    #[storage(read)]
    fn get_str_value() -> str[4] {
        storage.value_str.read()
    }

    #[storage(read)]
    fn get_bool_value() -> bool {
        storage.value_bool.read()
    }

    fn get(x: u64, y: u64) -> u64 {
        x + y
    }

    fn get_alt(t: MyType) -> MyType {
        t
    }

    fn get_single(x: u64) -> u64 {
        x
    }

    fn array_of_structs(p: [Person; 2]) -> [Person; 2] {
        p
    }

    fn array_of_enums(p: [State; 2]) -> [State; 2] {
        p
    }

    fn get_array(p: [u64; 2]) -> [u64; 2] {
        p
    }

    fn new() -> u64 {
        12345u64
    }
}

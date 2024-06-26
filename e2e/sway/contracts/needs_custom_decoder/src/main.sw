contract;

impl AbiEncode for [u8; 1000] {
    fn abi_encode(self, buffer: Buffer) -> Buffer {
        let mut buffer = buffer;
        let mut i = 0;
        while i < 1000 {
            buffer = self[i].abi_encode(buffer);
            i += 1;
        };

        buffer
    }
}

abi MyContract {
    fn i_return_a_1k_el_array() -> [u8; 1000];
    fn i_log_a_1k_el_array();
}

impl MyContract for Contract {
    fn i_log_a_1k_el_array() {
        let arr: [u8; 1000] = [0; 1000];
        log(arr);
    }

    fn i_return_a_1k_el_array() -> [u8; 1000] {
        [0; 1000]
    }
}

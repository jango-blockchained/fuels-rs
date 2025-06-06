#[cfg(test)]
mod tests {
    use fuels::{
        prelude::Result,
        types::{Bits256, EvmAddress, Identity},
    };

    #[tokio::test]
    async fn bytes32() -> Result<()> {
        // ANCHOR: bytes32
        use std::str::FromStr;

        use fuels::types::Bytes32;

        // Zeroed Bytes32
        let b256 = Bytes32::zeroed();

        // Grab the inner `[u8; 32]` from
        // `Bytes32` by dereferencing (i.e. `*`) it.
        assert_eq!([0u8; 32], *b256);

        // From a `[u8; 32]`.
        // ANCHOR: array_to_bytes32
        let my_slice = [1u8; 32];
        let b256 = Bytes32::new(my_slice);
        // ANCHOR_END: array_to_bytes32
        assert_eq!([1u8; 32], *b256);

        // From a hex string.
        // ANCHOR: hex_string_to_bytes32
        let hex_str = "0x0000000000000000000000000000000000000000000000000000000000000000";
        let b256 = Bytes32::from_str(hex_str)?;
        // ANCHOR_END: hex_string_to_bytes32
        assert_eq!([0u8; 32], *b256);
        // ANCHOR_END: bytes32

        // ANCHOR: bytes32_format
        let b256_string = b256.to_string();
        let b256_hex_string = format!("{b256:#x}");
        // ANCHOR_END: bytes32_format

        assert_eq!(hex_str[2..], b256_string);
        assert_eq!(hex_str, b256_hex_string);

        // ANCHOR: bytes32_to_str
        let _str_from_bytes32: &str = b256.to_string().as_str();
        // ANCHOR_END: bytes32_to_str

        Ok(())
    }
    #[tokio::test]
    async fn address() -> Result<()> {
        // ANCHOR: address
        use std::str::FromStr;

        use fuels::types::Address;

        // Zeroed Bytes32
        let address = Address::zeroed();

        // Grab the inner `[u8; 32]` from
        // `Bytes32` by dereferencing (i.e. `*`) it.
        assert_eq!([0u8; 32], *address);

        // From a `[u8; 32]`.
        // ANCHOR: array_to_address
        let my_slice = [1u8; 32];
        let address = Address::new(my_slice);
        // ANCHOR_END: array_to_address
        assert_eq!([1u8; 32], *address);

        // From a string.
        // ANCHOR: hex_string_to_address
        let hex_str = "0x0000000000000000000000000000000000000000000000000000000000000000";
        let address = Address::from_str(hex_str)?;
        // ANCHOR_END: hex_string_to_address
        assert_eq!([0u8; 32], *address);
        // ANCHOR_END: address

        // ANCHOR: address_to_identity
        let _identity_from_address = Identity::Address(address);
        // ANCHOR_END: address_to_identity

        // ANCHOR: address_to_str
        let _str_from_address: &str = address.to_string().as_str();
        // ANCHOR_END: address_to_str

        // ANCHOR: address_to_bits256
        let bits_256 = Bits256(address.into());
        // ANCHOR_END: address_to_bits256

        // ANCHOR: b256_to_evm_address
        let _evm_address = EvmAddress::from(bits_256);
        // ANCHOR_END: b256_to_evm_address

        Ok(())
    }

    #[tokio::test]
    async fn asset_id() -> Result<()> {
        // ANCHOR: asset_id
        use std::str::FromStr;

        use fuels::types::AssetId;

        // Zeroed Bytes32
        let asset_id = AssetId::zeroed();

        // Grab the inner `[u8; 32]` from
        // `Bytes32` by dereferencing (i.e. `*`) it.
        assert_eq!([0u8; 32], *asset_id);

        // From a `[u8; 32]`.
        // ANCHOR: array_to_asset_id
        let my_slice = [1u8; 32];
        let asset_id = AssetId::new(my_slice);
        // ANCHOR_END: array_to_asset_id
        assert_eq!([1u8; 32], *asset_id);

        // From a string.
        // ANCHOR: string_to_asset_id
        let hex_str = "0x0000000000000000000000000000000000000000000000000000000000000000";
        let asset_id = AssetId::from_str(hex_str)?;
        // ANCHOR_END: string_to_asset_id
        assert_eq!([0u8; 32], *asset_id);
        // ANCHOR_END: asset_id
        Ok(())
    }

    #[tokio::test]
    async fn contract_id() -> Result<()> {
        // ANCHOR: contract_id
        use std::str::FromStr;

        use fuels::types::ContractId;

        // Zeroed Bytes32
        let contract_id = ContractId::zeroed();

        // Grab the inner `[u8; 32]` from
        // `Bytes32` by dereferencing (i.e. `*`) it.
        assert_eq!([0u8; 32], *contract_id);

        // From a `[u8; 32]`.
        // ANCHOR: array_to_contract_id
        let my_slice = [1u8; 32];
        let contract_id = ContractId::new(my_slice);
        // ANCHOR_END: array_to_contract_id
        assert_eq!([1u8; 32], *contract_id);

        // From a string.
        // ANCHOR: string_to_contract_id
        let hex_str = "0x0000000000000000000000000000000000000000000000000000000000000000";
        let contract_id = ContractId::from_str(hex_str)?;
        // ANCHOR_END: string_to_contract_id
        assert_eq!([0u8; 32], *contract_id);
        // ANCHOR_END: contract_id

        // ANCHOR: contract_id_to_identity
        let _identity_from_contract_id = Identity::ContractId(contract_id);
        // ANCHOR_END: contract_id_to_identity

        // ANCHOR: contract_id_to_str
        let _str_from_contract_id: &str = contract_id.to_string().as_str();
        // ANCHOR_END: contract_id_to_str

        Ok(())
    }

    #[tokio::test]
    async fn type_conversion() -> Result<()> {
        // ANCHOR: type_conversion
        use fuels::types::{AssetId, ContractId};

        let contract_id = ContractId::new([1u8; 32]);

        let asset_id: AssetId = AssetId::new(*contract_id);

        assert_eq!([1u8; 32], *asset_id);
        // ANCHOR_END: type_conversion

        // ANCHOR: asset_id_to_str
        let _str_from_asset_id: &str = asset_id.to_string().as_str();
        // ANCHOR_END: asset_id_to_str

        // ANCHOR: contract_id_to_bits256
        let _contract_id_to_bits_256 = Bits256(contract_id.into());
        // ANCHOR_END: contract_id_to_bits256

        // ANCHOR: asset_id_to_bits256
        let _asset_id_to_bits_256 = Bits256(asset_id.into());
        // ANCHOR_END: asset_id_to_bits256

        Ok(())
    }

    #[tokio::test]
    async fn unused_generics() -> Result<()> {
        use fuels::prelude::*;
        abigen!(Contract(
            name = "MyContract",
            abi = "e2e/sway/types/contracts/generics/out/release/generics-abi.json"
        ));

        // ANCHOR: unused_generics_struct
        assert_eq!(
            <StructUnusedGeneric<u16, u32>>::new(15),
            StructUnusedGeneric {
                field: 15,
                _unused_generic_0: std::marker::PhantomData,
                _unused_generic_1: std::marker::PhantomData
            }
        );
        // ANCHOR_END: unused_generics_struct

        let my_enum = <EnumUnusedGeneric<u32, u64>>::One(15);
        // ANCHOR: unused_generics_enum
        match my_enum {
            EnumUnusedGeneric::One(_value) => {}
            EnumUnusedGeneric::IgnoreMe(..) => panic!("Will never receive this variant"),
        }
        // ANCHOR_END: unused_generics_enum

        Ok(())
    }
}

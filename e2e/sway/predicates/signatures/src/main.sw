predicate;

use std::{
    b512::B512,
    crypto::{
        message::Message,
        secp256k1::Secp256k1,
    },
    inputs::input_predicate_data,
};

fn extract_public_key_and_match(signature: B512, expected_public_key: b256) -> u64 {
    let signature = Secp256k1::from(signature);

    if let Result::Ok(pub_key_sig) = signature.address(Message::from(b256::zero()))
    {
        if pub_key_sig == Address::from(expected_public_key) {
            return 1;
        }
    }

    0
}

fn main(signatures: [B512; 3]) -> bool {
    let public_keys = [
        0xd58573593432a30a800f97ad32f877425c223a9e427ab557aab5d5bb89156db0,
        0x14df7c7e4e662db31fe2763b1734a3d680e7b743516319a49baaa22b2032a857,
        0x3ff494fb136978c3125844625dad6baf6e87cdb1328c8a51f35bda5afe72425c,
    ];

    let mut matched_keys = 0;

    matched_keys = extract_public_key_and_match(signatures[0], public_keys[0]);
    matched_keys = matched_keys + extract_public_key_and_match(signatures[1], public_keys[1]);
    matched_keys = matched_keys + extract_public_key_and_match(signatures[2], public_keys[2]);

    matched_keys > 1
}

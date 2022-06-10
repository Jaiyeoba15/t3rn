use sp_std::convert::TryInto;
pub use ethereum_types::{H256, H160, U256};
pub use rlp::{RlpStream, Rlp, Encodable, Decodable, DecoderError};
pub use keccak_hash::{keccak};
use sp_std::convert::TryFrom;
use frame_support::pallet_prelude::TypeInfo;
// pub use ethers_core::types::{H256 as Hash, Signature, SignatureError};
use codec::{Encode, Decode, MaxEncodedLen};
use sp_std::vec::Vec;
use crate::Error;

use crate::crypto::Signature;

#[derive(Debug, Clone, Eq, PartialEq, Encode, Decode, TypeInfo)]
pub struct Header {
    pub chain_id: u64,
    pub parent_hash: H256,
    pub sha3_uncles: H256,
    pub miner: H160,
    pub state_root: H256,
    pub transactions_root: H256,
    pub receipts_root: H256,
    pub logs_bloom: LogsBloom,
    pub difficulty: u64,
    pub number: u64,
    pub gas_limit: U256,
    pub gas_used: U256,
    pub timestamp: u64,
    pub extra: [u8; 32],
    pub validators: Option<[H160; 21]>,
    pub signature: [u8; 65],
    pub mix_hash: H256,
    pub nonce: [u8; 8],
}

#[derive(Debug, Encode, Decode, TypeInfo)]
pub struct ValidatorSet {
    pub last_update: u64,
    pub validators: [H160; 21]
}

impl MaxEncodedLen for ValidatorSet {
    fn max_encoded_len() -> usize {
        428 // 20 * 21 + 8 bytes
    }
}

impl MaxEncodedLen for Header {
    fn max_encoded_len() -> usize {
        1089 // 256 + 65 + 9 * 32 + 22 * 20 + 5 * 8 = (1089)
    }
}

#[derive(Debug, Clone, Decode, PartialEq, Eq)]
pub struct Topics(pub Vec<H256>);


#[derive(Debug, Clone, Eq, PartialEq, Decode)]
pub struct Proof {
    pub bytes: Vec<Vec<u8>>,
    pub index: [u8; 1]
}

impl Header {

    pub fn hash(&self) -> H256 {
        let res = rlp::encode(self);
        // println!("Header Hash: {:?}", keccak(res.as_ref()));
        H256::from(keccak(res.as_ref()).as_fixed_bytes())
    }

    pub fn signature_valid(&self) -> Result<(), Option<&'static str>> {
        let sig = &Signature::try_from(self.signature.as_ref()).unwrap();
        let valid = sig.verify(self.hash(), self.miner);
        match valid {
            Ok(_) => Ok(()),
            Err(_) => Err(None)
        }
    }

    pub fn signer_valid(&self, validator_set: &ValidatorSet) -> bool {
        // ensure the validator_set is allowed to sign the header
        if self.number > validator_set.last_update + 200 && self.number > validator_set.last_update {
            return false
        };

        // ensure the signer is part of the validator_set
        if !&validator_set.validators.contains(&self.miner) {
            return false
        }

        return true
    }

    fn construct_extra_field(&self) -> Vec<u8> {
        let mut field: Vec<u8> = self.extra.to_vec();

        if &self.number % 200 == 0 {
            for validator in self.validators.unwrap() {
                field.append(&mut validator.as_bytes().to_vec())
            }
        }

        return field
    }

}

impl Encodable for Header {
    // this contains all fields needed to reproduce the hash used for consensus. THIS IS NOT THE HASH RETURNED FROM RPC
    fn rlp_append(&self, s: &mut RlpStream) {
        s.begin_list(16);
        s.append(&self.chain_id);
        s.append(&self.parent_hash);
        s.append(&self.sha3_uncles);
        s.append(&self.miner);
        s.append(&self.state_root);
        s.append(&self.transactions_root);
        s.append(&self.receipts_root);
        s.append(&self.logs_bloom);
        s.append(&self.difficulty);
        s.append(&self.number);
        s.append(&self.gas_limit);
        s.append(&self.gas_used);
        s.append(&self.timestamp);
        s.append(&self.construct_extra_field());
        s.append(&self.mix_hash);
        s.append(&self.nonce.to_vec());
    }
}
//
// #[derive(Debug, Clone, Eq, PartialEq, Decode)]
// pub struct Receipt {
//     pub status: bool,
//     pub cumulative_gas_used: U256,
//     pub logs_bloom: LogsBloom,
//     pub logs: Vec<Event>,
// }
//
// impl Encodable for Receipt {
//     // this contains all fields needed to reproduce the hash used for consensus. THIS IS NOT THE HASH RETURNED FROM RPC
//     fn rlp_append(&self, s: &mut RlpStream) {
//         s.begin_list(4);
//         s.append(&self.status);
//         s.append(&self.cumulative_gas_used);
//         s.append(&self.logs_bloom);
//         s.append_list::<Event, Event>(&self.logs);
//     }
// }
//
// impl Decodable for Receipt {
//     fn decode(rlp: &Rlp) -> Result<Self, DecoderError> {
//
//         Ok(Receipt {
//             status: rlp.val_at(0)?,
//             cumulative_gas_used: rlp.val_at(1)?,
//             logs_bloom: rlp.val_at(2)?,
//             logs: rlp.list_at(3)?,
//         })
//     }
// }
//
// #[derive(Debug, Clone, Eq, PartialEq, Decode)]
// pub struct Event {
//     pub address: H160,
//     pub topics: Topics,
//     pub data: Vec<u8>,
// }
//
// impl Encodable for Event {
//     // this contains all fields needed to reproduce the hash used for consensus. THIS IS NOT THE HASH RETURNED FROM RPC
//     fn rlp_append(&self, s: &mut RlpStream) {
//         s.begin_list(3);
//         s.append(&self.address);
//         s.append_list(&self.topics.0);
//         s.append(&self.data);
//     }
// }

//
// impl Decodable for Event {
//     fn decode(rlp: &Rlp) -> Result<Self, DecoderError> {
//
//         Ok(Event {
//             address: rlp.val_at(0)?,
//             topics: Topics(rlp.list_at(1)?),
//             data: rlp.val_at(2)?,
//         })
//     }
// }

#[derive(Debug, Clone, Encode, Decode, PartialEq, Eq, TypeInfo)]
pub struct LogsBloom(pub [u8; 256]);

impl Encodable for LogsBloom {
   fn rlp_append(&self, s: &mut RlpStream) {
		s.encoder().encode_value(&self.as_slice());
	}
}

impl Decodable for LogsBloom {
    fn decode(rlp: &Rlp) -> Result<Self, DecoderError> {
        // a bit messy that we're converting twice in this function, but I get lifetime errors when I want to keep this a slice
        let res = rlp.decoder().decode_value(|bytes| Ok(bytes.to_vec()));

        match res {
            Ok(val) => return Ok(LogsBloom::from(val)),
            Err(err) => panic!("{:?}", err)
        }
    }
}

impl LogsBloom {
    fn as_slice(&self) -> &[u8] {
        self.0.as_slice()
    }
}

impl From<Vec<u8>> for LogsBloom {
    fn from(item: Vec<u8>) -> Self {
        let bloom: Result<[u8; 256], _> = item.try_into();

        match bloom {
            Ok(log) => return LogsBloom(log),
            Err(error)=> panic!("{:?}", error)
        }
    }
}

#[cfg(test)]
mod tests {
    use sp_std::sync::Arc;
    use sp_std::vec::Vec;
    use crate::types::Header;
    use codec::Decode;
    use frame_support::assert_ok;
    use crate::Error;

    const VALID_EPOCH: [u8; 1090] = [56, 0, 0, 0, 0, 0, 0, 0, 243, 247, 170, 68, 94, 168, 240, 122, 56, 249, 106, 176, 222, 159, 140, 206, 75, 214, 28, 179, 0, 127, 181, 255, 43, 170, 245, 85, 13, 65, 195, 143, 29, 204, 77, 232, 222, 199, 93, 122, 171, 133, 181, 103, 182, 204, 212, 26, 211, 18, 69, 27, 148, 138, 116, 19, 240, 161, 66, 253, 64, 212, 147, 71, 104, 91, 29, 237, 128, 19, 120, 93, 102, 35, 204, 24, 210, 20, 50, 11, 107, 182, 71, 89, 141, 253, 180, 148, 144, 234, 70, 80, 154, 228, 41, 94, 91, 251, 174, 37, 162, 39, 239, 158, 208, 78, 72, 85, 74, 251, 228, 249, 115, 210, 53, 59, 217, 130, 39, 173, 222, 247, 132, 187, 135, 1, 22, 144, 40, 120, 128, 112, 185, 219, 113, 177, 64, 189, 188, 118, 245, 188, 51, 145, 105, 225, 137, 68, 95, 243, 132, 6, 46, 245, 218, 123, 33, 255, 248, 65, 145, 147, 32, 117, 221, 214, 158, 29, 90, 183, 176, 84, 27, 147, 118, 77, 20, 71, 55, 204, 38, 107, 162, 139, 12, 166, 27, 16, 178, 91, 4, 65, 236, 4, 142, 105, 78, 32, 135, 60, 148, 92, 33, 72, 127, 131, 198, 128, 130, 200, 137, 88, 104, 145, 54, 40, 128, 112, 49, 95, 238, 88, 182, 100, 129, 88, 71, 237, 15, 97, 168, 36, 206, 78, 72, 50, 24, 251, 103, 10, 24, 37, 173, 57, 1, 246, 54, 203, 249, 139, 102, 181, 223, 162, 29, 94, 132, 6, 239, 168, 117, 87, 41, 90, 170, 198, 28, 100, 178, 37, 198, 119, 220, 5, 196, 200, 86, 218, 142, 124, 107, 30, 21, 164, 191, 119, 34, 151, 20, 68, 216, 86, 152, 143, 34, 244, 226, 187, 132, 42, 24, 87, 197, 82, 135, 47, 132, 51, 72, 16, 236, 4, 49, 157, 17, 132, 77, 179, 156, 176, 217, 216, 53, 222, 98, 166, 12, 157, 148, 129, 249, 152, 193, 101, 125, 194, 136, 141, 28, 51, 131, 140, 75, 200, 243, 92, 215, 152, 150, 33, 205, 129, 38, 156, 47, 139, 107, 20, 69, 155, 200, 129, 209, 84, 26, 52, 162, 152, 142, 113, 68, 96, 65, 27, 15, 47, 52, 155, 166, 15, 36, 148, 178, 177, 72, 138, 23, 226, 165, 214, 16, 18, 69, 154, 77, 217, 38, 52, 59, 127, 84, 34, 240, 34, 0, 242, 124, 108, 198, 5, 113, 100, 67, 48, 117, 40, 152, 241, 128, 10, 197, 74, 54, 202, 198, 204, 101, 90, 132, 57, 234, 13, 33, 84, 3, 45, 2, 0, 0, 0, 0, 0, 0, 0, 208, 239, 22, 1, 0, 0, 0, 0, 66, 4, 181, 5, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 62, 76, 203, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 42, 6, 150, 98, 0, 0, 0, 0, 216, 131, 1, 1, 10, 132, 103, 101, 116, 104, 136, 103, 111, 49, 46, 49, 51, 46, 52, 133, 108, 105, 110, 117, 120, 0, 0, 0, 195, 22, 123, 223, 1, 36, 101, 23, 108, 70, 26, 251, 49, 110, 188, 119, 60, 97, 250, 238, 133, 166, 81, 93, 170, 41, 94, 38, 73, 92, 239, 111, 105, 223, 166, 153, 17, 217, 216, 228, 243, 187, 173, 184, 155, 41, 169, 124, 110, 255, 184, 164, 17, 218, 188, 106, 222, 239, 170, 132, 245, 6, 124, 139, 190, 45, 76, 64, 123, 190, 73, 67, 142, 216, 89, 254, 150, 91, 20, 13, 207, 26, 171, 113, 169, 63, 52, 155, 186, 254, 193, 85, 24, 25, 184, 190, 30, 254, 162, 252, 70, 202, 116, 154, 161, 104, 91, 29, 237, 128, 19, 120, 93, 102, 35, 204, 24, 210, 20, 50, 11, 107, 182, 71, 89, 112, 246, 87, 22, 78, 91, 117, 104, 155, 100, 183, 253, 31, 162, 117, 243, 52, 242, 142, 24, 114, 182, 28, 96, 20, 52, 45, 145, 68, 112, 236, 122, 194, 151, 91, 227, 69, 121, 108, 43, 122, 226, 245, 185, 227, 134, 205, 27, 80, 164, 85, 6, 150, 217, 87, 203, 73, 0, 240, 58, 139, 108, 143, 217, 61, 111, 76, 234, 66, 187, 179, 69, 219, 198, 240, 223, 219, 91, 236, 115, 159, 140, 205, 175, 204, 57, 243, 199, 214, 235, 246, 55, 201, 21, 22, 115, 203, 195, 107, 136, 166, 247, 155, 96, 53, 159, 20, 29, 249, 10, 12, 116, 81, 37, 177, 49, 202, 175, 253, 18, 170, 207, 106, 129, 25, 247, 225, 22, 35, 181, 164, 61, 166, 56, 233, 31, 102, 154, 19, 15, 172, 14, 21, 160, 56, 238, 223, 198, 139, 163, 195, 92, 115, 254, 213, 190, 74, 7, 175, 181, 190, 128, 125, 221, 176, 116, 99, 156, 217, 250, 97, 180, 118, 118, 192, 100, 252, 80, 214, 44, 206, 47, 215, 84, 78, 11, 44, 201, 70, 146, 212, 167, 4, 222, 190, 247, 188, 182, 19, 40, 226, 211, 167, 57, 239, 252, 211, 169, 147, 135, 208, 21, 226, 96, 238, 250, 199, 46, 190, 161, 233, 174, 50, 97, 164, 117, 162, 123, 177, 2, 143, 20, 11, 194, 167, 200, 67, 49, 138, 253, 234, 10, 110, 60, 81, 27, 189, 16, 244, 81, 158, 206, 55, 220, 36, 136, 126, 17, 181, 93, 238, 34, 99, 121, 219, 131, 207, 252, 104, 20, 149, 115, 12, 17, 253, 222, 121, 186, 76, 12, 239, 2, 116, 227, 24, 16, 201, 223, 2, 249, 143, 175, 222, 15, 132, 31, 78, 102, 161, 205, 225, 206, 184, 239, 238, 202, 255, 50, 251, 159, 138, 97, 5, 58, 86, 182, 9, 161, 87, 207, 150, 92, 169, 210, 124, 241, 138, 38, 231, 125, 95, 161, 53, 36, 241, 62, 46, 53, 4, 34, 63, 237, 21, 254, 35, 7, 184, 249, 167, 176, 60, 144, 109, 135, 11, 164, 81, 91, 237, 98, 17, 37, 81, 232, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
    const INVALID_SIG: [u8; 1090] = [56, 0, 0, 0, 0, 0, 0, 0, 243, 247, 170, 68, 94, 168, 240, 122, 56, 249, 106, 176, 222, 159, 140, 206, 75, 214, 28, 179, 0, 127, 181, 255, 43, 170, 245, 85, 13, 65, 195, 143, 29, 204, 77, 232, 222, 199, 93, 122, 171, 133, 181, 103, 182, 204, 212, 26, 211, 18, 69, 27, 148, 138, 116, 19, 240, 161, 66, 253, 64, 212, 147, 71, 104, 91, 29, 237, 128, 19, 120, 93, 102, 35, 204, 24, 210, 20, 50, 11, 107, 182, 71, 89, 141, 253, 180, 148, 144, 234, 70, 80, 154, 228, 41, 94, 91, 251, 174, 37, 162, 39, 239, 158, 208, 78, 72, 85, 74, 251, 228, 249, 115, 210, 53, 59, 217, 130, 39, 173, 222, 247, 132, 187, 135, 1, 22, 144, 40, 120, 128, 112, 185, 219, 113, 177, 64, 189, 188, 118, 245, 188, 51, 145, 105, 225, 137, 68, 95, 243, 132, 6, 46, 245, 218, 123, 33, 255, 248, 65, 145, 147, 32, 117, 221, 214, 158, 29, 90, 183, 176, 84, 27, 147, 118, 77, 20, 71, 55, 204, 38, 107, 162, 139, 12, 166, 27, 16, 178, 91, 4, 65, 236, 4, 142, 105, 78, 32, 135, 60, 148, 92, 33, 72, 127, 131, 198, 128, 130, 200, 137, 88, 104, 145, 54, 40, 128, 112, 49, 95, 238, 88, 182, 100, 129, 88, 71, 237, 15, 97, 168, 36, 206, 78, 72, 50, 24, 251, 103, 10, 24, 37, 173, 57, 1, 246, 54, 203, 249, 139, 102, 181, 223, 162, 29, 94, 132, 6, 239, 168, 117, 87, 41, 90, 170, 198, 28, 100, 178, 37, 198, 119, 220, 5, 196, 200, 86, 218, 142, 124, 107, 30, 21, 164, 191, 119, 34, 151, 20, 68, 216, 86, 152, 143, 34, 244, 226, 187, 132, 42, 24, 87, 197, 82, 135, 47, 132, 51, 72, 16, 236, 4, 49, 157, 17, 132, 77, 179, 156, 176, 217, 216, 53, 222, 98, 166, 12, 157, 148, 129, 249, 152, 193, 101, 125, 194, 136, 141, 28, 51, 131, 140, 75, 200, 243, 92, 215, 152, 150, 33, 205, 129, 38, 156, 47, 139, 107, 20, 69, 155, 200, 129, 209, 84, 26, 52, 162, 152, 142, 113, 68, 96, 65, 27, 15, 47, 52, 155, 166, 15, 36, 148, 178, 177, 72, 138, 23, 226, 165, 214, 16, 18, 69, 154, 77, 217, 38, 52, 59, 127, 84, 34, 240, 34, 0, 242, 124, 108, 198, 5, 113, 100, 67, 48, 117, 40, 152, 241, 128, 10, 197, 74, 54, 202, 198, 204, 101, 90, 132, 57, 234, 13, 33, 84, 3, 45, 2, 0, 0, 0, 0, 0, 0, 0, 208, 239, 22, 1, 0, 0, 0, 0, 66, 4, 181, 5, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 62, 76, 203, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 42, 6, 150, 98, 0, 0, 0, 0, 216, 131, 1, 1, 10, 132, 103, 101, 116, 104, 136, 103, 111, 49, 46, 49, 51, 46, 52, 133, 108, 105, 110, 117, 120, 0, 0, 0, 195, 22, 123, 223, 1, 36, 101, 23, 108, 70, 26, 251, 49, 110, 188, 119, 60, 97, 250, 238, 133, 166, 81, 93, 170, 41, 94, 38, 73, 92, 239, 111, 105, 223, 166, 153, 17, 217, 216, 228, 243, 187, 173, 184, 155, 41, 169, 124, 110, 255, 184, 164, 17, 218, 188, 106, 222, 239, 170, 132, 245, 6, 124, 139, 190, 45, 76, 64, 123, 190, 73, 67, 142, 216, 89, 254, 150, 91, 20, 13, 207, 26, 171, 113, 169, 63, 52, 155, 186, 254, 193, 85, 24, 25, 184, 190, 30, 254, 162, 252, 70, 202, 116, 154, 161, 104, 91, 29, 237, 128, 19, 120, 93, 102, 35, 204, 24, 210, 20, 50, 11, 107, 182, 71, 89, 112, 246, 87, 22, 78, 91, 117, 104, 155, 100, 183, 253, 31, 162, 117, 243, 52, 242, 142, 24, 114, 182, 28, 96, 20, 52, 45, 145, 68, 112, 236, 122, 194, 151, 91, 227, 69, 121, 108, 43, 122, 226, 245, 185, 227, 134, 205, 27, 80, 164, 85, 6, 150, 217, 87, 203, 73, 0, 240, 58, 139, 108, 143, 217, 61, 111, 76, 234, 66, 187, 179, 69, 219, 198, 240, 223, 219, 91, 236, 115, 159, 140, 205, 175, 204, 57, 243, 199, 214, 235, 246, 55, 201, 21, 22, 115, 203, 195, 107, 136, 166, 247, 155, 96, 53, 159, 20, 29, 249, 10, 12, 116, 81, 37, 177, 49, 202, 175, 253, 18, 170, 207, 106, 129, 25, 247, 225, 22, 35, 181, 164, 61, 166, 56, 233, 31, 102, 154, 19, 15, 172, 14, 21, 160, 56, 238, 223, 198, 139, 163, 195, 92, 115, 254, 213, 190, 74, 7, 175, 181, 190, 128, 125, 221, 176, 116, 99, 156, 217, 250, 97, 180, 118, 118, 192, 100, 252, 80, 214, 44, 206, 47, 215, 84, 78, 11, 44, 201, 70, 146, 212, 167, 4, 222, 190, 247, 188, 182, 19, 40, 226, 211, 167, 57, 239, 252, 211, 169, 147, 135, 208, 21, 226, 96, 238, 250, 199, 46, 190, 161, 233, 174, 50, 97, 164, 117, 162, 123, 177, 2, 143, 20, 11, 194, 167, 200, 67, 49, 138, 253, 234, 10, 110, 60, 81, 27, 189, 16, 244, 81, 158, 206, 55, 220, 36, 136, 126, 17, 181, 93, 238, 34, 99, 121, 219, 131, 207, 252, 104, 20, 149, 115, 12, 17, 253, 222, 121, 186, 76, 12, 239, 2, 116, 227, 24, 16, 201, 223, 2, 249, 143, 175, 222, 15, 132, 31, 78, 102, 161, 205, 225, 206, 184, 239, 238, 202, 255, 50, 251, 159, 138, 97, 5, 58, 86, 182, 9, 161, 87, 207, 150, 92, 169, 210, 124, 241, 138, 38, 231, 125, 95, 161, 53, 36, 241, 62, 46, 53, 4, 34, 63, 237, 21, 254, 35, 7, 184, 249, 167, 176, 60, 144, 109, 135, 11, 164, 81, 91, 237, 98, 17, 37, 81, 231, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
    const CORRUPT_EPOCH: [u8; 1089] = [0, 0, 0, 0, 0, 0, 0, 243, 247, 170, 68, 94, 168, 240, 122, 56, 249, 106, 176, 222, 159, 140, 206, 75, 214, 28, 179, 0, 127, 181, 255, 43, 170, 245, 85, 13, 65, 195, 143, 29, 204, 77, 232, 222, 199, 93, 122, 171, 133, 181, 103, 182, 204, 212, 26, 211, 18, 69, 27, 148, 138, 116, 19, 240, 161, 66, 253, 64, 212, 147, 71, 104, 91, 29, 237, 128, 19, 120, 93, 102, 35, 204, 24, 210, 20, 50, 11, 107, 182, 71, 89, 141, 253, 180, 148, 144, 234, 70, 80, 154, 228, 41, 94, 91, 251, 174, 37, 162, 39, 239, 158, 208, 78, 72, 85, 74, 251, 228, 249, 115, 210, 53, 59, 217, 130, 39, 173, 222, 247, 132, 187, 135, 1, 22, 144, 40, 120, 128, 112, 185, 219, 113, 177, 64, 189, 188, 118, 245, 188, 51, 145, 105, 225, 137, 68, 95, 243, 132, 6, 46, 245, 218, 123, 33, 255, 248, 65, 145, 147, 32, 117, 221, 214, 158, 29, 90, 183, 176, 84, 27, 147, 118, 77, 20, 71, 55, 204, 38, 107, 162, 139, 12, 166, 27, 16, 178, 91, 4, 65, 236, 4, 142, 105, 78, 32, 135, 60, 148, 92, 33, 72, 127, 131, 198, 128, 130, 200, 137, 88, 104, 145, 54, 40, 128, 112, 49, 95, 238, 88, 182, 100, 129, 88, 71, 237, 15, 97, 168, 36, 206, 78, 72, 50, 24, 251, 103, 10, 24, 37, 173, 57, 1, 246, 54, 203, 249, 139, 102, 181, 223, 162, 29, 94, 132, 6, 239, 168, 117, 87, 41, 90, 170, 198, 28, 100, 178, 37, 198, 119, 220, 5, 196, 200, 86, 218, 142, 124, 107, 30, 21, 164, 191, 119, 34, 151, 20, 68, 216, 86, 152, 143, 34, 244, 226, 187, 132, 42, 24, 87, 197, 82, 135, 47, 132, 51, 72, 16, 236, 4, 49, 157, 17, 132, 77, 179, 156, 176, 217, 216, 53, 222, 98, 166, 12, 157, 148, 129, 249, 152, 193, 101, 125, 194, 136, 141, 28, 51, 131, 140, 75, 200, 243, 92, 215, 152, 150, 33, 205, 129, 38, 156, 47, 139, 107, 20, 69, 155, 200, 129, 209, 84, 26, 52, 162, 152, 142, 113, 68, 96, 65, 27, 15, 47, 52, 155, 166, 15, 36, 148, 178, 177, 72, 138, 23, 226, 165, 214, 16, 18, 69, 154, 77, 217, 38, 52, 59, 127, 84, 34, 240, 34, 0, 242, 124, 108, 198, 5, 113, 100, 67, 48, 117, 40, 152, 241, 128, 10, 197, 74, 54, 202, 198, 204, 101, 90, 132, 57, 234, 13, 33, 84, 3, 45, 2, 0, 0, 0, 0, 0, 0, 0, 208, 239, 22, 1, 0, 0, 0, 0, 66, 4, 181, 5, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 62, 76, 203, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 42, 6, 150, 98, 0, 0, 0, 0, 216, 131, 1, 1, 10, 132, 103, 101, 116, 104, 136, 103, 111, 49, 46, 49, 51, 46, 52, 133, 108, 105, 110, 117, 120, 0, 0, 0, 195, 22, 123, 223, 1, 36, 101, 23, 108, 70, 26, 251, 49, 110, 188, 119, 60, 97, 250, 238, 133, 166, 81, 93, 170, 41, 94, 38, 73, 92, 239, 111, 105, 223, 166, 153, 17, 217, 216, 228, 243, 187, 173, 184, 155, 41, 169, 124, 110, 255, 184, 164, 17, 218, 188, 106, 222, 239, 170, 132, 245, 6, 124, 139, 190, 45, 76, 64, 123, 190, 73, 67, 142, 216, 89, 254, 150, 91, 20, 13, 207, 26, 171, 113, 169, 63, 52, 155, 186, 254, 193, 85, 24, 25, 184, 190, 30, 254, 162, 252, 70, 202, 116, 154, 161, 104, 91, 29, 237, 128, 19, 120, 93, 102, 35, 204, 24, 210, 20, 50, 11, 107, 182, 71, 89, 112, 246, 87, 22, 78, 91, 117, 104, 155, 100, 183, 253, 31, 162, 117, 243, 52, 242, 142, 24, 114, 182, 28, 96, 20, 52, 45, 145, 68, 112, 236, 122, 194, 151, 91, 227, 69, 121, 108, 43, 122, 226, 245, 185, 227, 134, 205, 27, 80, 164, 85, 6, 150, 217, 87, 203, 73, 0, 240, 58, 139, 108, 143, 217, 61, 111, 76, 234, 66, 187, 179, 69, 219, 198, 240, 223, 219, 91, 236, 115, 159, 140, 205, 175, 204, 57, 243, 199, 214, 235, 246, 55, 201, 21, 22, 115, 203, 195, 107, 136, 166, 247, 155, 96, 53, 159, 20, 29, 249, 10, 12, 116, 81, 37, 177, 49, 202, 175, 253, 18, 170, 207, 106, 129, 25, 247, 225, 22, 35, 181, 164, 61, 166, 56, 233, 31, 102, 154, 19, 15, 172, 14, 21, 160, 56, 238, 223, 198, 139, 163, 195, 92, 115, 254, 213, 190, 74, 7, 175, 181, 190, 128, 125, 221, 176, 116, 99, 156, 217, 250, 97, 180, 118, 118, 192, 100, 252, 80, 214, 44, 206, 47, 215, 84, 78, 11, 44, 201, 70, 146, 212, 167, 4, 222, 190, 247, 188, 182, 19, 40, 226, 211, 167, 57, 239, 252, 211, 169, 147, 135, 208, 21, 226, 96, 238, 250, 199, 46, 190, 161, 233, 174, 50, 97, 164, 117, 162, 123, 177, 2, 143, 20, 11, 194, 167, 200, 67, 49, 138, 253, 234, 10, 110, 60, 81, 27, 189, 16, 244, 81, 158, 206, 55, 220, 36, 136, 126, 17, 181, 93, 238, 34, 99, 121, 219, 131, 207, 252, 104, 20, 149, 115, 12, 17, 253, 222, 121, 186, 76, 12, 239, 2, 116, 227, 24, 16, 201, 223, 2, 249, 143, 175, 222, 15, 132, 31, 78, 102, 161, 205, 225, 206, 184, 239, 238, 202, 255, 50, 251, 159, 138, 97, 5, 58, 86, 182, 9, 161, 87, 207, 150, 92, 169, 210, 124, 241, 138, 38, 231, 125, 95, 161, 53, 36, 241, 62, 46, 53, 4, 34, 63, 237, 21, 254, 35, 7, 184, 249, 167, 176, 60, 144, 109, 135, 11, 164, 81, 91, 237, 98, 17, 37, 81, 232, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
    // const VALID_EPOCH: [u8; 1090] = [56, 0, 0, 0, 0, 0, 0, 0, 243, 247, 170, 68, 94, 168, 240, 122, 56, 249, 106, 176, 222, 159, 140, 206, 75, 214, 28, 179, 0, 127, 181, 255, 43, 170, 245, 85, 13, 65, 195, 143, 29, 204, 77, 232, 222, 199, 93, 122, 171, 133, 181, 103, 182, 204, 212, 26, 211, 18, 69, 27, 148, 138, 116, 19, 240, 161, 66, 253, 64, 212, 147, 71, 104, 91, 29, 237, 128, 19, 120, 93, 102, 35, 204, 24, 210, 20, 50, 11, 107, 182, 71, 89, 141, 253, 180, 148, 144, 234, 70, 80, 154, 228, 41, 94, 91, 251, 174, 37, 162, 39, 239, 158, 208, 78, 72, 85, 74, 251, 228, 249, 115, 210, 53, 59, 217, 130, 39, 173, 222, 247, 132, 187, 135, 1, 22, 144, 40, 120, 128, 112, 185, 219, 113, 177, 64, 189, 188, 118, 245, 188, 51, 145, 105, 225, 137, 68, 95, 243, 132, 6, 46, 245, 218, 123, 33, 255, 248, 65, 145, 147, 32, 117, 221, 214, 158, 29, 90, 183, 176, 84, 27, 147, 118, 77, 20, 71, 55, 204, 38, 107, 162, 139, 12, 166, 27, 16, 178, 91, 4, 65, 236, 4, 142, 105, 78, 32, 135, 60, 148, 92, 33, 72, 127, 131, 198, 128, 130, 200, 137, 88, 104, 145, 54, 40, 128, 112, 49, 95, 238, 88, 182, 100, 129, 88, 71, 237, 15, 97, 168, 36, 206, 78, 72, 50, 24, 251, 103, 10, 24, 37, 173, 57, 1, 246, 54, 203, 249, 139, 102, 181, 223, 162, 29, 94, 132, 6, 239, 168, 117, 87, 41, 90, 170, 198, 28, 100, 178, 37, 198, 119, 220, 5, 196, 200, 86, 218, 142, 124, 107, 30, 21, 164, 191, 119, 34, 151, 20, 68, 216, 86, 152, 143, 34, 244, 226, 187, 132, 42, 24, 87, 197, 82, 135, 47, 132, 51, 72, 16, 236, 4, 49, 157, 17, 132, 77, 179, 156, 176, 217, 216, 53, 222, 98, 166, 12, 157, 148, 129, 249, 152, 193, 101, 125, 194, 136, 141, 28, 51, 131, 140, 75, 200, 243, 92, 215, 152, 150, 33, 205, 129, 38, 156, 47, 139, 107, 20, 69, 155, 200, 129, 209, 84, 26, 52, 162, 152, 142, 113, 68, 96, 65, 27, 15, 47, 52, 155, 166, 15, 36, 148, 178, 177, 72, 138, 23, 226, 165, 214, 16, 18, 69, 154, 77, 217, 38, 52, 59, 127, 84, 34, 240, 34, 0, 242, 124, 108, 198, 5, 113, 100, 67, 48, 117, 40, 152, 241, 128, 10, 197, 74, 54, 202, 198, 204, 101, 90, 132, 57, 234, 13, 33, 84, 3, 45, 2, 0, 0, 0, 0, 0, 0, 0, 208, 239, 22, 1, 0, 0, 0, 0, 66, 4, 181, 5, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 62, 76, 203, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 42, 6, 150, 98, 0, 0, 0, 0, 216, 131, 1, 1, 10, 132, 103, 101, 116, 104, 136, 103, 111, 49, 46, 49, 51, 46, 52, 133, 108, 105, 110, 117, 120, 0, 0, 0, 195, 22, 123, 223, 1, 36, 101, 23, 108, 70, 26, 251, 49, 110, 188, 119, 60, 97, 250, 238, 133, 166, 81, 93, 170, 41, 94, 38, 73, 92, 239, 111, 105, 223, 166, 153, 17, 217, 216, 228, 243, 187, 173, 184, 155, 41, 169, 124, 110, 255, 184, 164, 17, 218, 188, 106, 222, 239, 170, 132, 245, 6, 124, 139, 190, 45, 76, 64, 123, 190, 73, 67, 142, 216, 89, 254, 150, 91, 20, 13, 207, 26, 171, 113, 169, 63, 52, 155, 186, 254, 193, 85, 24, 25, 184, 190, 30, 254, 162, 252, 70, 202, 116, 154, 161, 104, 91, 29, 237, 128, 19, 120, 93, 102, 35, 204, 24, 210, 20, 50, 11, 107, 182, 71, 89, 112, 246, 87, 22, 78, 91, 117, 104, 155, 100, 183, 253, 31, 162, 117, 243, 52, 242, 142, 24, 114, 182, 28, 96, 20, 52, 45, 145, 68, 112, 236, 122, 194, 151, 91, 227, 69, 121, 108, 43, 122, 226, 245, 185, 227, 134, 205, 27, 80, 164, 85, 6, 150, 217, 87, 203, 73, 0, 240, 58, 139, 108, 143, 217, 61, 111, 76, 234, 66, 187, 179, 69, 219, 198, 240, 223, 219, 91, 236, 115, 159, 140, 205, 175, 204, 57, 243, 199, 214, 235, 246, 55, 201, 21, 22, 115, 203, 195, 107, 136, 166, 247, 155, 96, 53, 159, 20, 29, 249, 10, 12, 116, 81, 37, 177, 49, 202, 175, 253, 18, 170, 207, 106, 129, 25, 247, 225, 22, 35, 181, 164, 61, 166, 56, 233, 31, 102, 154, 19, 15, 172, 14, 21, 160, 56, 238, 223, 198, 139, 163, 195, 92, 115, 254, 213, 190, 74, 7, 175, 181, 190, 128, 125, 221, 176, 116, 99, 156, 217, 250, 97, 180, 118, 118, 192, 100, 252, 80, 214, 44, 206, 47, 215, 84, 78, 11, 44, 201, 70, 146, 212, 167, 4, 222, 190, 247, 188, 182, 19, 40, 226, 211, 167, 57, 239, 252, 211, 169, 147, 135, 208, 21, 226, 96, 238, 250, 199, 46, 190, 161, 233, 174, 50, 97, 164, 117, 162, 123, 177, 2, 143, 20, 11, 194, 167, 200, 67, 49, 138, 253, 234, 10, 110, 60, 81, 27, 189, 16, 244, 81, 158, 206, 55, 220, 36, 136, 126, 17, 181, 93, 238, 34, 99, 121, 219, 131, 207, 252, 104, 20, 149, 115, 12, 17, 253, 222, 121, 186, 76, 12, 239, 2, 116, 227, 24, 16, 201, 223, 2, 249, 143, 175, 222, 15, 132, 31, 78, 102, 161, 205, 225, 206, 184, 239, 238, 202, 255, 50, 251, 159, 138, 97, 5, 58, 86, 182, 9, 161, 87, 207, 150, 92, 169, 210, 124, 241, 138, 38, 231, 125, 95, 161, 53, 36, 241, 62, 46, 53, 4, 34, 63, 237, 21, 254, 35, 7, 184, 249, 167, 176, 60, 144, 109, 135, 11, 164, 81, 91, 237, 98, 17, 37, 81, 232, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
    #[test]
    fn decodes_valid_header() {
        let header: Result<Header, _> = Decode::decode(&mut &*VALID_EPOCH.to_vec());

        assert_ok!(header);
    }

    #[test]
    fn fails_on_invalid_encoding() {
        let header: Result<Header, _> = Decode::decode(&mut &*CORRUPT_EPOCH.to_vec());

        assert!(header.is_err());
    }

    #[test]
    fn verify_valid_signature() {
         let header: Header = Decode::decode(&mut &*VALID_EPOCH.to_vec()).unwrap();

        assert_ok!(header.signature_valid());
    }

    #[test]
    fn fail_on_invalid_signature() {
         let header: Header = Decode::decode(&mut &*INVALID_SIG.to_vec()).unwrap();

        assert_eq!(header.signature_valid(), Err(None));
    }

    #[test]
    fn hash_header_correctly() {
        let header: Header = Decode::decode(&mut &*VALID_EPOCH.to_vec()).unwrap();
        let expected: [u8; 32] = [247,52,9,170,125,59,101,76,55,212,188,151,200,195,161,96,11,149,197,225,205,17,2,167,158,55,78,93,142,77,144,38];
        assert_eq!(header.hash(), expected.into())
    }
}

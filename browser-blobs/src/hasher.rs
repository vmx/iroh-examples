use bao_tree::{Hash, Hasher};
use sha2::{Digest, Sha256};

//type HasherToUse = bao_tree::Blake3Hasher;
pub type HasherToUse = CommpHasher;

fn calculate_merkle_root(data: &[u8]) -> Vec<u8> {
    // Split the data into 32-byte chunks
    let chunk_size = 32;
    let mut hashes: Vec<Vec<u8>> = data
        .chunks(chunk_size)
        .map(|chunk| chunk.to_vec())
        .collect();

    // Build the Merkle tree
    while hashes.len() > 1 {
        let mut new_hashes: Vec<Vec<u8>> = Vec::new();
        for i in (0..hashes.len()).step_by(2) {
            let mut hasher = Sha256::new();
            hasher.update(&hashes[i]);
            hasher.update(&hashes[i + 1]);
            let mut hashed = hasher.finalize();
            // CommP uses a 254-bit SHA-256 hash, hence zero the last two bits.
            hashed[31] &= 0b0011_1111;
            new_hashes.push(hashed.to_vec());
        }
        hashes = new_hashes;
    }

    // The root hash is the last remaining hash
    hashes[0].clone()
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct CommpHasher;

impl Hasher for CommpHasher {
    fn hash_chunk(_start_chunk: u64, data: &[u8], _is_root: bool) -> Hash {
        //println!("vmx: hash_chunk: data len: {}", data.len());
        // TODO vmx 2025-09-21: no clue when 20 and 64 bytes are hashed, so this is a hack for now
        // to at least keep things running.
        if data.len() < 64 {
            return [0u8; 32].into();
        }
        let root = calculate_merkle_root(data);
        //println!("vmx: hash_chunk: root: {:X?}", root);
        <[u8; 32]>::try_from(&root[..32]).unwrap().into()
    }
    fn hash_inner(
        left_child: &Hash,
        right_child: &Hash,
        _is_root: bool,
    ) -> Hash {
        let data: Vec<_> = [&left_child.as_bytes()[..], &right_child.as_bytes()[..]].concat();
        let mut hashed = Sha256::digest(&data);
        //println!("vmx: inner data: {:X?}", data);
        //println!("vmx: inner hashed: {:X?}", hashed);
        // CommP uses a 254-bit SHA-256 hash, hence zero the last two bits.
        hashed[31] &= 0b0011_1111;
        //hashed[31] &= 0b1111_1100;
        <[u8; 32]>::from(hashed).into()
    }
}

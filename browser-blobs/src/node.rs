use std::path::Path;

use anyhow::{anyhow, Result};
use bao_tree::{Blake3Hasher, Hasher};
use bytes::Bytes;
use iroh::{discovery::static_provider::StaticProvider, protocol::Router, Endpoint, EndpointId};
use iroh_blobs::{
    api::{blobs::{BlobStatus, ImportMode, AddPathOptions}, downloader::Downloader, Store},
    ticket::BlobTicket,
    BlobFormat, BlobsProtocol, Hash,
};
use sha2::{Digest, Sha256};

//type HasherToUse = Blake3Hasher;
pub type HasherToUse = CommpHasher;


// Based on Mistral:
// write some rust code that takes 1024 bytes as input and calculate the root of its merkle tree. the hashing function should be sha2-256.
fn calculate_merkle_root(data: &[u8]) -> Vec<u8> {
    // Split the data into 32-byte chunks
    let chunk_size = 32;
    let mut hashes: Vec<Vec<u8>> = data.chunks(chunk_size).map(|chunk| chunk.to_vec()).collect();

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
    fn hash_chunk(_start_chunk: u64, data: &[u8], _is_root: bool) -> bao_tree::Hash {
        //println!("vmx: hash_chunk: data len: {}", data.len());
        // TODO vmx 2025-09-21: no clue when 20 and 64 bytes are hashed, so this is a hack for now
        // to at least keep things running.
        if data.len() < 64 {
            return [0u8; 32].into()
        }
        let root = calculate_merkle_root(data);
        //println!("vmx: hash_chunk: root: {:X?}", root);
        <[u8; 32]>::try_from(&root[..32]).unwrap().into()
    }
    fn hash_inner(left_child: &bao_tree::Hash, right_child: &bao_tree::Hash, _is_root: bool) -> bao_tree::Hash {
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


#[derive(Debug, Clone)]
pub struct BlobsNode {
    discovery: StaticProvider,
    router: Router,
    pub blobs: Store,
    downloader: Downloader,
}

impl BlobsNode {
    pub async fn spawn() -> Result<Self> {
        let discovery = StaticProvider::default();
        let endpoint = iroh::Endpoint::bind().await?;
        endpoint.discovery().add(discovery.clone());

        #[cfg(not(feature = "cli"))]
        let store = iroh_blobs::store::mem::MemStore::<HasherToUse>::default();
        #[cfg(feature = "cli")]
        let store = iroh_blobs::store::fs::FsStore::load::<HasherToUse>("datastore").await?;
        let downloader = Downloader::new::<HasherToUse>(&store, &endpoint);
        let router = Router::builder(endpoint)
            .accept(iroh_blobs::ALPN, BlobsProtocol::<HasherToUse>::new(&store, None))
            .spawn();
        Ok(Self {
            blobs: store.as_ref().clone(),
            router,
            downloader,
            discovery,
        })
    }

    pub fn endpoint_id(&self) -> EndpointId {
        self.router.endpoint().id()
    }

    pub fn endpoint(&self) -> &Endpoint {
        self.router.endpoint()
    }

    pub async fn download(&self, ticket: BlobTicket) -> anyhow::Result<Hash> {
        self.discovery.add_endpoint_info(ticket.addr().clone());
        self.downloader
            .download(ticket.hash_and_format(), [ticket.addr().id])
            .await?;
        Ok(ticket.hash())
    }

    pub async fn import(&self, path: &Path) -> Result<BlobTicket> {
        let tag = self
            .blobs
            .add_path(&path)
            //.add_path_with_opts(AddPathOptions {
            //    path: path.to_path_buf(),
            //    format: BlobFormat::Raw,
            //    mode: ImportMode::TryReference
            //})
            .await
            .inspect_err(|err| tracing::warn!(?err, "import failed"))?;
        tracing::info!(?tag, "imported!");
        let ticket = self.ticket(tag.hash, tag.format).await?;
        Ok(ticket)
    }

    pub async fn complete_size(&self, hash: Hash) -> Result<u64> {
        match self.blobs.status(hash).await? {
            BlobStatus::NotFound => Err(anyhow!("not found")),
            BlobStatus::Partial { size: _ } => Err(anyhow!("blob is incomplete")),
            BlobStatus::Complete { size } => Ok(size),
        }
    }

    pub async fn ticket(&self, hash: Hash, format: BlobFormat) -> Result<BlobTicket> {
        self.endpoint().online().await;
        let addr = self.endpoint().addr();
        Ok(BlobTicket::new(addr, hash, format))
    }
}

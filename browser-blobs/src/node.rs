use std::path::Path;

use anyhow::Result;
use iroh::{discovery::static_provider::StaticProvider, protocol::Router, Endpoint};
use iroh_blobs::{
    api::{
        blobs::{AddPathOptions, ImportMode},
        Store,
    },
    ticket::BlobTicket,
    BlobFormat, BlobsProtocol, Hash,
};

use crate::hasher::HasherToUse;

#[derive(Debug, Clone)]
pub struct BlobsNode {
    router: Router,
    pub blobs: Store,
}

impl BlobsNode {
    pub async fn spawn() -> Result<Self> {
        let discovery = StaticProvider::default();
        let endpoint = iroh::Endpoint::bind().await?;
        endpoint.discovery().add(discovery);

        let store = iroh_blobs::store::fs::FsStore::load::<HasherToUse>("datastore").await?;
        let router = Router::builder(endpoint)
            .accept(
                iroh_blobs::ALPN,
                BlobsProtocol::<HasherToUse>::new(&store, None),
            )
            .spawn();
        Ok(Self {
            blobs: store.as_ref().clone(),
            router,
        })
    }

    pub fn endpoint(&self) -> &Endpoint {
        self.router.endpoint()
    }

    pub async fn import(&self, path: &Path) -> Result<BlobTicket> {
        let tag = self
            .blobs
            .add_path_with_opts(AddPathOptions {
                path: path.to_path_buf(),
                format: BlobFormat::Raw,
                mode: ImportMode::TryReference,
            })
            .await
            .inspect_err(|err| tracing::warn!(?err, "import failed"))?;
        tracing::info!(?tag, "imported!");
        let ticket = self.ticket(tag.hash, tag.format).await?;
        Ok(ticket)
    }

    pub async fn ticket(&self, hash: Hash, format: BlobFormat) -> Result<BlobTicket> {
        self.endpoint().online().await;
        let addr = self.endpoint().addr();
        Ok(BlobTicket::new(addr, hash, format))
    }
}

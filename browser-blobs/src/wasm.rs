use anyhow::{anyhow, Result};
use bao_tree::io::BaoContentItem;
use futures::channel::mpsc;
use iroh::{discovery::static_provider::StaticProvider, protocol::Router};
use iroh_blobs::{get::request::GetBlobItem, ticket::BlobTicket, BlobsProtocol};
use js_sys::Uint8Array;
use n0_future::{SinkExt, StreamExt};
use tracing::level_filters::LevelFilter;
use tracing_subscriber_wasm::MakeConsoleWriter;
use wasm_bindgen::{prelude::wasm_bindgen, JsError, JsValue};
use wasm_streams::{readable::sys::ReadableStream as JsReadableStream, ReadableStream};

#[wasm_bindgen(start)]
fn start() {
    console_error_panic_hook::set_once();

    tracing_subscriber::fmt()
        .with_max_level(LevelFilter::TRACE)
        .with_writer(
            // To avoide trace events in the browser from showing their JS backtrace
            MakeConsoleWriter::default().map_trace_level_to(tracing::Level::DEBUG),
        )
        // If we don't do this in the browser, we get a runtime error.
        .without_time()
        .with_ansi(false)
        .init();

    tracing::info!("(testing logging) Logging setup");
}

#[wasm_bindgen]
pub struct BlobsNode {
    router: Router,
}

#[wasm_bindgen]
impl BlobsNode {
    pub async fn spawn() -> Result<Self, JsError> {
        let discovery = StaticProvider::default();
        let endpoint = iroh::Endpoint::bind().await?;
        endpoint.discovery().add(discovery.clone());

        let store = iroh_blobs::store::mem::MemStore::<crate::HasherToUse>::default();
        let router = Router::builder(endpoint)
            .accept(
                iroh_blobs::ALPN,
                BlobsProtocol::<crate::HasherToUse>::new(&store, None),
            )
            .spawn();
        Ok(Self { router })
    }

    pub fn endpoint_id(&self) -> String {
        self.router.endpoint().id().to_string()
    }

    pub async fn download(&self, ticket: String) -> Result<JsReadableStream, JsError> {
        let ticket: BlobTicket = ticket.parse().map_err(to_js_err)?;
        let connection = self
            .router
            .endpoint()
            .connect(ticket.addr().id, iroh_blobs::ALPN)
            .await?;
        let mut progress =
            iroh_blobs::get::request::get_blob::<crate::HasherToUse>(connection, ticket.hash());

        let (mut tx, rx) = mpsc::channel::<Result<JsValue, JsValue>>(1);

        wasm_bindgen_futures::spawn_local(async move {
            loop {
                match progress.next().await {
                    Some(GetBlobItem::Item(item)) => match item {
                        BaoContentItem::Leaf(leaf) => {
                            //tracing::info!("vmx: data received: {:?}", &leaf.data);
                            let js_value = Uint8Array::from(&leaf.data[..]).into();
                            tx.send(Ok(js_value)).await.unwrap();

                            //tokio::io::stdout().write_all(&leaf.data).await?;
                        }
                        BaoContentItem::Parent(parent) => {
                            tracing::info!("Parent: {parent:?}");
                        }
                    },
                    Some(GetBlobItem::Done(stats)) => {
                        //break stats;
                        println!("vmx: stats: {:?}", stats);
                        break;
                    }
                    Some(GetBlobItem::Error(err)) => {
                        return Err(anyhow!("Error while streaming blob: {err}"))
                            .map_err(to_js_err)
                            .expect("vmx: error while streaming blob");
                    }
                    None => {
                        return Err(anyhow!("Stream ended unexpectedly."))
                            .map_err(to_js_err)
                            .expect("vmx: error stream ended");
                    }
                }
            }
        });

        let output_stream = ReadableStream::from_stream(rx).into_raw();
        Ok(output_stream)
    }
}

fn to_js_err(err: impl Into<anyhow::Error>) -> JsError {
    let err: anyhow::Error = err.into();
    JsError::new(&err.to_string())
}

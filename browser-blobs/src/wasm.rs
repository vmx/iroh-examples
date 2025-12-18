use anyhow::{anyhow, Result};
use bao_tree::io::BaoContentItem;
use bytes::Bytes;
use futures::channel::mpsc;
use iroh_blobs::{get::request::GetBlobItem, ticket::BlobTicket, Hash};
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
pub struct BlobsNode(crate::BlobsNode);

#[wasm_bindgen]
impl BlobsNode {
    pub async fn spawn() -> Result<Self, JsError> {
        Ok(Self(crate::BlobsNode::spawn().await.map_err(to_js_err)?))
    }

    pub fn endpoint_id(&self) -> String {
        self.0.endpoint().id().to_string()
    }

    //pub async fn download(&self, ticket: String) -> Result<String, JsError> {
    pub async fn download(&self, ticket: String) -> Result<JsReadableStream, JsError> {
        let ticket: BlobTicket = ticket.parse().map_err(to_js_err)?;
        //let hash = self.0.download(ticket).await.map_err(to_js_err)?;
        //Ok(hash.to_string())
        let connection = self
            .0
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
                            println!("vmx: data received: {:?}", &leaf.data);
                            tracing::info!("vmx: data received: {:?}", &leaf.data);
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

    pub async fn complete_size(&self, hash: String) -> Result<u64, JsError> {
        let hash: Hash = hash.parse().map_err(to_js_err)?;
        let size = self.0.complete_size(hash).await.map_err(to_js_err)?;
        Ok(size)
    }

    pub async fn get(&self, hash: String) -> Result<Uint8Array, JsError> {
        let hash: Hash = hash.parse().map_err(to_js_err)?;
        let bytes = self.0.blobs.get_bytes(hash).await?;
        Ok(bytes_to_uint8array(&bytes))
    }
}

fn to_js_err(err: impl Into<anyhow::Error>) -> JsError {
    let err: anyhow::Error = err.into();
    JsError::new(&err.to_string())
}

pub fn uint8array_to_bytes(data: &Uint8Array) -> Bytes {
    let mut buffer = vec![0u8; data.length() as usize];
    data.copy_to(&mut buffer[..]);
    Bytes::from(buffer)
}

pub fn bytes_to_uint8array(bytes: &[u8]) -> Uint8Array {
    // Create a Uint8Array with the same length
    let array = Uint8Array::new_with_length(bytes.len() as u32);
    // Copy the bytes into the JS Uint8Array
    array.copy_from(bytes);
    array
}

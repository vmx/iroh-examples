import init, { BlobsNode } from "./wasm/blobs_wasm.js";

await init();

log("launching iroh endpoint …");

const blobs = await BlobsNode.spawn();
console.log(blobs);

log("iroh endpoint launched");
log(`our node id: ${blobs.endpoint_id()}`);

document.querySelector(".forms").style.display = "flex";
console.log(document.querySelector(".forms").style);

// when submitting the download form: download a blob from a ticket
document.querySelector("form#download").onsubmit = (e) => {
  e.preventDefault();
  const ticket = new FormData(e.target).get("ticket");
  if (!ticket) return;
  downloadBlob(ticket);
};

async function saveStream(stream, suggestedFilename) {
  const fileHandle = await showSaveFilePicker({
    suggestedName: suggestedFilename,
  });

  if (!fileHandle) return;

  const writer = await fileHandle.createWritable();
  await stream.pipeTo(writer);
}

async function downloadBlob(ticket) {
  try {
    log("downloading...");
    //const hash = await blobs.download(ticket);
    const stream = await blobs.download(ticket);
    await saveStream(stream, 'data.bin')
    log("download finished");
  } catch (err) {
    log(`download failed: ${err}`);
  }
}

function log(line, className) {
  const time = new Date().toISOString().substring(11, 22);
  const el = document.createElement("div");
  el.innerHTML = `<span class=time>${time}: </span>${line}`;
  if (className) el.classList.add(className);
  document.querySelector("main").appendChild(el);
}

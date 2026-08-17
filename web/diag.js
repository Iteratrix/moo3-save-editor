const wait = (ms) => new Promise((r) => setTimeout(r, ms));
try {
  const response = await fetch("./e2e-fixture.gam");
  const file = new File([await response.arrayBuffer()], "e2e-fixture.gam");
  const dataTransfer = new DataTransfer();
  dataTransfer.items.add(file);
  const ev = new DragEvent("drop", { dataTransfer, bubbles: true, cancelable: true });
  const item = ev.dataTransfer?.items?.[0];
  let handleInfo = "no-fn";
  if (item?.getAsFileSystemHandle) {
    handleInfo = "pending";
    try {
      const h = await Promise.race([item.getAsFileSystemHandle(), wait(2000).then(() => "timeout")]);
      handleInfo = h === "timeout" ? "timeout" : h === null ? "null" : `kind=${h.kind}`;
    } catch (e) { handleInfo = `threw:${e.name}`; }
  }
  document.title = `DIAG dt=${!!ev.dataTransfer} items=${ev.dataTransfer?.items?.length} files=${ev.dataTransfer?.files?.length} fsAccess=${"showOpenFilePicker" in window} handle=${handleInfo} fileSize=${file.size}`;
} catch (error) {
  document.title = `DIAG ERR ${error}`;
}

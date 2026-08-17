// Headless UI test, driven by scripts/ui-test.sh. Loaded from e2e.html (a
// generated copy of index.html); synthesizes a file drop of the fixture
// save against the real app.js handlers and reports via document.title.
const wait = (ms) => new Promise((resolve) => setTimeout(resolve, ms));

try {
  const response = await fetch("./e2e-fixture.gam");
  const file = new File([await response.arrayBuffer()], "e2e-fixture.gam");
  const dataTransfer = new DataTransfer();
  dataTransfer.items.add(file);
  document.getElementById("drop-zone").dispatchEvent(
    new DragEvent("drop", { dataTransfer, bubbles: true, cancelable: true }),
  );

  for (let i = 0; i < 100 && document.getElementById("editor").hidden; i++) await wait(100);

  const rows = document.querySelectorAll("#species-table tbody tr").length;
  const meta = document.getElementById("file-meta").textContent;

  document.querySelector('input[name=scope][value="everywhere"]').checked = true;
  document.getElementById("preview-btn").click();
  for (let i = 0; i < 50 && document.getElementById("plan-result").hidden; i++) await wait(100);
  const plan = document.getElementById("plan-summary").textContent;
  const planned = document.querySelectorAll("#plan-list li").length;

  const ok =
    rows > 0 &&
    meta.includes("250 systems") &&
    meta.includes("2794 populated regions") &&
    plan.includes("would become Klackon") &&
    planned > 0;
  document.title = ok
    ? `E2E PASS: ${rows} species; ${meta}; ${planned} planned; ${plan}`
    : `E2E FAIL: rows=${rows}; meta=${meta}; plan=${plan}; planned=${planned}; status=${document.getElementById("status").textContent}`;
} catch (error) {
  document.title = `E2E FAIL: ${error}`;
}

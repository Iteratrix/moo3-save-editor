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

  const turnValue = document.getElementById("turn-input").value;
  const empireRows = document.querySelectorAll("#empire-table tbody tr").length;
  const firstAu = document.querySelector("#empire-table tbody input");
  const auValue = firstAu?.value;

  document.getElementById("planet-query").value = "Alrisha I";
  document.getElementById("planet-search-btn").click();
  for (let i = 0; i < 50 && !document.querySelector(".region-row"); i++) await wait(100);
  const regionRows = document.querySelectorAll(".region-row").length;
  const firstPop = document.querySelector(".region-row input");
  let fieldsEnabled = false;
  if (firstPop) {
    firstPop.value = "9.9";
    firstPop.dispatchEvent(new Event("input"));
    fieldsEnabled = !document.getElementById("fields-save-btn").disabled;
  }

  const ok =
    rows > 0 &&
    meta.includes("250 systems") &&
    meta.includes("2794 populated regions") &&
    plan.includes("would become Klackon") &&
    planned > 0 &&
    turnValue === "115" &&
    empireRows === 11 &&
    auValue === "64573" &&
    regionRows > 0 &&
    fieldsEnabled;
  document.title = ok
    ? `E2E PASS: ${rows} species; ${meta}; turn ${turnValue}; ${empireRows} empires (you: ${auValue} AU); ${planned} planned; ${regionRows} region rows editable`
    : `E2E FAIL: rows=${rows}; meta=${meta}; plan=${plan}; planned=${planned}; turn=${turnValue}; empires=${empireRows}; au=${auValue}; regionRows=${regionRows}; fieldsEnabled=${fieldsEnabled}; status=${document.getElementById("status").textContent}`;
} catch (error) {
  document.title = `E2E FAIL: ${error}`;
}

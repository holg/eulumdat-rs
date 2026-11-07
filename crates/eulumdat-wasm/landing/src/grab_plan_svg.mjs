// Save the Interior designer's plan (with the sensor / emergency overlay) as
// a landing image.
//
// The plan is drawn by the app (ZonalSvg::room_plan_with_overlay + the
// sensor and emergency overlays), so the image is taken from a running
// local build through Chrome DevTools, in a fresh browser context (no saved
// projects or sensors of earlier sessions):
//
//   ./scripts/build-wasm-split.sh            # then serve dist/ (port 8042)
//   chrome --headless=new --remote-debugging-port=9333 &
//   node crates/eulumdat-wasm/landing/src/grab_plan_svg.mjs 9333 \
//     "http://localhost:8042/app/#interior?preset=2&sample=demo-led-3000k&sensor=demo0" \
//     crates/eulumdat-wasm/landing/img/sensor_planning.svg
//   node crates/eulumdat-wasm/landing/src/grab_plan_svg.mjs 9333 \
//     "http://localhost:8042/app/#interior?preset=0&sample=demo-led-3000k&sensor=none&shape=0,0;1,0;1,0.5;0.5,0.5;0.5,1;0,1&em=1&eml=route&emr=19.5,3.75;5,3.75;5,14.5~2&emp=fe:15,7.4,270_fa:0.1,10,0_ex:20,3.75,180_ex:5,15,270" \
//     crates/eulumdat-wasm/landing/img/emergency_plan.svg
//
// The numbers in the captions are pinned by tests: `sens_pcap` by
// crates/eulumdat/tests/sensor_demo.rs::landing_page_classroom_plan,
// `em_cap` by crates/eulumdat/tests/emergency.rs::landing_page_l_office.
const [port, url, out] = process.argv.slice(2);
const fs = await import("node:fs");
const version = await (await fetch(`http://127.0.0.1:${port}/json/version`)).json();
const ws = new WebSocket(version.webSocketDebuggerUrl);
let id = 0;
const pending = new Map();
ws.onmessage = (m) => {
  const msg = JSON.parse(m.data);
  if (msg.id && pending.has(msg.id)) { pending.get(msg.id)(msg); pending.delete(msg.id); }
};
await new Promise((r) => (ws.onopen = r));
const send = (method, params = {}, sessionId) =>
  new Promise((r) => {
    const i = ++id;
    pending.set(i, r);
    ws.send(JSON.stringify(sessionId ? { id: i, method, params, sessionId } : { id: i, method, params }));
  });
// a context of its own: empty storage, nothing of earlier sessions
const { browserContextId } = (await send("Target.createBrowserContext")).result;
const { targetId } = (await send("Target.createTarget", { url: "about:blank", browserContextId })).result;
const { sessionId } = (await send("Target.attachToTarget", { targetId, flatten: true })).result;
const page = (method, params = {}) => send(method, params, sessionId);
await page("Runtime.enable");
await page("Page.enable");
await page("Network.enable");
await page("Network.setCacheDisabled", { cacheDisabled: true });
await page("Page.navigate", { url });
await new Promise((r) => setTimeout(r, 12000));
const r = await page("Runtime.evaluate", {
  returnByValue: true,
  expression: `(function(){const s=document.querySelector('.zonal-plan-svg svg'); return s ? new XMLSerializer().serializeToString(s) : null;})()`,
});
let svg = r.result?.result?.value;
await send("Target.disposeBrowserContext", { browserContextId });
if (!svg) { console.error("no plan SVG on the page"); process.exit(1); }
// Inline in the app the text inherits the page font; as an <img> it would
// fall back to a serif font. Same font as the CLI diagrams.
svg = svg.replace("<svg ", '<svg font-family="system-ui, -apple-system, sans-serif" ');
fs.writeFileSync(out, svg + "\n");
console.log(
  "wrote", out, svg.length, "bytes,",
  (svg.match(/stroke="#7c3aed"/g) || []).length, "sensors,",
  (svg.match(/>E<\/text>/g) || []).length, "emergency luminaires",
);
ws.close();
process.exit(0);

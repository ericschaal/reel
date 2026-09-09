import { createServer } from "node:http";
import { readFile } from "node:fs/promises";
import { extname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = fileURLToPath(new URL("./prototype/playback/", import.meta.url));
const port = Number(process.env.PORT ?? 4173);
const files = new Map([
  ["/app.js", "app.js"],
  ["/styles.css", "styles.css"],
]);
const contentTypes = {
  ".html": "text/html; charset=utf-8",
  ".js": "text/javascript; charset=utf-8",
  ".css": "text/css; charset=utf-8",
};

createServer(async (request, response) => {
  const url = new URL(request.url ?? "/", `http://${request.headers.host}`);
  const filename = files.get(url.pathname) ??
    (url.pathname === "/" || url.pathname === "/prototype/playback"
      ? "index.html"
      : undefined);

  if (!filename) {
    response.writeHead(404).end("Not found");
    return;
  }

  try {
    const body = await readFile(join(root, filename));
    response.writeHead(200, {
      "content-type": contentTypes[extname(filename)],
      "cache-control": "no-store",
    });
    response.end(body);
  } catch {
    response.writeHead(500).end("Prototype file unavailable");
  }
}).listen(port, "127.0.0.1", () => {
  console.log(`Reel playback prototype: http://127.0.0.1:${port}/prototype/playback`);
});

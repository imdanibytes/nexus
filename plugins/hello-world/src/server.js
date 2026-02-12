const http = require("http");
const fs = require("fs");
const path = require("path");

const PORT = 80;
const NEXUS_TOKEN = process.env.NEXUS_TOKEN || "";
const NEXUS_API_URL =
  process.env.NEXUS_API_URL || "http://host.docker.internal:9600";

const publicDir = path.join(__dirname, "public");

const MIME_TYPES = {
  ".html": "text/html",
  ".css": "text/css",
  ".js": "application/javascript",
  ".json": "application/json",
  ".png": "image/png",
  ".svg": "image/svg+xml",
};

const server = http.createServer((req, res) => {
  // Health check
  if (req.url === "/health") {
    res.writeHead(200, { "Content-Type": "application/json" });
    res.end(JSON.stringify({ status: "ok" }));
    return;
  }

  // Token endpoint — lets the frontend JS retrieve the auth token
  if (req.url === "/api/config") {
    res.writeHead(200, {
      "Content-Type": "application/json",
      "Access-Control-Allow-Origin": "*",
    });
    res.end(
      JSON.stringify({
        token: NEXUS_TOKEN,
        apiUrl: NEXUS_API_URL,
      })
    );
    return;
  }

  // Serve static files
  let filePath = req.url === "/" ? "/index.html" : req.url;
  const fullPath = path.join(publicDir, filePath);
  const ext = path.extname(fullPath);
  const contentType = MIME_TYPES[ext] || "application/octet-stream";

  fs.readFile(fullPath, (err, data) => {
    if (err) {
      res.writeHead(404, { "Content-Type": "text/plain" });
      res.end("Not Found");
      return;
    }
    res.writeHead(200, { "Content-Type": contentType });
    res.end(data);
  });
});

server.listen(PORT, () => {
  console.log(`Hello World plugin running on port ${PORT}`);
});

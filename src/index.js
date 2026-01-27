#!/usr/bin/env node

const cli = require("./cli/cli.js");

// Parse command line arguments
const args = process.argv.slice(2);
const options = {
  port: null,
  nodeId: null,
  connect: null,
  autoStart: false,
};

// Parse arguments
for (let i = 0; i < args.length; i++) {
  const arg = args[i];

  if (arg === "-p" || arg === "--port") {
    options.port = parseInt(args[++i]);
  } else if (arg === "-n" || arg === "--node" || arg === "--id") {
    options.nodeId = args[++i];
  } else if (arg === "-c" || arg === "--connect") {
    options.connect = args[++i]; // format: host:port
  } else if (arg === "-a" || arg === "--auto") {
    options.autoStart = true;
  } else if (arg === "-h" || arg === "--help") {
    console.log(`
Mini Blockchain - A simple blockchain for learning

Usage: node src/index.js [options]

Options:
  -p, --port <port>      Port to run the P2P server on
  -n, --node <id>        Node ID for persistent storage
  -c, --connect <addr>   Connect to peer (format: host:port)
  -a, --auto             Auto-start server on the specified port
  -h, --help             Show this help message

Examples:
  # Start node 1 on port 3000
  node src/index.js -n node1 -p 3000 -a

  # Start node 2 on port 3001 and connect to node 1
  node src/index.js -n node2 -p 3001 -a -c localhost:3000

  # Start interactive mode
  node src/index.js
`);
    process.exit(0);
  }
}

// Start CLI with options
cli(options);

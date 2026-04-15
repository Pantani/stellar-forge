export const stellarState = {
  "api": {
    "database": "sqlite",
    "enabled": true,
    "events_backend": "rpc-poller",
    "framework": "fastify",
    "openapi": true,
    "relayer": false
  },
  "contracts": {
    "app": {
      "alias": "app",
      "bindings": [
        "typescript"
      ],
      "deploy_on": [
        "local",
        "testnet"
      ],
      "init": null,
      "path": "contracts/app",
      "template": "basic"
    }
  },
  "defaults": {
    "identity": "alice",
    "network": "testnet",
    "output": "human"
  },
  "deployment": {
    "contracts": {},
    "tokens": {}
  },
  "environment": "testnet",
  "events": {
    "backend": "rpc-poller",
    "contracts": [
      "app"
    ],
    "cursor_names": [],
    "cursors": {},
    "tokens": []
  },
  "frontend": {
    "enabled": true,
    "framework": "react-vite"
  },
  "network": {
    "allow_http": false,
    "friendbot": true,
    "horizon_url": "https://horizon-testnet.stellar.org",
    "kind": "testnet",
    "network_passphrase": "Test SDF Network ; September 2015",
    "rpc_url": "https://soroban-testnet.stellar.org"
  },
  "project": {
    "name": "demo",
    "package_manager": "pnpm",
    "slug": "demo",
    "version": "0.1.0"
  },
  "tokens": {},
  "wallets": {
    "alice": {
      "identity": "alice",
      "kind": "classic"
    }
  }
} as const;

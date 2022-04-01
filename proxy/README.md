# Proxy

A little Rust HTTP proxy that only allows certain requests through.

Examples:

```bash
# proxy <port> <host_to_proxy_to>
proxy 8080 localhost:5001
proxy 8080 rebase-ipfs.internal:5001
```

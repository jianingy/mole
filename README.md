# Introduction

Mole is an HTTP proxy scanner. It can be used for scanning and verifying HTTP proxy servers within a subnet.

# Features

- Find HTTP proxy servers in a given subnet.
- Find HTTP proxy servers from a given server list.
- Verify server capabilities:
  - Capable of HTTPS
  - Capable of CONNECT to arbitrary ports
- API for getting valid proxy servers

# Example

## Scan a subnet

```
mole scan 192.168.122.0/24 --database postgres://127.0.0.1/mole --workers 128
```

## Import server list and verify
```
mole import --database postgres://127.0.0.1/mole server_list
mole verify --database postgres://127.0.0.1/mole --workers 128
```

## Start API server and query proxy servers

```
mole serve --database postgres://127.0.0.1/mole --bind 127.0.0.1:3000 &
curl http://127.0.0.1:3000/api/v1/servers?lag=15&tags=HTTP_PROXY
```

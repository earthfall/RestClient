# HTTP Client

An HTTP Client implemented in Rust, compatible with the IntelliJ IDEA HTTP Client format.

## Features

- ✅ Parse and execute `.http` and `.rest` files
- ✅ HTTP/HTTPS request support (GET, POST, PUT, DELETE, etc.)
- ✅ WebSocket support
- ✅ GraphQL support
- ✅ Environment variable support (`{{variable}}`)
- ✅ Proxy configuration
- ✅ SSL/TLS certificate configuration
- ✅ cURL command conversion

## Installation

```bash
cargo build --release
```

The built binary is generated at `target/release/http-client`.

## Usage

### Running HTTP Requests

```bash
http-client run example.http
```

Using environment variables:

```bash
http-client run example.http --env dev
```

Specifying environment files:

```bash
http-client run example.http --env-file http-client.env.json --private-env-file http-client.private.env.json
```

### cURL Conversion

Convert cURL commands to HTTP request format:

```bash
http-client convert "curl https://api.example.com/users -H 'Accept: application/json'"
```

Convert HTTP request format to cURL commands:

```bash
http-client to-curl example.http
```

## HTTP File Format

### Basic HTTP Request

```http
### Get Users
GET https://api.example.com/users
Accept: application/json
```

### POST Request

```http
### Create User
POST https://api.example.com/users
Content-Type: application/json

{
  "name": "John Doe",
  "email": "john@example.com"
}
```

### Using Environment Variables

```http
### Get User
GET {{API_URL}}/users/{{USER_ID}}
Authorization: Bearer {{TOKEN}}
```

### WebSocket Request

```http
### WebSocket Connection
WEBSOCKET ws://localhost:8080/websocket
Content-Type: application/json

{
  "message": "Hello"
}

===
{
  "message": "Second message"
}

===
wait-for-server
{
  "message": "After server response"
}
```

### GraphQL Request

```http
### GraphQL Query
GRAPHQL http://localhost:8080/graphql

query {
  users {
    id
    name
    email
  }
}
```

Using GraphQL variables:

```http
### GraphQL Query with Variables
GRAPHQL http://localhost:8080/graphql

query ($id: ID!) {
  user(id: $id) {
    id
    name
  }
}

{
  "id": "{{USER_ID}}"
}
```

## Environment Variable Files

### http-client.env.json

Public environment variable file:

```json
{
  "dev": {
    "API_URL": "https://api-dev.example.com",
    "TOKEN": "dev-token-123"
  },
  "prod": {
    "API_URL": "https://api.example.com",
    "TOKEN": "prod-token-456"
  }
}
```

### http-client.private.env.json

Private environment variable file (contains sensitive information):

```json
{
  "dev": {
    "API_KEY": "secret-key-123",
    "SSLConfiguration": {
      "clientCertificate": "cert.pem",
      "clientCertificateKey": "key.pem",
      "verifyHostCertificate": false
    }
  }
}
```

## Examples

Example files are available in the project's `examples/` directory.

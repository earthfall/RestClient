# HTTP Client Specification

## Overview

The HTTP Client plugin in IntelliJ IDEA allows you to create, edit, and execute HTTP requests directly in the code editor. HTTP requests are stored in `.http` and `.rest` files and are marked with the HTTP file icon.

### Main Use Cases

1. **Developing RESTful web services**: Make sure the service works as expected, is accessible in compliance with the specification, and responds correctly.
2. **Developing applications that consume RESTful web services**: Investigate the access to the service and required input data before development. During development, call the web service from outside your application to locate errors.

## Features

HTTP Client support includes:

- Code highlighting
- Code completion for hosts, method types, header fields, and endpoints defined via OpenAPI
- Code folding for requests, their parts, and response handler scripts
- Reformat requests according to your HTTP Request code style
- Inline documentation for request header fields and doc tags
- Viewing the structure of HTTP request files
- Language injections in Web languages inside the request message body
- Move refactorings
- Live templates

## Creating HTTP Request Files

### Scratch Files

Scratch files can be used to test HTTP requests during development. Scratch files are not stored inside a project, so IntelliJ IDEA can modify them and add additional information about the request. When an HTTP request is executed from a scratch file, the link to the response output file is added below the request and at the top of the requests history file.

**Create an HTTP request scratch file:**
- Press `Ctrl+Alt+Shift+Insert` and select HTTP Request.

### Physical Files

Physical files can be used for documenting, testing, and validating HTTP requests. Physical files are stored inside your project, and IntelliJ IDEA will not modify them. When an HTTP request is executed from a physical file, this file is not modified. Information about the executed request with the link to the response output file is added to the top of the requests history file.

**Create a physical HTTP request file:**
- In the File menu, point to New, and then click HTTP Request.

### Moving HTTP Requests

You can use the Move refactoring (`F6`) to move HTTP requests from scratches to physical files, as well as between physical files.

1. In the editor, place the caret at the request to be moved and do one of the following:
   - From the main menu or the context menu, select Refactor | Move
   - Press `Alt+Enter` and select the Move HTTP Requests intention action
   - Press `F6`

2. In the Move HTTP Requests dialog:
   - In the Path field, choose one of the existing `.http` files from the list or click the Browse button to locate the file
   - You can also type the full path to the file manually. If you specify the name of a non-existing file, a new file with the provided name will be created automatically
   - In the Requests list, select the checkboxes next to the requests you want to move

## Composing HTTP Requests

IntelliJ IDEA uses the HTTP request in Editor format, which provides a simple way to create, execute, and store information about HTTP requests. You can type them directly in the created HTTP request files using the following general syntax:

```
### Method Request-URI HTTP-Version
Header-field: Header-value
Request-Body
```

After the `###` separator, you can enter any comments preceded by `#` or `//`.

To quickly find your request in run/debug configurations, Search Everywhere, and Run Anything, you can give it a name.

You can use the Editor | Color Scheme | HTTP Request settings to customize colors and style for highlighting request syntax (name, comments, parameters, headers, and so on).

### Quick Request Creation

To speed up composing HTTP requests, you can:

- Click Tools | HTTP Client | Create Request in HTTP Client. If a request file is opened in the editor, this will add a request template to the opened file. Otherwise, this will create a new `.http` scratch file.
- Click the Add Request button on top of the request's editor panel. In the popup menu, select the type of the request to add.
- Use live templates. In the editor, you can press `Ctrl+J` to view the list of available templates. For example:
  - `gtr` expands to a simple GET request
  - `mptr` expands to a `multipart/form-data` POST request

### URL Encoding

If you use `application/x-www-form-urlencoded` content type, you should use `%` to escape `%`, `&`, `=`, and `+` special characters in the keys and values of the request body.

For example, for a server to receive `field1=value%2Bvalue&field2=value%26value` (where `%2B` corresponds to `+` and `%26` corresponds to `&`), the request body should be as follows:

```
POST https://ijhttp-examples.jetbrains.com/post
Content-Type: application/x-www-form-urlencoded

field1=value%+value&field2=value%&value
```

Or you can write encoded values right away, for example `value1%ADvalue2` for `value1 value2` or `value1%2Bvalue2` for `value1+value2`.

### Custom HTTP Methods

If a web service requires you to use custom HTTP methods, you can add such methods to IntelliJ IDEA and use them in your HTTP requests.

1. In an `.http` file, type a custom method in uppercase letters.
2. When this method is highlighted as unknown, press `Alt+Enter` (Show Context Actions) and select Add custom HTTP method.

IntelliJ IDEA will now recognize it as a valid HTTP method. You can find all custom HTTP methods (and add new ones) in the IDE settings (`Ctrl+Alt+S`), under Tools | HTTP Client | Custom HTTP methods.

### HTTP/2 Support

Starting with version 2024.1, IntelliJ IDEA provides support for HTTP/2 in HTTP requests. You can specify the HTTP version after the URL part, for example:

```
GET https://example.org HTTP/2
```

If no version is specified, the HTTP Client attempts to use HTTP/2 for secure connections (and falls back to HTTP/1.1 if HTTP/2 negotiation fails) and HTTP/1.1 for non-secure connections.

**Select HTTP version:**

1. After the request URL, put a white space and press `Ctrl+Space` or start typing `HTTP`.
2. From the completion list, select one of the suggested values:
   - `HTTP/1.1` to enforce the use of HTTP/1.1
   - `HTTP/2` to enable the use of HTTP/2
   - `HTTP/2 (Prior Knowledge)` to send using HTTP/2 without HTTP/1.1 Upgrade. Use it if you know your server can handle HTTP/2 connections

### HTTP Requests Collection

To get an overview of the HTTP Client features, you can explore the HTTP Requests Collection, which is a handful selection of composed requests.

**Open a request from the HTTP Requests Collection:**

1. Click the Examples shortcut link on top of the request's editor panel.
2. In the popup menu, choose the HTTP Requests collection you wish to open.

### Converting cURL Requests

If you are working with cURL requests, you can convert between cURL requests and the HTTP request in Editor format.

**Convert cURL to HTTP request:**

- Paste the cURL request into an HTTP request file. IntelliJ IDEA will convert it to the HTTP request format and leave the original cURL request commented out for later reference.
- Alternatively, click Convert cURL to HTTP request on top of the HTTP request editor panel and select Convert cURL to HTTP Request. In the Convert cURL to HTTP Request dialog, type or paste the cURL request that you want to convert.

**Example cURL request:**

```bash
curl 'https://httpbin.org/' -H 'Connection: keep-alive' -H 'Accept: text/html' -H 'Accept-Encoding: gzip, deflate' -H 'Accept-Language: en-US,en;q=0.9,es;q=0.8'
```

IntelliJ IDEA will convert it to the HTTP request format.

## WebSocket Requests

The HTTP Client supports WebSocket requests. For the HTTP Client to treat your request as a WebSocket request, start it with the `WEBSOCKET` keyword followed by a server address. The request has the following structure:

```
WEBSOCKET ws://localhost:8080/websocket
Content-Type: application/json

// Used for content highlighting only
// Request body, for example:
{
  "message": "First message sent on connection"
}

===
// message separator
{
  "message": "Second message"
  // will be sent right after the previous one
}

===
wait-for-server
// keyword used to wait for the server response
{
  "message": "Send this after the server response"
}
```

While the `Content-Type` header is not used in WebSocket connections, you can use it in IntelliJ IDEA WebSocket requests to highlight syntax of transmitted data.

**Quick WebSocket request creation:**

- Click The Add Request button on top of the editor panel of an `.http` file and select WebSocket Request.
- In an `.http` file, type `wsr` and press Enter to apply the WebSocket live template.

### Sending Multiple Messages

Use the `===` separator to send multiple messages:

```
{
  "message": "First message sent on connection"
}

===
// message separator
{
  "message": "Second message"
}

===
{
  "message": "Third message"
}
```

### Sending Messages After Server Response

Before a message, enter `=== wait-for-server`. This will make the HTTP Client wait for the server response before sending the message. You can wait for multiple responses by repeating the `=== wait-for-server` line. For example, the following message will be sent after 3 server responses:

```
===
wait-for-server
===
wait-for-server
===
wait-for-server
{
  "message": "This messages is sent after 3 server responses"
}
```

### Interactive WebSocket Messaging

Once you have initiated a connection, you can interact with your server right from the Services tool window. You can send messages and view server responses to each new message.

1. In the Services tool window, select an opened connection.
2. In the lower part of the window, under Message to be sent to WebSocket, enter the message content.
3. To the right of it, select the message format: plain text, JSON, XML, or HTML.
4. Press `Ctrl+Enter` to send the request.

In the upper part of the window, you'll see the server response.

## GraphQL Support

IntelliJ IDEA provides support for sending GraphQL operations in the HTTP request body. You can send them over HTTP or WebSocket.

For the GraphQL language support in the request body (syntax highlighting, quick navigation to schemas, and so on), you can install and enable the GraphQL plugin.

### Compose an HTTP Request with GraphQL Query

1. In an `.http` file, enter the `GRAPHQL` keyword followed by a server address.
2. In the request body, compose your GraphQL operation (query, mutation, or subscription), for example:

```
### HTTP request with GraphQL query
GRAPHQL http://localhost:8080/graphql

query {
  toDos {
    title,
    completed,
    author {
      username
    }
  }
}
```

**Quick GraphQL request creation:**

- Click The Add Request button on top of the editor panel of an `.http` file and select GraphQL Query Request.
- In an `.http` file, type `gqlr` and press Enter to apply the GraphQL live template.

### Using GraphQL Variables

In the HTTP request body, you can use GraphQL variables if you want to pass some dynamic data separately from the query string.

After the query part, enter a JSON variables dictionary:

```
query ($name: String!, $capital: String!) {
  country(name: $name, capital: $capital) {
    name
    capital
  }
}

{
  "name": "France",
  "capital": "Paris"
}
```

You can also use HTTP Client environment variables as GraphQL variable values. For example, in this JSON, `"{{Author}}"` is an environment variable; its value at runtime depends on the environment that you select while sending the request:

```json
{
  "author": "{{Author}}"
}
```

You can quickly add a variable block to the GraphQL query by pressing `Alt+Enter` (Show Context Actions) in the request body and selecting Add GraphQL JSON variables block.

### Create GraphQL Request from Spring Controller

If you use GraphQL in your Spring applications, you can quickly create HTTP requests from your controller source code using the dedicated gutter icon.

To use this feature, install and enable the GraphQL and Spring GraphQL plugins. IntelliJ IDEA suggests installing the Spring GraphQL plugin if it detects GraphQL dependencies in your Spring project.

1. In your Spring controller code, click GraphQL gutter icon in the gutter next to the `@QueryMapping`, `@MutationMapping`, or `@SubscriptionMapping` annotation.
2. In the context menu that opens, select Generate request in HTTP Client.

This will add a new `GRAPHQL` request to the generated-requests.http file.

## Configuration

### Proxy Settings

1. In the Settings dialog (`Ctrl+Alt+S`), choose System Settings under Appearance & Behavior, then choose HTTP Proxy.
2. In the HTTP Proxy dialog that opens, select Manual proxy configuration and specify the following:
   - Enter the proxy host name and port number in the Host name and Port number fields.
   - To enable authorization, select the Proxy authentication checkbox and type the username and password in the corresponding fields.

### Client SSL/TLS Certificate

If an HTTP server requires SSL/TLS authentication for secure communication, you may need to specify the client certificate before sending an HTTPS request. In the HTTP Client, you can set up the client certificate using the private environment file.

Currently, configuring an SSL/TLS certificate is not supported in HTTP Client CLI. The only supported SSL-related setting in HTTP Client CLI is the ability to disable certificate verification.

#### Specify Path to Certificate

1. In an `.http` file, in the Run with list, select Add Environment to Private File.
2. In the `http-client.private.env.json` file that opens, add the `SSLConfiguration` object to the needed environment. In `clientCertificate`, enter a path to your client certificate. If a certificate key is stored in a separate file, enter its path in `clientCertificateKey`. For example:

```json
{
  "dev": {
    "MyVar": "SomeValue",
    "SSLConfiguration": {
      "clientCertificate": "cert.pem",
      "clientCertificateKey": "MyFolder/key.pem"
    }
  }
}
```

You can specify an absolute path or a path relative to the `http-client.private.env.json` file. If the environment file is stored in scratches, you can additionally specify a path relative to your project root. Start typing a path to get the code completion popup.

Alternatively, you can describe `clientCertificate` and `clientCertificateKey` as objects, which lets you specify the certificate format in addition to the path. For example:

```json
{
  "dev": {
    "SSLConfiguration": {
      "clientCertificate": {
        "path": "file.crt",
        "format": "PEM"
      },
      "clientCertificateKey": {
        "path": "file.key",
        "format": "DER"
      }
    }
  }
}
```

#### Set Up a Certificate Passphrase

If you used a passphrase when generating your client certificate, you should provide it to the HTTP Client.

1. In the `http-client.private.env.json` file, add `"hasCertificatePassphrase": true` to the `SSLConfiguration` object, for example:

```json
{
  "dev": {
    "SSLConfiguration": {
      "clientCertificate": "file.crt",
      "hasCertificatePassphrase": true
    }
  }
}
```

2. Click Set value for certificate passphrase in the gutter or, with the caret placed at `hasCertificatePassphrase`, press `Alt+Enter` and select Set value for 'Certificate passphrase'.
3. In the window that opens, enter your certificate passphrase.

You can omit the second step if you do not want to enter the passphrase now. In this case, IntelliJ IDEA will prompt you to enter the passphrase when you execute an HTTPS request.

#### Disable Certificate Verification

For development purposes, you may have a host with self-signed or expired certificates. If you trust this host, you can disable verification of its certificate.

In the `http-client.private.env.json` file, add `"verifyHostCertificate": false` to the `SSLConfiguration` object. For example:

```json
{
  "sslTest": {
    "SSLConfiguration": {
      "verifyHostCertificate": false
    }
  }
}
```

If you run a request with this environment, IntelliJ IDEA will not verify host certificates.

## Client API Reference

The `client` object provides methods for testing and working with HTTP responses:

- `test()`: Create and run response tests
- `assert()`: Check conditions in responses
- `log()`: Print output text
- `exit()`: Terminate script execution

### Global Variables

Use `client.global.set()` and `client.global.get()` for variable storage across requests. This allows you to reuse variables and headers across multiple requests in your HTTP files.

## References

- [HTTP Client Documentation](https://www.jetbrains.com/help/idea/http-client-in-product-code-editor.html)
- [Exploring the HTTP request syntax](https://www.jetbrains.com/help/idea/exploring-http-syntax.html)
- [HTTP Client reference](https://www.jetbrains.com/help/idea/http-client-reference.html)

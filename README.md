<div align="center">
<p align="center">
  <a href="https://www.edgee.cloud">
    <picture>
      <source media="(prefers-color-scheme: dark)" srcset="https://cdn.edgee.cloud/img/component-dark.svg">
      <img src="https://cdn.edgee.cloud/img/component.svg" height="100" alt="Edgee">
    </picture>
  </a>
</p>
</div>

<h1 align="center">Slack message component for Edgee</h1>

[![Coverage Status](https://coveralls.io/repos/github/edgee-cloud/slack-message-component/badge.svg)](https://coveralls.io/github/edgee-cloud/slack-message-component)
[![GitHub issues](https://img.shields.io/github/issues/edgee-cloud/slack-message-component.svg)](https://github.com/edgee-cloud/slack-message-component/issues)
[![Edgee Component Registry](https://img.shields.io/badge/Edgee_Component_Registry-Public-green.svg)](https://www.edgee.cloud/edgee/slack-message)


This component provides a simple way to send Slack messages on [Edgee](https://www.edgee.cloud),
served directly at the edge. You map the component to a specific endpoint such as `/slack-message`, and
then you invoke it from your frontend code.


## Quick Start

1. Download the latest component version from our [releases page](../../releases)
2. Place the `slack.wasm` file in your server (e.g., `/var/edgee/components`)
3. Add the following configuration to your `edgee.toml`:

```toml
[[components.edge_functions]]
id = "slack-message"
file = "/var/edgee/components/slack.wasm"
settings.edgee_path = "/slack-message"
settings.webhook_url = "https://hooks.slack.com/services/XYZ"
```

### How to use the HTTP endpoint

You can send requests to the endpoint as follows:

```javascript

await fetch('/slack-message', {
  method: 'POST',
  body: JSON.stringify({
    "message": "hello world!",
  })
});

```

## Development

### Building from Source
Prerequisites:
- [Rust](https://www.rust-lang.org/tools/install)

Build command:
```bash
edgee component build
```

Test command (with local HTTP emulator):
```bash
edgee component test
```

Test coverage command:
```bash
make test.coverage[.html]
```

### Contributing
Interested in contributing? Read our [contribution guidelines](./CONTRIBUTING.md)

### Security
Report security vulnerabilities to [security@edgee.cloud](mailto:security@edgee.cloud)

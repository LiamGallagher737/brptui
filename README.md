<div align="center">

# brptui

A [Bevy Remote Protocol](https://bevyengine.org/news/bevy-0-15/#bevy-remote-protocol-brp) client for the terminal.

</div>

## Features

#### Complete

- Viewing entities and their components
- Despawning entities and removing components (<kbd>x</kbd>)

#### To come

- Respect entity hierarchy (parent and child entities)

#### Blocked by BRP capabilities

- Viewing and editing resources

## Installation

- **Source:** `cargo install --git https://github.com/LiamGallagher737/brptui`

## Usage

Enable the `bevy_remote` feature for the Bevy dependency in your projecet.

```
cargo add bevy -F bevy_remote
```

```toml
[dependencies]
bevy = { version = "0.15", features = ["bevy_remote"] }
```

Then add `RemotePlugin` and `RemoteHttpPlugin` to your app.

```rs
use bevy::prelude::*;
use bevy::remote::{http::RemoteHttpPlugin, RemotePlugin};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(RemotePlugin::default())
        .add_plugins(RemoteHttpPlugin::default())
        .run();
}
```

Now you can run `brptui` to inspect the entities in your running app using the BRP.

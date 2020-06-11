i3wsr - i3 workspace renamer
======
[![Build Status](https://travis-ci.org/roosta/i3wsr.svg?branch=master)](https://travis-ci.org/roosta/i3wsr)
![Crates.io](https://img.shields.io/crates/v/i3wsr)

`i3wsr` is a small program that uses [I3's](https://i3wm.org/) [IPC Interface](https://i3wm.org/docs/ipc.html)
to change the name of a workspace based on its contents.

## Table of content

<!-- vim-markdown-toc GFM -->

* [Details](#details)
* [Installation](#installation)
  * [Arch linux](#arch-linux)
* [Usage](#usage)
* [i3 configuration](#i3-configuration)
* [Configuration / options](#configuration--options)
  * [Icons](#icons)
  * [Aliases](#aliases)
  * [Seperator](#seperator)
  * [Default icon](#default-icon)
  * [No names](#no-names)
  * [Remove duplicates](#remove-duplicates)
  * [Use instance](#use-instance)
* [Sway](#sway)
* [Contributors](#contributors)
* [Test environment](#test-environment)
* [Attribution](#attribution)

<!-- vim-markdown-toc -->

## Details

The chosen name for a workspace is a composite of the `WM_CLASS` X11 window
property for each window in a workspace. In action it would look something like this:

![](https://raw.githubusercontent.com/roosta/i3wsr/master/assets/preview.gif)

## Installation
[Rust](https://www.rust-lang.org/en-US/), and [Cargo](http://doc.crates.io/) is
required, and `i3wsr` can be installed using cargo like so:

```sh
cargo install i3wsr
```

Or alternatively, you can build a release binary,

```sh
cargo build --release
```

Then place the built binary, located at `target/release/i3wsr`, somewhere on your `$path`.

### Arch linux
If you're running Arch you can install either [stable](https://aur.archlinux.org/packages/i3wsr/), or [latest](https://aur.archlinux.org/packages/i3wsr-git/) from AUR thanks to reddit user [u/OniTux](https://www.reddit.com/user/OniTux).

## Usage
Just launch the program and it'll listen for events if you are running I3.
Another option is to put something like this in your i3 config

```
# cargo
exec_always --no-startup-id $HOME/.cargo/bin/i3wsr
# AUR
exec_always --no-startup-id /usr/bin/i3wsr
```

## i3 configuration

This program depends on numbered workspaces, since we're constantly changing the
workspace name. So your I3 configuration need to reflect this:

```
bindsym $mod+1 workspace number 1
```

If you're like me and don't necessarily bind your workspaces to only numbers, or
you want to keep a part of the name constant you can do like this:

```
bindsym $mod+q workspace number 1:[Q]
```

This way the workspace would look something like this when it gets changed:

```
1:[Q] Emacs|Firefox
```
You can take this a bit further by using a bar that trims the workspace number and be left with only
```
[Q] Emacs|Firefox
```

## Configuration / options

Configuration for i3wsr can be done using cmd flags, or a config file. A config
file allows for more nuanced settings, and is required to configure icons and
aliases. To use a config file pass to the `--config` option on invocation:
```bash
i3wsr --config ~/my_config.toml
```
Example config can be found in
[assets/example_config.toml](https://github.com/roosta/i3wsr/blob/master/assets/example_config.toml).


### Icons
You can configure icons for the respective classes, a very basic preset for
font-awesome is configured, to enable it use the option `--icons awesome`
(requires font-awesome to be installed).

A more in depth icon configuration can be setup by using a configuration file.
In there you can define icons for whatever class you'd like.
```toml
[icons]
Firefox = "🌍"

# Use quote when matching anything other than [a-zA-Z]
"Org.gnome.Nautilus" = "📘"
```
A font that provides icons is of course recommended, like
[font-awesome](https://fontawesome.com/). Make sure your bar has that font
configured.

### Aliases
Sometimes class names for windows can be overly verbose, so its possible to
match a class name with an alias:

```toml
[aliases]
Google-chrome-unstable = "Chrome-dev"

# Use quote when matching anything other than [a-zA-Z]
"Org.gnome.Nautilus" = "Nautilus"
```
Now i3wsr will display the alias instead of the full class name.

### Seperator

Normally i3wsr uses the pipe character `|` between class names in a workspace,
but a custom separator can be configured in the config file:
```toml
[general]
separator = "  "
```

### Default icon
To use a default icon when no other is defined use:
```toml
[general]
default_icon = "💀"
```

### No names
If you have icons and don't want the names to be displayed, you can use the
`--no-names` flag, or enable it in your config file like so:
```toml
[options]
no_names = true
```

### Remove duplicates
If you want duplicates removed from workspaces use either the flag
`--remove-duplicates`, or configure it in the `options` section of the config
file:
```toml
[options]
remove_duplicates = true
```

### Use instance
Use WM_INSTANCE instead of WM_CLASS when assigning workspace names, instance is
usually more specific. i3wsr will try to match icon with instance, and if that
fail, will fall back to class.

To enable this, either pass the flag `--use-instance`, or add it in your config
file under `options`.
```toml
[options]
use_instance = true
```

A use case for this option could be launching `chromium
--app="https://web.whatsapp.com"`, and then assign a different icon to whatsapp
in your config file:
```toml
[icons]
"web.whatsapp.com" = "💧"
```

Aliases will also match on instance:
```toml
[aliases]
"web.whatsapp.com" = "WhatsApp"
```

## Sway
Check [Pedro Scaff](https://github.com/pedroscaff)'s port [swaywsr](https://github.com/pedroscaff/swaywsr).

## Contributors
* [Daniel Berg (roosta)](https://github.com/roosta)
* [Cauê Baasch de Souza (cauebs)](https://github.com/cauebs)
* [Pedro Scaff (pedroscaff)](https://github.com/pedroscaff)
* [Ben Brooks (bbrks)](https://github.com/bbrks)
* [luukvbaal](https://github.com/luukvbaal)

## Test environment
To run the tests `Xvfb` needs to be installed and run:

```shell
Xvfb :99.0
```
This sets up a headless x server running on DISPLAY :99.0, then some apps needs to be run in this new server:

```shell
env DISPLAY=:99.0 gpick
env DISPLAY=:99.0 i3 -c /etc/i3/config
```

refer to [.travis.yml](https://github.com/roosta/i3wsr/blob/master/.travis.yml) for a CI example

## Attribution
This program would not be possible without
[i3ipc-rs](https://github.com/tmerr/i3ipc-rs), a rust library for controlling
i3-wm through its IPC interface and
[rust-xcb](https://github.com/rtbo/rust-xcb), a set of rust bindings and
wrappers for [XCB](http://xcb.freedesktop.org/).

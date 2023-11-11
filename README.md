i3wsr - i3 workspace renamer
======

[![Test Status](https://github.com/roosta/i3wsr/actions/workflows/test.yaml/badge.svg?branch=develop)](https://github.com/roosta/i3wsr/actions)
[![Crates.io](https://img.shields.io/crates/v/i3wsr)](https://crates.io/crates/i3wsr)


`i3wsr` is a small program that uses [I3's](https://i3wm.org/) [IPC Interface](https://i3wm.org/docs/ipc.html)
to change the name of a workspace based on its contents.

## TOC

- [i3wsr - i3 workspace renamer](#i3wsr---i3-workspace-renamer)
- [TOC](#toc)
    - [Details](#details)
    - [Requirements](#requirements)
    - [Installation](#installation)
        - [Arch linux](#arch-linux)
    - [Usage](#usage)
    - [i3 configuration](#i3-configuration)
        - [Keeping part of the workspace name](#keeping-part-of-the-workspace-name)
    - [Configuration / options](#configuration--options)
        - [Aliases](#aliases)
        - [Aliases based on property](#aliases-based-on-property)
            - [Class](#class)
            - [Instance](#instance)
            - [Name](#name)
        - [Display property](#display-property)
        - [Icons](#icons)
        - [Separator](#separator)
        - [Default icon](#default-icon)
        - [Empty label](#empty-label)
        - [No icon names](#no-icon-names)
        - [No names](#no-names)
        - [Remove duplicates](#remove-duplicates)
        - [Split at character](#split-at-character)
    - [Sway](#sway)
    - [Testing](#testing)
    - [Attribution](#attribution)

## Details

The chosen name for a workspace is a composite of the `WM_CLASS` X11 window
property for each window in a workspace. In action it would look something like this:

![](https://raw.githubusercontent.com/roosta/i3wsr/main/assets/preview.gif)
## Requirements

i3wsr requires [i3wm](https://i3wm.org/) and [numbered
workspaces](https://i3wm.org/docs/userguide.html#_changing_named_workspaces_moving_to_workspaces),
see [i3-configuration](#i3-configuration)

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
assign [class="(?i)firefox"] number 1
```

### Keeping part of the workspace name

If you're like me and don't necessarily bind your workspaces to only numbers,
or you want to keep a part of the name constant you can do like this:

```
set $myws "1:[Q]" # my sticky part
bindsym $mod+q workspace number $myws
assign [class="(?i)firefox"] number $myws
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
aliases. By default i3wsr looks for the config file at
`$XDG_HOME/.config/i3wsr/config.toml` or `$XDG_CONFIG_HOME/i3wsr/config.toml`.
To specify another path, pass it to the `--config` option on invocation:
```bash
i3wsr --config ~/my_config.toml
```
Example config can be found in
[assets/example\_config.toml](https://github.com/roosta/i3wsr/blob/main/assets/example_config.toml).


### Aliases


Sometimes a class, instance or name can be overly verbose, use aliases that
match to window properties to create simpler names instead of showing the full
property


```toml
[aliases.class]

# Exact match
"^Google-chrome-unstable$" = "Chrome-dev"

# Substring match
firefox = "Firefox"

# Escape if you want to match literal periods
"Org\\.gnome\\.Nautilus" = "Nautilus"
```
Alias keys uses regex for matching, so it's possible to get creative:

```toml
# This will match gimp regardless of version number reported in class
"Gimp-\\d\\.\\d\\d" = "Gimp"
```

Remember to quote anything but `[a-zA-Z]`, and to escape your slashes. Due to
rust string escapes if you want a literal backslash use two slashes `\\d`.

### Aliases based on property

i3wsr supports 3 window properties currently:

```toml
[aliases.name]     // 1
[aliases.instance] // 2
[aliases.class]    // 3
```
These are checked in descending order, so if i3wsr finds a name alias, it'll
use that and if not, then check instance, then finally use class

> Deprecation note: previously `wm_property` defined which prop to check for
> aliases, but this newer approach will allow for multiple types of aliases

#### Class

This is the default, and the most succinct.

#### Instance

Use `WM_INSTANCE` instead of `WM_CLASS` when assigning workspace names,
instance is usually more specific. i3wsr will try to get the instance but if it
isn't defined will fall back to class.

A use case for this option could be launching `chromium
--app="https://web.whatsapp.com"`, and then assign a different icon to whatsapp
in your config file, while chrome retains its own alias:
```toml
[icons]
"WhatsApp" = "🗩"

[aliases.class]
Google-chrome = "Chrome"

[aliases.instance]
"web\\.whatsapp\\.com" = "Whatsapp"
```

#### Name

Uses `WM_NAME` instead of  `WM_INSTANCE` and `WM_CLASS`, this option is very
verbose and relies on regex matching of aliases to be of any use.

A use-case is running some terminal application, and as default i3wsr will only
display class regardless of whats running in the terminal.

So you could do something like this:

```toml
[aliases.name]
".*mutt$" = "Mutt"
```

You could display whatever the terminal is running, but this comes with one
caveat: i3 has no way of knowing what happens in a terminal and starting say
mutt will not trigger any IPC events. The alias will take effect whenever i3
receives a window or workspace event.

It should be possible to write a launcher script, that wraps whatever
command your running with a custom i3 ipc trigger event. If anyone figures out
a nice way of doing it let me know.

### Display property

Which property to display if no aliases if found:

```toml
[general]
display_property = "instance"
```

Possible options are `class`, `instance`, and `name`, and will default to `class`
if not present.

### Icons

You can configure icons for your WM property, a very basic preset for
font-awesome is configured, to enable it use the option `--icons awesome`
(requires font-awesome to be installed).

A more in depth icon configuration can be setup by using a configuration file.
In there you can define icons for whatever title you'd like.
```toml
[icons]
Firefox = "🌍"

# Use quote when matching anything other than [a-zA-Z]
"Org.gnome.Nautilus" = "📘"
```
i3wsr tries to match an icon with an alias first, if none are found it then
checks your `display_property`, and tries to match an icon with a non aliased
`display_property`, lastly it will try to match on class.

```toml
[aliases.class]
"Gimp-\\d\\.\\d\\d" = "Gimp"

[icons]
Gimp = "📄"
```

A font that provides icons is of course recommended, like
[font-awesome](https://fontawesome.com/). Make sure your bar has that font
configured.

### Separator

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
### Empty label

Set a label for empty workspaces.

```toml
[general]
empty_label = "🌕"
```
### No icon names
To display names only if icon is not available, you can use the
`--no-icon-names` flag, or enable it in your config file like so:
```toml
[options]
no_icon_names = true
```
### No names
If you don't want i3wsr to display names at all, you can use the
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

### Split at character

By default i3wsr will keep everything until the first `space` character is found,
then replace the remainder with titles.

If you want to define a different character that is used to split the
numbered/constant part of the workspace and the dynamic content, you can use
the option `--split-at [CHAR]`

```toml
[general]
split_at = ":"
```

Here we define colon as the split character, which results in i3wsr only
keeping the numbered part of a workspace name when renaming.

This can give a cleaner config, but I've kept the old behavior as default.


## Sway
    Check [Pedro Scaff](https://github.com/pedroscaff)'s port [swaywsr](https://github.com/pedroscaff/swaywsr).

## Testing

To run tests locally [Vagrant](https://www.vagrantup.com/) is required. Run
`script/run_tests.sh` to run tests on ubuntu xenial.

## Attribution

This program would not be possible without
[i3ipc-rs](https://github.com/tmerr/i3ipc-rs), a rust library for controlling
i3wm through its IPC interface and
[rust-xcb](https://github.com/rtbo/rust-xcb), a set of rust bindings and
wrappers for [XCB](http://xcb.freedesktop.org/).

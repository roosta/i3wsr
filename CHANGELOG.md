# Changelog

All notable changes to this project will be documented in this file.

## [Unreleased] - 2025-07-21

## [v3.1.2] - 2025-07-21
### Bug fixes

- Fix issue with unwanted workspace focus change on rename dispatches. Add
  feature flag for the focus fix, now needs to be enabled manually via either
  config or cmdline flags
  `--focus-fix`.
  ```toml
  [options]
  focus_fix = true
  ```

Ref: https://github.com/roosta/i3wsr/issues/42

## [v3.1.1] - 2025-01-25

### Bug Fixes

- Fix an issue where `i3wsr` would not collect `i3` titles correctly. This also
  more cleanly handles the differing tree structures between sway and i3
  without the need for a conditional.

## [v3.1.0] - 2025-01-22

### Bug Fixes

- Fix [multi-monitor window dragging issue specific to i3](https://github.com/roosta/i3wsr/issues/34)
    - Sway doesn't trigger any events on window drag

### Features

- Add sway support
- Add `--verbose` cmdline flag for easier debugging in case of issues

#### Sway

Support for [Sway](https://github.com/swaywm/sway) is added, new config key
addition `app_id` in place of `class` when running native Wayland applications:

```
[aliases.app_id]
firefox-developer-edition = "Firefox Developer"
```
`i3wsr` will still check for `name`, `instance`, and `class` for `Xwayland`
windows, where applicable. So some rules can be preserved. To migrate replace
`[aliases.class]` with `[aliases.app_id]`, keep in mind that `app_id` and
`class` aren't always interchangeable , so some additional modifications is
usually needed.

> A useful script figuring out `app_id` can be found [here](https://gist.github.com/crispyricepc/f313386043395ff06570e02af2d9a8e0#file-wlprop-sh), it works like `xprop` but for Wayland.

### Deprecations

I've flagged `--icons` as deprecated, it will not exit the application but it
no longer works. I'd be surprised if anyone actually used that preset, as it
was only ever for demonstration purposes, and kept around as a holdover from
previous versions.

## [v3.0.0] - 2024-02-19

**BREAKING**: Config syntax changes, see readme for new syntax but in short
`wm_property` is no longer, and have been replaced by scoped aliases that are
checked in this order:
```toml
[aliases.name] # 1
".*mutt$" = "Mutt"

[aliases.instance] # 2
"open.spotify.com" = "Spotify"

[aliases.class] # 3
"^firefoxdeveloperedition$" = "Firefox-dev"
```

If there are no alias defined, `i3wsr` will default class, but this can be
configured with
```
--display-property=[class|instance|name]`
```
or config file:

```toml
[general]
display_property = "instance" # class, instance, name
```

### Bug Fixes

- Missing instance in class string
- Remove old file from package exclude
- Tests, update connection namespace
- Clean cache on vagrant machine
- Format source files using rustfmt
- Refresh lock file
- License year to current
- Ignore scratch buffer
- Tests
- Handle no alias by adding display_prop conf
- Add display property as a cmd opt

### Documentation

- Fix readme url (after branch rename)
- Update instance explanation
- Update toc
- Document aliases usage
- Update readme and example config
- Fix badge, update toc, fix section placement
- Fix typo

### Features

- Update casing etc for error msg
- Add split_at option
- [**breaking**] Enable wm_property scoped aliases
- Add empty_label option

### Miscellaneous Tasks

- Add test workflow, update scripts
- Update test branch
- Fix job name
- Remove old travis conf
- Remove leftover cmd opt

### Refactor

- Rewrite failure logic
- Remove lazy_static
- Move i3ipc to dependency section
- Remove unneeded extern declarations
- Update clap, rewrite args parsing
- Move cmd arg parsing to new setup fn
- Replace xcb with i3ipc window_properties

### Styling

- Rustfmt

### Testing

- Update ubuntu version
- Fix tests after failure refactor

### Deps

- Update to latest version of xcb
- Update toml (0.7.6)
- Update serde (1.0.171)
- Update itertools (0.11.0)
- Pin regex to 1.9.1
- Pin endoing to 0.2.33

## [v2.1.1] - 2022-03-15

### Bug Fixes

- Use with_context() instead of context()

## [v2.1.0] - 2022-03-14

### Bug Fixes

- Build warnings

### Documentation

- Add examples of workspace assignment
- Document about the default config file


[Unreleased]: https://github.com/roosta/i3wsr/compare/v3.1.2...HEAD
[v3.1.2]: https://github.com/roosta/i3wsr/compare/v3.1.1...v3.1.2
[v3.1.1]: https://github.com/roosta/i3wsr/compare/v3.1.0...v3.1.1
[v3.1.0]: https://github.com/roosta/i3wsr/compare/v3.0.0...v3.1.0
[v3.0.0]: https://github.com/roosta/i3wsr/compare/v2.1.1...v3.0.0
[v2.1.1]: https://github.com/roosta/i3wsr/compare/v2.1.0...v2.1.1
[v2.1.0]: https://github.com/roosta/i3wsr/compare/v2.1.1...v3.0.0

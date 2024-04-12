# Tablet-mode assistance

[![github](https://img.shields.io/badge/github-katyo/tablet--assist-8da0cb.svg?style=for-the-badge&logo=github)](https://github.com/katyo/tablet-assist)
[![MIT](https://img.shields.io/badge/License-MIT-brightgreen.svg?style=for-the-badge)](https://opensource.org/licenses/MIT)
[![CI](https://img.shields.io/github/actions/workflow/status/katyo/tablet-assist/ci.yml?branch=master&style=for-the-badge&logo=github-actions&logoColor=white)](https://github.com/katyo/tablet-assist/actions?query=workflow%3ARust)

This software intended to support convertible and tablet devices with Linux.
Current status still work-in-progress but main features already implemented.

## Packages

- **tablet-assist-service** - system service which interacts with hardware (tablet switch and accelerometer sensors) and provides D-Bus properties to be able detect current device usage
- **tablet-assist-agent** - user session service which configures screen orientation and input devices according to settings and current device usage
- **tablet-assist-ui** - simple GTK-based UI to interact with agent for switching modes and configuring

### Service

Features:

- [x] Support for Libinput tablet-mode switches
- [x] Support for Industrial IO accelerometers
- [ ] Support tablet-mode detection using two accelerometers

System D-Bus service properties:

- `HasTabletMode` - tablet-mode detection supported by service
  - `true` - detection supported
  - `false` - detection not supported
- `TabletMode` - detected mode
  - `true` - currently in tablet mode
  - `false` - currently in laptop mode
- `HasOrientation` - screen orientation detection supported by service
  - `true` - detection supported
  - `false` - detection not supported
- `Orientation` - detected orientation
  - `top-up`
  - `bottom-up`
  - `left-up`
  - `right-up`

### Agent

Features:

- [x] Enabling/disabling input devices when tablet-mode switched
- [x] Changing screen orientation when orientation changed
- [x] Auto and manual tablet-mode switching
- [x] Auto and manual screen orientation changing

### UI

- [x] Tray indicator for quick controls
- [ ] Input devices configuration dialog

## Installation

### Service

```sh
cargo build --release -p tablet-assist-service
cp target/release/tablet-assist-service /usr/sbin
cp data/tablet-assist.service /usr/lib/systemd/system
cp data/tablet.assist.Service.conf /usr/share/dbus-1/system.d
cp data/tablet.assist.Service.service /usr/share/dbus-1/system-services
```

### Agent

```sh
cargo build --release -p tablet-assist-agent
cp target/release/tablet-assist-agent /usr/sbin
cp data/tablet-assist-agent.service /usr/share/systemd/user
cp data/tablet.assist.Agent.service /usr/share/dbus-1/services
```

### UI

```sh
cargo build --release -p tablet-assist-ui
cp target/release/tablet-assist-ui /usr/bin
cp data/tablet-assist.desktop /etc/xdg/autostart
```

## Supported devices

- [x] Lenovo ThinkPad X1
  - [x] Gen4 (tested)
- [ ] Teclast F5

Welcome for issues and PRs with support for other devices!

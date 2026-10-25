# blackshark-battery-tray

Show the battery level of a Razer BlackShark V2 HyperSpeed headset in tray on Linux. No Razer Synapse needed.

## Supported hardware

| Device | USB ID | Status |
|---|---|---|
| BlackShark V2 HyperSpeed, 2.4 GHz dongle | `1532:0565` | Tested |
| BlackShark V2 HyperSpeed, USB-C cable | `1532:056e` | Tested: while charging from the computer, the headset answers over the cable instead |
| BlackShark V2 Pro and other models | | Not supported: they use a different protocol |

`lsusb -d 1532:` shows which one you have.


## Image source convertion

SVG headset image is coming from https://develop.kde.org/frameworks/breeze-icons/ and was converted with [resvg](https://github.com/linebender/resvg):

```
resvg headset.svg headset.png --width 32
```

## Install

Download the static binary for your machine from the latest release and run its installer:

```
chmod +x /tmp/blackshark-battery
/tmp/blackshark-battery install
```

Builds are available for `x86_64` and `aarch64`; each has a `.sha256` file next to it. To build from source instead:

```
cargo build --release --target x86_64-unknown-linux-musl
target/x86_64-unknown-linux-musl/release/blackshark-battery install
```


### What the installer does, and why it needs root

`install` shows what it is about to write and asks for your password once (through `pkexec`, or `sudo` if `pkexec` isn't available). It then, as root:

- copies the binary to `/usr/local/bin/blackshark-battery`;
- writes `/etc/udev/rules.d/60-blackshark-battery.rules`, which allows access to HID devices to read them

The rest of the commands for the service is made to run as a user service.

`blackshark-battery uninstall` stops the service and removes all three files.

## Use

- `blackshark-battery status` prints the current level, e.g. `89%` or `100% charging`.
- `journalctl --user -u blackshark-battery -f` shows level changes and when the headset comes and goes.

The service checks the headset every 5 seconds, and updates the tray icon text accordingly. When the headset is switched off or the dongle is unplugged, the icon changes back to a generic headset icon.

## How it works

1. Every 5 seconds it sends two read commands over the vendor HID interface: battery level (`0x21`) and charging state (`0x2a`). Normally they go to the dongle, which relays them to the headset over the air. While the headset is plugged into the computer by cable it stops answering through the dongle, so the commands go over the cable instead; the device is looked up afresh on every check, so switching back and forth needs nothing from you.

The dongle protocol was reverse-engineered by [justik13/razer-blackshark-v2-hyperspeed-webhid](https://github.com/justik13/razer-blackshark-v2-hyperspeed-webhid). blackshark-battery only ever sends read commands: it doesn't change any headset setting.

## Develop

```
cargo build --release
target/release/blackshark-battery install --yes
target/release/blackshark-battery status
target/release/blackshark-battery run --interval 10
```

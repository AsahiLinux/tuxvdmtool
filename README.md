# Apple Silicon to Apple Silicon VDM tool

This tool lets you get a serial console on an Apple Silicon device and reboot it remotely, using only another Apple Silicon device running Linux and a standard Type C cable.

## Copyright

This is based on [macvdmtool](https://github.com/AsahiLinux/macvdmtool) without replicating portions of [ThunderboltPatcher](https://github.com/osy/ThunderboltPatcher) and licensed under Apache-2.0.

* Copyright (C) 2019 osy86. All rights reserved.
* Copyright (C) 2021 The Asahi Linux Contributors

Thanks to t8012.dev and mrarm for assistance with the VDM and Ace2 host interface commands.

## Building

Install Rust cargo and type `cargo build`.

## Usage

Connect the two devices via their DFU ports. That's:
 - the rear port on MacBook Air and 13" MacBook Pro
 - the port next to the MagSafe connector on the 14" and 16" MacBook Pro
 - the port nearest to the power plug on Mac Mini (M1 and M2)

You need to use a *USB 3.0 compatible* (SuperSpeed) Type C cable. USB 2.0-only cables, including most cables meant for charging, will not work, as they do not have the required pins. Thunderbolt cables work too.

Note that the numbering of the i2c busses is not stable and the default bus `/dev/i2c-0` will be wrong randomly. To find the correct buse use `grep cd321x /sys/class/i2c-dev/i2c-?/device/?-0038/name`.

Run it as root (`sudo ./tuxvdmtool`).

```
USAGE:
    linuxvdmtool [OPTIONS] [SUBCOMMAND]

OPTIONS:
    -a, --address [<ADDRESS>...]    i2c slave address of the USB-C controller device. [default:
                                    0x38]
    -b, --bus [<BUS>...]            i2c bus of the USB-C controller device. [default: /dev/i2c-0]
    -h, --help                    Print help information
    -V, --version                 Print version information

SUBCOMMANDS:
    dfu              put the target into DFU mode
    help             Print this message or the help of the given subcommand(s)
    nop              Do nothing
    reboot           reboot the target
    reboot serial    reboot the target and enter serial mode
    serial           enter serial mode on both ends
```

Use `/dev/ttySAC0` on the local machine as your serial device. To use it with m1n1, `export M1N1DEVICE=/dev/ttySAC0`.

For typical development, the command you want to use is `tuxvdmtool reboot serial`. This will reboot the target, and immediately put it back into serial mode, with the right timing to make it work.

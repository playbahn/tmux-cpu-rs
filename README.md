# tmux-cpu-rs

A small, fast Rust-based CLI tool to display CPU usage inside your `tmux` status line — with **caching** for efficiency, and optional gradients & formatting.

![Intro Picture 1](./assets/images/header-1.png)

![Intro Picture 2](./assets/images/header-2.png)

## Platform Support

**Linux only** - This tool reads CPU statistics from `/proc/stat` and only works on Linux systems.

**Tested and supported architectures:**
- x86_64 (64-bit Intel/AMD)

**Untested architectures:**
- aarch64 (64-bit ARM) 
- armv7 (32-bit ARM)
- i686 (32-bit Intel/AMD)

---

## Features

- ✅ **Accurate CPU usage** using `/proc/stat`
- 🎨 **Color gradients** from green (low) to red (high) CPU usage
- 🖋 **Customizable formatting** with `tmux`-like format strings
- ⚡ **Caching** for minimal system overhead
- 🛠 **Raw output mode** for re-use in scripts
- 🔧 **Configurable precision** for `f64`-based stats

---

## Installation

Using `cargo`:

```console
cargo install tmux-cpu-rs
```

Or clone and build manually:

```console
git clone https://github.com/yourusername/tmux-cpu-rs
cd tmux-cpu-rs
cargo build --release
```

## Usage

```console
Usage: tmux-cpu-rs [OPTIONS] --uid <UID> --pid <PID>

Options:
  -u, --uid <UID>              Pass in #{client_uid}
  -p, --pid <PID>              Pass in #{client_pid}
  -H, --no-hook                Disable setting of cache removal hooks (client-detached, session-closed)
  -P, --precision <PRECISION>  f64 precision to use for displayed stats [default: 0]
  -b, --before <BEFORE>        Tmux format strings (sort of) to place before CPU usage (see below)
  -a, --after <AFTER>          Tmux format strings (sort of) to place after CPU usage (see below)
  -r, --raw                    Get CPU usage and gradient in a "raw" reusable format (see below)
  -c, --cachedir <CACHEDIR>    Directory to cache stats in [default: /tmp/tmcpu/]
  -d, --display <DELAY>        Display output in status line with `tmux display-message` [aliases: --delay]
  -h, --help                   Print help
  -V, --version                Print version

...
```

## Formatting

`--before` and `--after` take `tmux`-like format strings in the form: `--before '#<style>text'`

where:

1. **text** is any valid UTF-8 text (printed as-is)

2. **style** uses `tmux` style syntax (validity is not checked), but with an additional feature:

    Every occurrence of the word **GRADIENT** inside the style string will be replaced by a color value ranging from Hue 120° (green) to Hue 0° (red) depending on CPU usage.

## Example

```tmux
set -g status-right "#(path/to/tmux-cpu-rs --uid #{client_uid} --pid #{client_pid} -P1 -b '#<fg=GRADIENT>') "
```

With a low CPU usage, the above will render as:

![Single precision and foreground gradient](./assets/images/single-precision-fg-gradient.png)

## Raw Output

When using `--raw`, the output format becomes:

```console
USAGE\nGRADIENT
```

where:

- `USAGE` is the numeric CPU usage
- `\n` is the Unicode U+000A newline character
- `GRADIENT` is the computed color (hex string)

This is useful if you want to build your own custom `tmux` formatting or use the values in other scripts.

## Example tmux Integrations

Adding a percent sign after usage:

```tmux
set -g status-right "#(path/to/tmux-cpu-rs --uid #{client_uid} --pid #{client_pid} \
--precision 1 --after '%') "
```

![Percent sign after usage](./assets/images/percent-sign.png)

With gradients turned on:

```tmux
set -g status-right "#(path/to/tmux-cpu-rs --uid #{client_uid} --pid #{client_pid} \
--precision 1 --before '#<fg=GRADIENT>' --after '%') "
```

![With gradient](./assets/images/gradient-on.png)

`tmux-cpu-rs` emits plain `tmux` format strings. So, styles are persistent:

```tmux
set -g status-right "#(path/to/tmux-cpu-rs --uid #{client_uid} --pid #{client_pid} \
--before '#<fg=GRADIENT>' -a%) #[reverse] #H #[noreverse]"
```

![Persistent Styles](./assets/images/persistent-style-no-ple.png)

This will have to be resolved with either:

```tmux
set -g status-right "#(path/to/tmux-cpu-rs --uid #{client_uid} --pid #{client_pid} \
--before '#<fg=GRADIENT>' -a% -a '#<fg=default>') #[reverse] #H #[noreverse]"
```

or

```tmux
set -g status-right "#(path/to/tmux-cpu-rs --uid #{client_uid} --pid #{client_pid} \
--before '#<fg=GRADIENT>' -a%) #[fg=default,reverse] #H #[noreverse]"
```

![Resetting foreground](./assets/images/reset-fg-gradient.png)

`--raw`:

```console
tmux-cpu-rs --uid $(tmux display -p '#{client_uid}') --pid $(tmux display -p '#{client_pid}') --raw
```

![Raw format](./assets/images/raw.png)

Last but not least, with Nerd Font (Powerline) half circles:

```tmux
set -g status-right "#(path/to/tmux-cpu-rs -u #{client_uid} -p #{client_pid} -P1 \
-b '#<fg=GRADIENT>\uE0B6' -b '#<reverse,bold>\uF4BC ' -a '#<noreverse>\uE0B4') \
#[fg=default]\uE0B6#[reverse,bold]#H#[noreverse]\uE0B4"
```

![Nerd Font half circles](./assets/images/nerd-font-half-circles.png)

With Nerd Font (Powerline) right dividers:

```tmux
set -g status-right "#(path/to/tmux-cpu-rs -u #{client_uid} -p #{client_pid} -P2 \
-b '#<fg=GRADIENT>\uE0B2' -b '#<reverse,bold> \uF4BC ' -a ' ' -a '#<noreverse,bg=#14b5ff>\uE0D6')\
#[fg=default,bg=default,reverse,bold] #H "
```

![Nerd Font right dividers](./assets/images/nerd-font-right-dividers.png)

### Note

Unicode escape sequences in the form given above are only rendered as glyphs if they're parsed by `tmux`. In other words, this works at the `tmux` command prompt or `.tmux.conf`:

```tmux
display "A\uE0B2B"
```

But at the (`bash`) shell command prompt or a (`bash`) shell script, it will need to be written as:

```bash
tmux display "A"$'\uE0B2'"B"
```

## ANOTHER CPU usage monitor?

While looking for a CPU usage monitor for my `tmux` statusline, I looked through some options, but was kind of annoyed by the fact that all (couldn't have been more than one or two) of them `sleep` every time `tmux` evaluates `#()`'s. Taking a *noticeable* time to end execution, throwing away current values, I wasn't happy. So, I wrote one in Rust that cached current stats in `/tmp` for the next delta calculation for CPU usage. I looked once more, and found [tmux-plugins/tmux-cpu](https://github.com/tmux-plugins/tmux-cpu). Although `tmux-cpu-rs` is not a rewrite of the CPU portion of `tmux-plugins/tmux-cpu` (they also have a GPU portion), they're quite similar, as they both use caching. Then, I thought "let's make it use-able by people" and here we are. I would say the (deemed) *selling point* of `tmux-cpu-rs` would be caching.

## License

MIT OR Apache-2.0 License


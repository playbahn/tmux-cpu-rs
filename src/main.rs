use std::io::{BufRead, Read, Seek, Write};
use std::path::PathBuf;

use clap::{ArgAction, Parser};

use palette::{Hsl, IntoColor, Srgb};

const AFTER_HELP: &str = color_print::cstr!(
"<bold,underline>Formatting:</bold,underline>
<bold>--before</> and <bold>--after</> take tmux-like format strings: <bold>#<<<u>style</>>><u>text</></> where
<bold>i.</>  <u>text</> is any valid UTF8 text that is printed without any processing, and
<bold>ii.</> <u>style</> takes tmux style strings (validity unchecked), with the additional feature that
    every match for the exact pattern <bold>HEXGRAD</> inside it is replaced by a color from
    Hue 120 degrees (green) to Hue 0 degrees (red) (going from low to high usage)
    e.g. <bold>#<<fg=HEXGRAD>>abc</> will be replaced by <bold>#[fg=#00ff00]abc</> for a 0%-ish CPU usage
Any empty <bold>#<<>></>'s at the start are trimmed.

<bold>--raw</> prints CPU usage and the gradient in the format: <bold><u>USAGE</>\\n<u>HEXGRAD</></> where <bold>\\n</> is U+000A (newline)

Furthermore, <bold>--before</> and <bold>--after</> conflict with <bold>--raw</>"
);

#[derive(Parser, Debug)]
#[command(version, long_about = None, after_help = AFTER_HELP)]
struct Config {
    /// Pass in #{client_pid}
    pid: String,

    /// Disable setting of cache removal hooks (client-detached, session-closed)
    #[arg(short = 'H', long = "no-hook", action = ArgAction::SetFalse)]
    hook: bool,

    /// f64 precision to use for displayed stats
    #[arg(short = 'P', long, default_value_t = 0)]
    precision: usize,

    /// Tmux format strings (sort of) to place before CPU usage (see below)
    #[arg(short, long)]
    before: Vec<String>,

    /// Tmux format strings (sort of) to place after CPU usage (see below)
    #[arg(short, long)]
    after: Vec<String>,

    /// Get CPU usage and gradient in a "raw" reusable format (see below)
    #[arg(short, long, conflicts_with_all = ["before", "after"])]
    raw: bool,

    /// Directory to cache stats in
    #[arg(short, long, default_value = "/tmp/tmcpu/")]
    cachedir: PathBuf,

    #[cfg(debug_assertions)]
    /// Display output in status line with `tmux display-message`
    #[arg(short, long, value_name = "DELAY", visible_alias = "delay")]
    display: Option<u16>,
}

fn main() {
    let config = Config::parse();

    #[cfg(debug_assertions)]
    dbg!(&config);

    if let Err(e) = std::fs::create_dir(&config.cachedir)
        && e.kind() != std::io::ErrorKind::AlreadyExists
    {
        panic!("Could not create {}: {e:#?}", &config.cachedir.display())
    }

    let cache = config.cachedir.join(config.pid);
    let mut options = std::fs::File::options();

    let mut file = match options.read(true).write(true).truncate(false).open(&cache) {
        Ok(file) => file,
        Err(e) if e.kind() != std::io::ErrorKind::NotFound => {
            panic!("Could not open {}: {e:#?}", cache.display())
        }
        Err(_) => {
            if config.hook {
                let command = format!("run-shell 'rm {} 2> /dev/null'", cache.display());
                let output = std::process::Command::new("tmux")
                    .args(["set-hook", "-ga", "client-detached", &command, ";"])
                    .args(["set-hook", "-ga", "session-closed", &command])
                    .output();
                match output {
                    Err(e) => eprintln!("Could not run `tmux set-hook`'s: {e:#?}"),
                    Ok(output) => {
                        if !output.status.success() {
                            let stderr = String::from_utf8(output.stderr)
                                .expect("tmux produced illegal UTF8 on its stderr");
                            eprintln!(
                                "Could not create {} removal hooks. tmux stderr:\n{stderr}",
                                cache.display()
                            );
                        }
                    }
                }
            }

            options
                .create(true)
                .open(&cache)
                .unwrap_or_else(|e| panic!("could not create {}: {e:#?}", cache.display()))
        }
    };

    let mut line1 = String::new();
    let (mut new_nonidle, mut new_total) = (0, 0);

    std::io::BufReader::new(
        std::fs::File::open("/proc/stat").expect("Your system does not have a /proc/stat"),
    )
    .read_line(&mut line1)
    .unwrap_or_else(|e| panic!("Error reading from /proc/stat: {e:#?}"));
    let mut stats = line1
        .split_whitespace()
        .skip(1) // cpu
        .map(|time| {
            time.parse::<u64>()
                .expect("Did /proc/stat change its format?")
        });

    // user + nice + system
    new_nonidle += stats.next().expect("Did /proc/stat change its format?")
        + stats.next().expect("Did /proc/stat change its format?")
        + stats.next().expect("Did /proc/stat change its format?");
    // idle + iowait
    new_total += stats.next().expect("Did /proc/stat change its format?")
        + stats.next().expect("Did /proc/stat change its format?");
    // irq + softirq + steal
    new_nonidle += stats.next().expect("Did /proc/stat change its format?")
        + stats.next().expect("Did /proc/stat change its format?")
        + stats.next().expect("Did /proc/stat change its format?");
    // guest and guest nice are already accounted for in user and nice respectively
    // https://github.com/htop-dev/htop/blob/01a3c9e04668ecebba31972fd351ba818e19f9e2/linux/LinuxMachine.c#L444
    new_total += new_nonidle;

    let mut old_stats = String::new();
    if let Err(e) = file.read_to_string(&mut old_stats) {
        eprintln!("Error reading from {}: {e:#?}", cache.display());
        old_stats.clear();
    }

    let (old_nonidle, old_total) = old_stats.split_once('\n').unwrap_or_default();
    let mut out = String::new();

    match (old_nonidle.parse::<u64>(), old_total.parse::<u64>()) {
        (Err(e1), Err(e2))
            if *e1.kind() == std::num::IntErrorKind::Empty
                && *e2.kind() == std::num::IntErrorKind::Empty =>
        {
            eprintln!("{} is empty; no baseline.", cache.display())
        }
        (Ok(old_nonidle), Ok(old_total)) => {
            // LOL
            let total_d = new_total.wrapping_sub(old_total);
            let nonidle_d = new_nonidle.wrapping_sub(old_nonidle);

            let normalized_usage = nonidle_d as f64 / total_d as f64;

            let mut gradient = String::new();

            let calc_gradient = || {
                let hue = 120.0 - (120.0 * normalized_usage);
                let hsl: Hsl<_, f64> = Hsl::new(hue, 1.0, 0.5);
                let rgb: Srgb<f64> = hsl.into_color();
                let (r, g, b) = rgb.into_format::<u8>().into_components();
                format!("#{r:02x}{g:02x}{b:02x}")
            };

            out = if config.raw {
                format!(
                    "{:.*}\n{}",
                    config.precision,
                    normalized_usage * 100.0,
                    calc_gradient()
                )
            } else {
                let mut fold_affixes = |out: String, cur: &String| {
                    if cur.starts_with("#<") {
                        match cur.find('>') {
                            None => out + cur.as_str(),
                            Some(2) => out + &cur[3..],
                            Some(end) => {
                                // Why HEXGRAD and not GRADIENT, or COLOR etc? Cause `HEXGRAD` and
                                // `#RRGGBB` are the same length, so every replace of `HEXGRAD`
                                // with `#RRGGBB` doesn't change the position of every subseq
                                // `HEXGRAD` and also the trailing `<` for the style string `#<>`.
                                // So in gist, less calculations.
                                if cur[2..end].contains("HEXGRAD") && gradient.is_empty() {
                                    gradient = calc_gradient();
                                }

                                let mut cur =
                                    cur[..end].replace("HEXGRAD", &gradient) + &cur[end..];

                                cur.replace_range(1..2, "[");
                                cur.replace_range(end..end + 1, "]");

                                out + cur.as_str()
                            }
                        }
                    } else {
                        out + cur.as_str()
                    }
                };

                let prefix = config.before.iter().fold(String::new(), &mut fold_affixes);
                let suffix = config.after.iter().fold(String::new(), &mut fold_affixes);

                format!(
                    "{prefix}{:.*}{suffix}",
                    config.precision,
                    normalized_usage * 100.0
                )
            };
        }
        (r1, r2) => eprintln!("Error parsing {}:\n{r1:#?}\n{r2:#?}", cache.display()),
    }

    // So we dont have to call file.set_len
    const PADDING: usize = u64::BITS as usize;

    file.rewind()
        .unwrap_or_else(|e| panic!("Could not rewind cursor on {}: {e:#?}", cache.display()));
    let new_stats = format!("{new_nonidle:0PADDING$}\n{new_total:0PADDING$}");
    file.write_all(new_stats.as_bytes())
        .unwrap_or_else(|e| panic!("Could not write stats to {}: {e:#?}", cache.display()));

    print!("{out}");

    #[cfg(debug_assertions)]
    {
        println!();
        if let Some(delay) = config.display {
            let _ = dbg!(
                std::process::Command::new("tmux")
                    .args(["display", "-d", &format!("{delay}"), &out])
                    .spawn()
            );
        }
    }
}

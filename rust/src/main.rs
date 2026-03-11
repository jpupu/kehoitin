use std::io::{self, Read};

unsafe extern "C" {
    pub fn gethostname(name: *mut libc::c_char, size: libc::size_t) -> libc::c_int;
}

fn safe_gethostname() -> String {
    let mut buf: [libc::c_char; 100] = [0; 100];
    let success = unsafe { gethostname(&mut buf[0], 99) };
    if success != 0 {
        String::from("error")
    } else {
        unsafe { std::ffi::CStr::from_ptr(&buf[0]) }
            .to_string_lossy()
            .to_string()
    }
}

trait QuoteSplitter {
    fn split_quoted(&self) -> Vec<&str>;
}

impl QuoteSplitter for str {
    fn split_quoted(&self) -> Vec<&str> {
        let mut result = Vec::<&str>::new();

        enum State {
            Word,
            Space,
            Quote,
        }
        let mut state = State::Space;
        let mut word_start = 0;
        for (i, ch) in self.char_indices() {
            match state {
                State::Word => {
                    if ch.is_ascii_whitespace() {
                        result.push(&self[word_start..i]);
                        state = State::Space;
                    }
                }
                State::Space => {
                    if ch == '"' {
                        word_start = i + ch.len_utf8();
                        state = State::Quote;
                    } else if !ch.is_ascii_whitespace() {
                        word_start = i;
                        state = State::Word;
                    }
                }
                State::Quote => {
                    if ch == '"' {
                        result.push(&self[word_start..i]);
                        state = State::Space;
                    }
                }
            }
        }
        match state {
            State::Word | State::Quote => {
                result.push(&self[word_start..]);
            }
            State::Space => {}
        }

        result
    }
}

fn git_branch() -> Option<String> {
    let root = std::env::current_dir().ok()?;

    let gitdir = root.ancestors().find_map(|p| {
        let p = p.join(".git");
        if p.exists() {
            Some(p)
        } else {
            None
        }
    })?;

    let postfix = match () {
        _ if gitdir.join("rebase-merge").exists() => ":REBASE",
        _ if gitdir.join("rebase-apply").exists() => ":REBASE",
        _ if gitdir.join("MERGE_HEAD").exists() => ":MERGING",
        _ if gitdir.join("BISECT_LOG").exists() => ":BISECTING",
        _ => "",
    };

    let branch: String = match std::fs::read_to_string(gitdir.join("HEAD")) {
        Ok(x) if x.starts_with("ref: refs/heads/") => x[16..].trim_end().into(),
        Ok(x) if x.starts_with("ref:") => x[..].trim_end().into(),
        Ok(x) => x[..7].into(),
        Err(_) => None?,
    };

    Some(branch + postfix)
}

struct PromptBuilder {
    fg: String,
    bg: String,
    result: String,
}

impl PromptBuilder {
    fn new() -> Self {
        Self {
            fg: String::from("clear"),
            bg: String::from("clear"),
            result: String::new(),
        }
    }

    fn color(&mut self, fg: &str, bg: &str) {
        if fg != self.fg {
            self.fg = fg.into();
            if fg == "clear" {
                self.result.push_str("%f");
            } else {
                self.result.push_str(&format!("%F{{{}}}", fg));
            }
        }
        if bg != self.bg {
            self.bg = bg.into();
            if bg == "clear" {
                self.result.push_str("%k");
            } else {
                self.result.push_str(&format!("%K{{{}}}", bg));
            }
        }
    }

    fn push(&mut self, text: &str) {
        self.result.push_str(text);
    }
}

const SEPMAP: [(&str, &str, bool); 8] = [
    ("angle", "\u{e0b0}", false),
    ("rev-angle", "\u{e0b2}", true),
    ("round", "\u{e0b4}", false),
    ("rev-round", "\u{e0b6}", true),
    ("downslope", "\u{e0b8}", false),
    ("rev-downslope", "\u{e0ba}", true),
    ("slope", "\u{e0bc}", false),
    ("rev-slope", "\u{e0be}", true),
];

fn compile() {
    let mut result = PromptBuilder::new();

    for line in io::stdin().lines() {
        let linebuf = line.unwrap();
        let words = linebuf.split_quoted();
        let command = words[0];

        match command {
            "text" => result.push(words[1]),
            "color" => {
                let fg = words[1];
                let bg = words[2];
                let sep = words.get(3);
                if let Some(sep) = sep {
                    for (key, text, reverse) in &SEPMAP {
                        if sep == key {
                            if *reverse {
                                result.color(bg, &result.bg.clone());
                            } else {
                                result.color(&result.bg.clone(), bg);
                            }
                            result.push(text);
                            break;
                        }
                    }
                }
                result.color(fg, bg);
            }
            "func" => {
                result.push(&format!(":kcmd:({})", words[1]));
            }
            _ => result.push(&format!("[BAD COMMAND {}]", command)),
        }
    }

    print!("{}", result.result);
}

fn execute(buf: &str) {
    let mut out = String::new();

    let mut prev = 0;
    for (i, _) in buf.match_indices(":kcmd:(") {
        if let Some(k) = buf[(i + 7)..].find(')') {
            let func = &buf[(i + 7)..(i + 7 + k)];

            let funcout = match func {
                "cwd" => std::env::current_dir()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .into(),
                "host" => safe_gethostname(),
                "git-branch" => git_branch().unwrap_or_default(),
                _ => format!("(invalid function {})", func),
            };
            out.push_str(&funcout);
            prev += 8 + k;
        } else {
            out.push_str("(UNENDING_FUNCTION)");
        }
    }
    out.push_str(&buf[prev..]);

    print!("{}", out);
}

fn main() {
    match std::env::args().nth(1).as_deref() {
        Some("compile") => compile(),
        Some("execute") => {
            let mut buf = String::new();
            _ = io::stdin().read_to_string(&mut buf).unwrap();
            execute(&buf)
        }
        x => {
            eprintln!("usage: kehoitin compile|execute");
            eprintln!("got [{:?}]", x);
            std::process::exit(1);
        }
    }
}

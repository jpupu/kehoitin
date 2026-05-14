use std::{env, fs};

const EVAL_TEXT: &str = "
kehoitin_precmd() {
    export KEHOITIN_LAST_STATUS=$?
}
if (( !$precmd_functions[(Ie)kehoitin_precmd] )); then
    precmd_functions+=( kehoitin_precmd )
fi
PROMPT='$(_BINPATH_ prompt _INFILE_)'
";

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

enum Fragment {
    SetFg(String),
    SetBg(String),
    SetFgFromBg,
    AddText(String),
}

struct Segment(Vec<Fragment>);

impl Segment {
    fn new() -> Self {
        Self(Vec::new())
    }

    fn set_fg(&mut self, fg: &str) {
        self.0.push(Fragment::SetFg(fg.to_owned()));
    }

    fn set_bg(&mut self, bg: &str) {
        self.0.push(Fragment::SetBg(bg.to_owned()));
    }

    fn set_fg_from_bg(&mut self) {
        self.0.push(Fragment::SetFgFromBg);
    }

    fn push(&mut self, text: &str) {
        self.0.push(Fragment::AddText(text.to_owned()))
    }

    fn append(&mut self, other: &mut Segment) {
        self.0.append(&mut other.0);
    }

    fn render(self) -> String {
        let mut fg = String::from("clear");
        let mut bg = String::from("clear");
        let mut out = String::new();

        for fragment in self.0 {
            match fragment {
                Fragment::SetFg(new_fg) => {
                    if *new_fg != fg {
                        fg = new_fg;
                        if fg == "clear" {
                            out.push_str("%f");
                        } else {
                            out.push_str(&format!("%F{{{}}}", fg));
                        }
                    }
                }
                Fragment::SetBg(new_bg) => {
                    if *new_bg != bg {
                        bg = new_bg;
                        if bg == "clear" {
                            out.push_str("%k");
                        } else {
                            out.push_str(&format!("%K{{{}}}", bg));
                        }
                    }
                }
                Fragment::SetFgFromBg => {
                    let new_fg = bg.clone();
                    if new_fg != fg {
                        fg = new_fg;
                        if fg == "clear" {
                            out.push_str("%f");
                        } else {
                            out.push_str(&format!("%F{{{}}}", fg));
                        }
                    }
                }
                Fragment::AddText(text) => out.push_str(&text),
            }
        }
        out
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

fn eval_condition(cond: &str) -> Option<bool> {
    match cond {
        x if x.starts_with("!") => eval_condition(&cond[1..]).map(|c| !c),
        "true" => Some(true),
        "false" => Some(false),
        "git" => Some(git_branch().is_some()),
        "success" => Some(std::env::var("KEHOITIN_LAST_STATUS").map_or(true, |s| s == "0")),
        _ => None,
    }
}

fn read_block(mut lines: &mut dyn Iterator<Item = &str>) -> Segment {
    let mut out = Segment::new();
    while let Some(linebuf) = lines.next() {
        let words = linebuf.split_quoted();
        let command = words[0];

        match command {
            "text" => out.push(words[1]),
            "color" => {
                let fg = words[1];
                let bg = words[2];
                let sep = words.get(3);
                if let Some(sep) = sep {
                    for (key, text, reverse) in &SEPMAP {
                        if sep == key {
                            if *reverse {
                                out.set_fg(bg);
                            } else {
                                out.set_fg_from_bg();
                                out.set_bg(bg);
                            }
                            out.push(text);
                            break;
                        }
                    }
                }
                out.set_fg(fg);
                out.set_bg(bg);
            }
            "func" => {
                let func = words[1];
                let funcout = match func {
                    "cwd" => std::env::current_dir()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .into(),
                    "host" => safe_gethostname(),
                    "git-branch" => git_branch().unwrap_or_default(),
                    _ => format!("(invalid function {})", func),
                };
                out.push(&funcout);
            }
            "if" => {
                let cond = words[1];
                let mut body = read_block(&mut lines);
                match eval_condition(cond) {
                    Some(true) => out.append(&mut body),
                    Some(false) => (),
                    None => out.push(&format!("(invalid condition {})", cond)),
                }
            }
            "end" => break,
            _ => out.push("(bad command)"),
        }
    }
    out
}

fn interpret(infile: &str) -> String {
    match fs::read_to_string(infile) {
        Ok(input) => read_block(&mut input.lines()).render(),
        Err(_) => format!("(failed to read file '{}')", infile),
    }
}

fn main() {
    match env::args().nth(1).as_deref() {
        Some("prompt") => print!(
            "{}",
            interpret(
                env::args()
                    .nth(2)
                    .expect("Must provide input filepath")
                    .as_ref()
            )
        ),
        Some("eval") => print!(
            "{}",
            EVAL_TEXT
                .replace("_BINPATH_", &env::current_exe().unwrap().to_string_lossy())
                .replace(
                    "_INFILE_",
                    &fs::canonicalize(env::args().nth(2).expect("Must provide input filepath"))
                        .unwrap()
                        .to_string_lossy()
                )
        ),
        x => {
            eprintln!("usage: kehoitin prompt|eval <file>");
            eprintln!("got [{:?}]", x);
            std::process::exit(1);
        }
    }
}

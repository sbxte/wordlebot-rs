use std::collections::HashMap;
use std::io::Read;

use clap::{Command, command};

const WORD_LENGTH: usize = 5;

#[derive(Debug, Clone)]
enum CharInfo {
    Is(u8),
    Not(Vec<u8>),
}

#[derive(Debug, Clone)]
struct MatchState {
    chars: [CharInfo; WORD_LENGTH],
    nowhere: Vec<u8>,
    somewhere: Vec<u8>,
}

impl MatchState {
    fn empty() -> Self {
        Self {
            chars: [const { CharInfo::Not(vec![]) }; WORD_LENGTH],
            nowhere: vec![],
            somewhere: vec![],
        }
    }

    fn matches(&self, word: Word) -> bool {
        word.0
            .iter()
            .enumerate()
            .all(|(i, b)| match &self.chars[i] {
                CharInfo::Is(is) => *is == *b,
                CharInfo::Not(not) => {
                    not.iter().all(|n| *n != *b)
                }
            } && !self.nowhere.contains(b))
            && self.somewhere.iter().all(|b| word.0.contains(b))
    }

    fn serialize(&self) -> String {
        let mut buffer = String::new();

        // Nowhere
        buffer.push('{');
        if !self.nowhere.is_empty() {
            buffer.push(self.nowhere[0] as char);
            for b in &self.nowhere[1..] {
                buffer.push(' ');
                buffer.push(*b as char);
            }
        }
        buffer.push('}');

        // Somewhere
        buffer.push('[');
        if !self.somewhere.is_empty() {
            buffer.push(self.somewhere[0] as char);
            for b in &self.somewhere[1..] {
                buffer.push(' ');
                buffer.push(*b as char);
            }
        }
        buffer.push(']');

        // Chars
        buffer.push('(');
        match &self.chars[0] {
            CharInfo::Is(is) => buffer.push(*is as char),
            CharInfo::Not(not) => {
                buffer.push('[');
                if !not.is_empty() {
                    buffer.push(not[0] as char);
                    for b in &not[1..] {
                        buffer.push(',');
                        buffer.push(*b as char);
                    }
                }
                buffer.push(']');
            }
        }
        for c in &self.chars[1..] {
            buffer.push(' ');
            match c {
                CharInfo::Is(is) => buffer.push(*is as char),
                CharInfo::Not(not) => {
                    buffer.push('[');
                    if !not.is_empty() {
                        buffer.push(not[0] as char);
                        for b in &not[1..] {
                            buffer.push(',');
                            buffer.push(*b as char);
                        }
                    }
                    buffer.push(']');
                }
            }
        }
        buffer.push(')');

        buffer
    }

    fn deserialize(input: &str) -> Self {
        if input.is_empty() {
            return Self::empty();
        }

        let (nowhere, input) = input
            .trim_start_matches('{')
            .split_once("}[")
            .expect("No }[ found");
        let (somewhere, input) = input.split_once("](").expect("No ]( found");
        let chars = input.trim_end_matches(')');

        let nowhere = {
            if nowhere.is_empty() {
                vec![]
            } else {
                nowhere.split(' ').map(|s| s.as_bytes()[0]).collect()
            }
        };
        let somewhere = {
            if somewhere.is_empty() {
                vec![]
            } else {
                somewhere.split(' ').map(|s| s.as_bytes()[0]).collect()
            }
        };
        let chars = {
            let mut ch = [const { CharInfo::Not(vec![]) }; WORD_LENGTH];
            for (i, c) in chars.split(' ').enumerate() {
                ch[i] = if c.starts_with('[') {
                    CharInfo::Not({
                        let trimmed = c.trim_start_matches('[').trim_end_matches(']');
                        if trimmed.is_empty() {
                            vec![]
                        } else {
                            trimmed.split(',').map(|s| s.as_bytes()[0]).collect()
                        }
                    })
                } else {
                    CharInfo::Is(c.as_bytes()[0])
                }
            }
            ch
        };

        Self {
            nowhere,
            somewhere,
            chars,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
struct Word([u8; WORD_LENGTH]);

impl std::fmt::Display for Word {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}{}{}{}{}",
            self.0[0] as char,
            self.0[1] as char,
            self.0[2] as char,
            self.0[3] as char,
            self.0[4] as char
        )
    }
}

#[derive(PartialEq, Eq, Hash, Debug)]
enum CharMatch {
    Green,
    Yellow,
    Gray,
}

#[derive(Hash, Eq, PartialEq, Debug)]
struct WordMatch {
    cm: [CharMatch; WORD_LENGTH],
    word: Word,
}

impl WordMatch {
    fn is_win(&self) -> bool {
        self.cm.iter().all(|cm| matches!(cm, CharMatch::Green))
    }

    fn matches(&self, target: Word) -> bool {
        // God, this has been the most mind-melding thing this morning
        self.cm.iter().enumerate().all(|(i, cm)| match cm {
            CharMatch::Green => self.word.0[i] == target.0[i],
            CharMatch::Gray => target.0.iter().all(|tc| self.word.0[i] != *tc),
            CharMatch::Yellow => target
                .0
                .iter()
                .enumerate()
                .any(|(j, tc)| i != j && self.word.0[i] == *tc),
        })
    }

    fn deserialize(input: &str) -> Self {
        let input = input.trim();
        let mut new = Self {
            cm: [const { CharMatch::Gray }; WORD_LENGTH],
            word: Word([0; WORD_LENGTH]),
        };
        for (i, ch) in input.trim().split(' ').enumerate() {
            let c = ch[1..][..ch.len() - 1].trim().as_bytes()[0];
            new.word.0[i] = c;

            new.cm[i] = if ch.starts_with('[') {
                CharMatch::Yellow
            } else if ch.starts_with('{') {
                CharMatch::Gray
            } else {
                CharMatch::Green
            };
        }
        new
    }
}

struct MatchComb<'a, 'b> {
    state: &'a MatchState,
    wm: &'b WordMatch,
}

impl<'a, 'b> MatchComb<'a, 'b> {
    fn matches(&self, target: Word) -> bool {
        self.state.matches(target) && self.wm.matches(target)
    }

    fn merge(&self) -> MatchState {
        let mut state = self.state.to_owned();
        for (i, cm) in self.wm.cm.iter().enumerate() {
            let ch = self.wm.word.0[i];
            match cm {
                CharMatch::Green => {
                    state.chars[i] = CharInfo::Is(ch);
                    state.somewhere.retain(|n| *n != ch);

                    // I think this should be a noop but who knows
                    state.nowhere.retain(|n| *n != ch);
                }
                CharMatch::Yellow => {
                    if let CharInfo::Not(n) = &mut state.chars[i] {
                        n.push(ch);
                    }
                    // TODO: What if a single character appears multiple times in the WordMatch?
                    // So far we've only accounted for a character appearing once
                    if !state.somewhere.contains(&ch) {
                        state.somewhere.push(ch);
                    }
                }
                CharMatch::Gray => {
                    if (state.somewhere.contains(&ch)
                        || state.chars.iter().any(|c| {
                            if let CharInfo::Is(c) = c {
                                *c == ch
                            } else {
                                false
                            }
                        }))
                        && let CharInfo::Not(n) = &mut state.chars[i]
                    {
                        n.push(ch);
                    } else if !state.nowhere.contains(&ch) {
                        state.nowhere.push(ch);
                    }
                }
            }
        }

        state
    }
}

fn parse_words(input: &str) -> Vec<Word> {
    let words: Vec<_> = input
        .lines()
        .map(|s| {
            let bytes = s.as_bytes();
            Word([bytes[0], bytes[1], bytes[2], bytes[3], bytes[4]])
        })
        .collect();
    words
}

enum ScoreResult {
    Sorted(Vec<(Word, f64)>),
    Win(Word),
}

fn sort_scores(state: &MatchState, search: &[Word], words: &[Word]) -> ScoreResult {
    let mut matches = HashMap::new();
    let mut scores = Vec::with_capacity(words.len());

    let all = words.iter().filter(|w| state.matches(**w)).count();

    let mut prev_prog = 0.;
    for (wi, word) in search.iter().enumerate() {
        let progress = ((wi as f64) / (search.len() as f64) * 100.).floor();
        if progress >= prev_prog + 5. {
            eprintln!(
                "Thread#{} {progress}% searched {wi}",
                std::thread::current().name().unwrap()
            );
            prev_prog = progress;
        }

        matches.clear();
        for target in words.iter().filter(|w| state.matches(**w)) {
            let result = word_match(*word, *target);
            use std::collections::hash_map::Entry;
            match matches.entry(result) {
                Entry::Vacant(e) => {
                    e.insert(1);
                }
                Entry::Occupied(mut e) => {
                    *e.get_mut() += 1;
                }
            }
        }

        let mut total_expected_info = 0.;
        for (k, v) in &matches {
            let mc = MatchComb { state, wm: k };
            let remaining = words.iter().filter(|w| mc.matches(**w)).count();
            if remaining == 0 {
                continue;
            }
            if *v == all && k.is_win() {
                return ScoreResult::Win(k.word);
            }

            let bits = -((remaining as f64) / (all as f64)).log2();
            let probability = (*v as f64) / (all as f64);

            total_expected_info = bits.mul_add(probability, total_expected_info);
        }

        scores.push((*word, total_expected_info));
    }

    scores.sort_unstable_by(|(_, e1), (_, e2)| e1.total_cmp(e2).reverse());
    ScoreResult::Sorted(scores)
}

fn word_match(word: Word, target: Word) -> WordMatch {
    let mut result = [const { CharMatch::Gray }; WORD_LENGTH];
    for (i, wc) in word.0.iter().enumerate() {
        result[i] = if *wc == target.0[i] {
            CharMatch::Green
        } else if target.0.contains(wc) {
            CharMatch::Yellow
        } else {
            CharMatch::Gray
        };
    }
    WordMatch { cm: result, word }
}

fn handle_calc(state: &MatchState) {
    let threads: usize = std::thread::available_parallelism()
        .map(|x| x.get())
        .unwrap_or(1);

    let words = parse_words(include_str!("words.txt"));
    if words.is_empty() {
        panic!("No word dictionary found!");
    }

    let valid = {
        let v = parse_words(include_str!("valid.txt"));
        if v.is_empty() { words.clone() } else { v }
    };

    let len = words.len();
    let n = words.len() / threads;

    let rem = valid.iter().filter(|w| state.matches(**w)).count();
    println!("{rem} remaining words to search");

    let mut scores = Vec::with_capacity(len);
    let mut win = None;

    std::thread::scope(|s| {
        let words = &words[..];
        let valid = &valid[..];
        let state = &state;
        let mut handles = Vec::with_capacity(threads);

        for i in 0..threads - 1 {
            let builder = std::thread::Builder::new().name(format!("{}", i + 1));
            handles.push(
                builder
                    .spawn_scoped(s, move || {
                        sort_scores(state, &words[n * i..n * (i + 1)], valid)
                    })
                    .unwrap(),
            );
        }
        handles.push(
            std::thread::Builder::new()
                .name(format!("{threads}"))
                .spawn_scoped(s, move || {
                    sort_scores(state, &words[n * (threads - 1)..], valid)
                })
                .unwrap(),
        );

        for thread in handles {
            if let Ok(x) = thread.join() {
                match x {
                    ScoreResult::Sorted(s) => scores.extend(s),
                    ScoreResult::Win(w) => {
                        win = Some(w);
                    }
                }
            }
        }
    });
    if let Some(w) = win {
        println!("Winning word found: {w}");
        return;
    }

    scores.sort_unstable_by(|(_, e1), (_, e2)| e1.total_cmp(e2).reverse());

    println!();
    println!("Displaying top 25 options");
    for (i, (word, score)) in scores.iter().enumerate().take(25) {
        println!("{}. {score} {word}", i + 1);
    }

    if rem <= 10 {
        println!("{rem} possible answers remaining");
        for (i, word) in valid.iter().filter(|w| state.matches(**w)).enumerate() {
            println!("{}. {word}", i + 1);
        }
    }
}

fn handle_merge() {
    let mut buffer = String::new();
    std::io::stdin().read_to_string(&mut buffer).unwrap();
    let (state, wm) = buffer.split_once(';').unwrap();
    let (state, wm) = (
        MatchState::deserialize(state.trim()),
        WordMatch::deserialize(wm.trim()),
    );
    let new_state = MatchComb {
        state: &state,
        wm: &wm,
    }
    .merge();
    println!("{}", new_state.serialize());
}

fn handle_run() {
    let mut buffer = String::new();
    let mut state = MatchState::empty();
    loop {
        println!();
        println!("(a) Add info");
        println!("(v) View state");
        println!("(c) Calc");
        println!("(r) Reset");
        println!("(q) Quit");
        println!("Enter input:");
        buffer.clear();
        std::io::stdin().read_line(&mut buffer).unwrap();
        let choice = if let Ok(x) = buffer.trim().parse::<char>() {
            x
        } else {
            continue;
        };

        match choice {
            'a' => {
                println!("Enter new info: ");
                buffer.clear();
                std::io::stdin().read_line(&mut buffer).unwrap();
                let wm = WordMatch::deserialize(&buffer);
                state = MatchComb {
                    state: &state,
                    wm: &wm,
                }
                .merge();
            }
            'v' => {
                println!("Current state: {}", state.serialize());
            }
            'c' => {
                handle_calc(&state);
            }
            'r' => {
                state = MatchState::empty();
                println!("State reset!");
            }
            _ => break,
        }
    }
}

fn main() {
    let matches = command!()
        .subcommand(Command::new("merge"))
        .subcommand(Command::new("calc"))
        .subcommand(Command::new("run"))
        .get_matches();

    match matches.subcommand() {
        Some(("merge", _subm)) => handle_merge(),
        Some(("calc", _subm)) => {
            let mut buffer = String::new();
            std::io::stdin().read_to_string(&mut buffer).unwrap();
            let state = MatchState::deserialize(&buffer);
            handle_calc(&state)
        }
        Some(("run", _subm)) => handle_run(),
        _ => {}
    }
}

use std::collections::HashMap;
use std::fs::OpenOptions;
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

    #[cfg(feature = "dbg_progress")]
    let mut prev_prog = 0.;
    for (wi, word) in search.iter().enumerate() {
        #[cfg(feature = "dbg_progress")]
        {
            let progress = ((wi as f64) / (search.len() as f64) * 100.).floor();
            if progress >= prev_prog + 25. {
                eprintln!(
                    "Thread#{} {progress}% searched {wi}",
                    std::thread::current().name().unwrap()
                );
                prev_prog = progress;
            }
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

fn default_search(state: &MatchState, valid_words: &[Word], answer_words: &[Word]) -> SearchResult {
    let threads: usize = std::thread::available_parallelism()
        .map(|x| x.get())
        .unwrap_or(1);

    let words = valid_words;
    let valid = if answer_words.is_empty() {
        valid_words
    } else {
        answer_words
    };

    let SearchResult {
        mut scores,
        mut win,
        mut words_remaining,
    } = search(&words, &valid, threads, state);
    if words_remaining == 0 {
        println!("No answer found in valid.txt, falling back to words.txt");
        SearchResult {
            scores,
            win,
            words_remaining,
        } = search(&words, &words, threads, state);
    }

    scores.sort_unstable_by(|(_, e1), (_, e2)| e1.total_cmp(e2).reverse());

    SearchResult {
        scores,
        win,
        words_remaining,
    }
}

fn handle_calc(state: &MatchState, valid_words: &[Word], answer_words: &[Word]) -> SearchResult {
    let SearchResult {
        scores,
        win,
        words_remaining,
    } = default_search(state, valid_words, answer_words);

    if let Some(w) = win {
        println!("Winning word found: {w}");
        return SearchResult {
            scores,
            win,
            words_remaining,
        };
    }

    println!();
    println!("Displaying top 25 options");
    for (i, (word, score)) in scores.iter().enumerate().take(25) {
        println!("{}. {score} {word}", i + 1);
    }

    println!("{words_remaining} possible answers remaining");
    if words_remaining <= 20 {
        for (i, (w, _)) in scores.iter().filter(|(w, _)| state.matches(*w)).enumerate() {
            println!("{}. {}", i + 1, w);
        }
    }

    SearchResult {
        scores,
        win,
        words_remaining,
    }
}

fn handle_calc_word(search_result: &SearchResult, word: Word) {
    let SearchResult {
        scores,
        win: _,
        words_remaining,
    } = search_result;

    let (ranking, (_, score)) =
        if let Some(s) = scores.iter().enumerate().find(|(_idx, e)| e.0 == word) {
            s
        } else {
            println!("{} is an invalid word!", word);
            return;
        };
    let score = *score;

    println!();
    println!("Stats for \"{}\"", word);
    println!("Score: {}", score);
    println!("Rank: {}/{}", ranking + 1, scores.len());
    println!(
        "Avg # of possible words eliminated: {}",
        words_remaining - (*words_remaining as f64 / (2.0f64.powf(score))).ceil() as usize
    );
}

fn search(words: &[Word], valid: &[Word], threads: usize, state: &MatchState) -> SearchResult {
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

    SearchResult {
        scores,
        win,
        words_remaining: rem,
    }
}

struct SearchResult {
    scores: Vec<(Word, f64)>,
    win: Option<Word>,
    words_remaining: usize,
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

fn get_stdin(buffer: &mut String) {
    buffer.clear();
    std::io::stdin().read_line(buffer).unwrap();
}

fn handle_run(valid_words: &[Word], answer_words: &[Word]) {
    let mut buffer = String::new();
    let mut state = MatchState::empty();
    let mut last_search_result = SearchResult {
        scores: vec![],
        win: None,
        words_remaining: usize::MAX,
    };

    loop {
        println!();
        println!("(a) Add info");
        println!("(v) View state");
        println!("(c) Calc");
        println!("(w) Show stats for a word");
        println!("(l) List computed scores");
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
                get_stdin(&mut buffer);
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
                last_search_result = handle_calc(&state, valid_words, answer_words);
            }
            'w' => {
                if last_search_result.words_remaining == usize::MAX {
                    println!("Compute scores first!");
                    return;
                }

                println!("Enter word to calculate its score:");
                get_stdin(&mut buffer);
                let mut s = [0; WORD_LENGTH];
                s.copy_from_slice(buffer.trim().as_bytes());
                let word = Word(s);
                handle_calc_word(&last_search_result, word);
            }
            'l' => {
                if last_search_result.words_remaining == usize::MAX {
                    println!("Compute scores first!");
                    return;
                }
                println!("Range of scores to view:");
                get_stdin(&mut buffer);
                let (left, right) = if let Some(x) = buffer.trim().split_once('-') {
                    x
                } else {
                    println!("Insert a range e.g. 1-50!");
                    continue;
                };
                let (left, right) = (
                    match left.parse::<usize>() {
                        Ok(x) => x,
                        _ => {
                            println!("Insert a range with a valid positive integer!");
                            continue;
                        }
                    },
                    match right.parse::<usize>() {
                        Ok(x) => x,
                        _ => {
                            println!("Insert a range with a valid positive integer!");
                            continue;
                        }
                    },
                );
                if left == 0 {
                    println!("Insert a valid range!");
                }
                for i in (left - 1)..right {
                    let (word, score) = last_search_result.scores[i];
                    println!("{}. {} {:.3}", i + 1, word, score)
                }
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
    let mut buffer = String::new();
    let valid_words = {
        let mut f = OpenOptions::new()
            .read(true)
            .write(false)
            .open("words.txt")
            .expect("Unable to find or open words.txt");
        f.read_to_string(&mut buffer)
            .expect("Unable to read words.txt");
        parse_words(buffer.as_str())
    };

    let answer_words = {
        let mut f = OpenOptions::new()
            .read(true)
            .write(false)
            .open("answers.txt")
            .expect("Unable to find or open answers.txt");
        f.read_to_string(&mut buffer)
            .expect("Unable to read answers.txt");
        parse_words(buffer.as_str())
    };

    let matches = command!()
        .subcommand(Command::new("merge"))
        .subcommand(Command::new("calc"))
        .subcommand(Command::new("run"))
        .get_matches();

    match matches.subcommand() {
        Some(("merge", _subm)) => handle_merge(),
        Some(("calc", _subm)) => {
            let mut buffer = String::new();
            get_stdin(&mut buffer);
            let state = MatchState::deserialize(&buffer);
            handle_calc(&state, &valid_words, &answer_words);
        }
        Some(("run", _subm)) => handle_run(&valid_words, &answer_words),
        _ => {}
    }
}

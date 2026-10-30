use std::collections::{HashMap, HashSet, VecDeque};
use std::env;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{OnceLock, RwLock};
use std::time::{SystemTime, UNIX_EPOCH};

use fst::raw::Fst;

static FST_BYTES: &[u8] = include_bytes!("../../data/words.fst");
static COMMON_WORDS_TXT: &str = include_str!("../../data/common_words.txt");

static FST_INSTANCE: OnceLock<Option<Fst<&'static [u8]>>> = OnceLock::new();
static COMMON_WORDS_RANK: OnceLock<HashMap<&'static str, usize>> = OnceLock::new();
static USER_WORDS: OnceLock<RwLock<HashMap<String, u32>>> = OnceLock::new();

#[derive(serde::Serialize, serde::Deserialize)]
struct UserWordStore {
    version: u32,
    words: HashMap<String, u32>,
}

fn get_fst() -> Option<&'static Fst<&'static [u8]>> {
    FST_INSTANCE
        .get_or_init(|| match Fst::new(FST_BYTES) {
            Ok(fst) => Some(fst),
            Err(error) => {
                eprintln!("ibus-bornona: could not load the suggestion dictionary: {error}");
                None
            }
        })
        .as_ref()
}

fn get_common_words() -> &'static HashMap<&'static str, usize> {
    COMMON_WORDS_RANK.get_or_init(|| {
        let mut map = HashMap::with_capacity(5000);
        for (rank, line) in COMMON_WORDS_TXT.lines().enumerate() {
            let trimmed = line.trim();
            if !trimmed.is_empty() {
                map.insert(trimmed, rank);
            }
        }
        map
    })
}

fn parse_user_words(bytes: &[u8], path: &Path) -> HashMap<String, u32> {
    match serde_json::from_slice::<UserWordStore>(bytes) {
        Ok(store) if store.version == 1 => store.words,
        Ok(_) => {
            eprintln!("ibus-bornona: unsupported user dictionary version in {}", path.display());
            HashMap::new()
        }
        Err(store_error) => match serde_json::from_slice(bytes) {
            Ok(words) => words,
            Err(_) => {
                eprintln!("ibus-bornona: could not parse {}: {store_error}", path.display());
                HashMap::new()
            }
        },
    }
}

fn user_words_path() -> Result<Option<PathBuf>, String> {
    if let Some(custom) = env::var_os("IBUS_BORNONA_USER_WORDS_PATH") {
        return Ok(Some(PathBuf::from(custom)));
    }
    if cfg!(test) {
        return Ok(None);
    }
    let base = match env::var_os("XDG_DATA_HOME") {
        Some(path) if !path.is_empty() => {
            let path = PathBuf::from(path);
            if !path.is_absolute() {
                return Err("XDG_DATA_HOME must be an absolute path".into());
            }
            path
        }
        _ => {
            let home = env::var_os("HOME")
                .map(PathBuf::from)
                .ok_or_else(|| "HOME is not set and XDG_DATA_HOME is unavailable".to_string())?;
            home.join(".local/share")
        }
    };
    Ok(Some(base.join("ibus-bornona").join("user_dict.json")))
}

fn get_user_words() -> &'static RwLock<HashMap<String, u32>> {
    USER_WORDS.get_or_init(|| {
        let map = if let Some(path) = user_words_path().unwrap_or_else(|error| {
            eprintln!("ibus-bornona: {error}");
            None
        }) {
            match fs::read(&path) {
                Ok(bytes) => parse_user_words(&bytes, &path),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => HashMap::new(),
                Err(error) => {
                    eprintln!("ibus-bornona: could not read {}: {error}", path.display());
                    HashMap::new()
                }
            }
        } else {
            HashMap::new()
        };
        RwLock::new(map)
    })
}

fn char_to_byte(c: char) -> u8 {
    c as u8
}

fn byte_to_char(b: u8) -> Option<char> {
    if b == 0x0C {
        Some('\u{200C}')
    } else if b == 0x0D {
        Some('\u{200D}')
    } else {
        char::from_u32(0x0900 | (b as u32))
    }
}

fn normalize_prefix_for_fst(prefix: &str) -> String {
    prefix.replace("\u{09AF}\u{09BC}", "\u{09DF}")
}

fn normalize_candidate_for_output(word: &str) -> String {
    word.replace('\u{09DF}', "\u{09AF}\u{09BC}")
}

pub struct SuggestionEngine;

impl SuggestionEngine {
    /// Learn a chosen word by increasing its user frequency count.
    pub fn learn_word(word: &str) -> Result<(), String> {
        if word.is_empty() {
            return Ok(());
        }

        let lock = get_user_words();
        let mut map = lock
            .write()
            .map_err(|_| "user dictionary lock is poisoned".to_string())?;
        let mut updated = map.clone();
        let count = updated.entry(word.to_string()).or_insert(0);
        *count = count.saturating_add(1);
        persist_user_words(&updated)?;
        *map = updated;
        Ok(())
    }

    pub fn clear_learned_words() -> Result<(), String> {
        let lock = get_user_words();
        let mut map = lock
            .write()
            .map_err(|_| "user dictionary lock is poisoned".to_string())?;
        persist_user_words(&HashMap::new())?;
        map.clear();
        Ok(())
    }

    /// Retrieve candidates for a given Bengali prefix, sorted from most relevant to least relevant.
    pub fn suggest(prefix: &str, max_results: usize) -> Vec<String> {
        Self::suggest_with_learning(prefix, max_results, true)
    }

    pub fn suggest_with_learning(
        prefix: &str,
        max_results: usize,
        learning_enabled: bool,
    ) -> Vec<String> {
        if prefix.is_empty() {
            return Vec::new();
        }

        let Some(fst) = get_fst() else {
            return Vec::new();
        };
        let mut node = fst.root();
        let fst_prefix = normalize_prefix_for_fst(prefix);

        // Traverse down the prefix
        for c in fst_prefix.chars() {
            let byte = char_to_byte(c);
            match node.find_input(byte) {
                Some(idx) => {
                    node = fst.node(node.transition_addr(idx));
                }
                None => {
                    // Prefix not found in dictionary
                    return Vec::new();
                }
            }
        }

        let mut candidates = Vec::new();
        let mut visited = HashSet::new();

        // 1. If prefix itself is in dictionary, include it
        if node.is_final() {
            let prefix_out = normalize_candidate_for_output(prefix);
            candidates.push(prefix_out.clone());
            visited.insert(prefix_out);
        }

        // 2. BFS traversal to discover completions
        let mut queue = VecDeque::new();
        queue.push_back((node, String::new()));

        while let Some((curr_node, suffix)) = queue.pop_front() {
            for i in 0..curr_node.len() {
                let trans = curr_node.transition(i);
                if let Some(ch) = byte_to_char(trans.inp) {
                    let next_node = fst.node(trans.addr);
                    let mut next_suffix = suffix.clone();
                    next_suffix.push(ch);

                    let full_word = format!("{}{}", fst_prefix, next_suffix);
                    if next_node.is_final() {
                        let output_word = normalize_candidate_for_output(&full_word);
                        if visited.insert(output_word.clone()) {
                            candidates.push(output_word);
                            if candidates.len() >= 120 {
                                break;
                            }
                        }
                    }

                    // Continue BFS expansion up to 7 characters added
                    if next_suffix.chars().count() <= 7 {
                        queue.push_back((next_node, next_suffix));
                    }
                }
            }
            if candidates.len() >= 120 {
                break;
            }
        }

        if candidates.is_empty() {
            return Vec::new();
        }

        // 3. Score and sort candidates from most relevant to least relevant
        let common_words = get_common_words();
        let user_words_guard = if learning_enabled {
            Some(get_user_words().read().unwrap_or_else(|error| {
                eprintln!("ibus-bornona: user dictionary lock is poisoned");
                error.into_inner()
            }))
        } else {
            None
        };
        let user_map = user_words_guard.as_deref();

        let prefix_len = prefix.chars().count();

        let mut scored_candidates: Vec<(f32, String)> = candidates
            .into_iter()
            .map(|word| {
                let score = Self::calculate_relevance(
                    &word,
                    prefix,
                    prefix_len,
                    common_words,
                    user_map,
                );
                (score, word)
            })
            .collect();

        // Sort descending by score. On tie, sort shorter word first, then alphabetical.
        scored_candidates.sort_by(|(score_a, word_a), (score_b, word_b)| {
            score_b
                .partial_cmp(score_a)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| word_a.chars().count().cmp(&word_b.chars().count()))
                .then_with(|| word_a.cmp(word_b))
        });

        scored_candidates
            .into_iter()
            .take(max_results)
            .map(|(_, word)| word)
            .collect()
    }

    fn calculate_relevance(
        word: &str,
        prefix: &str,
        prefix_len: usize,
        common_words: &HashMap<&str, usize>,
        user_words: Option<&HashMap<String, u32>>,
    ) -> f32 {
        let mut score = 500.0;

        // 1. User learned frequency bonus (highest priority)
        if let Some(user_map) = user_words {
            if let Some(&count) = user_map.get(word) {
                score += 1500.0 + (count.min(100) as f32) * 50.0;
            }
        }

        // 2. Exact match bonus: prefix itself
        if word == prefix {
            score += 400.0;
        }

        // 3. Common words frequency bonus
        if let Some(&rank) = common_words.get(word) {
            // Rank 0 gets +600, rank 5000 gets ~0
            let bonus = (5000.0 - rank as f32).max(0.0) * 0.12;
            score += 200.0 + bonus;
        }

        // 4. Length penalty: words closer to prefix length are much more likely
        let word_len = word.chars().count();
        let diff = word_len.saturating_sub(prefix_len);
        score -= (diff as f32) * 25.0;

        // 5. Common Bengali suffix grammatical inflection bonus
        if word.starts_with(prefix) {
            let suffix = &word[prefix.len()..];
            if matches!(
                suffix,
                "র" | "ের"
                    | "কে"
                    | "তে"
                    | "ে"
                    | "য়"
                    | "\u{09AF}\u{09BC}"
                    | "টি"
                    | "টা"
                    | "গুলো"
                    | "রা"
                    | "দের"
                    | "ই"
                    | "ও"
            ) {
                score += 45.0;
            }
        }

        score
    }
}

fn persist_user_words(words: &HashMap<String, u32>) -> Result<(), String> {
    let Some(path) = user_words_path()? else {
        return Ok(());
    };
    persist_user_words_at(&path, words)
}

fn persist_user_words_at(path: &Path, words: &HashMap<String, u32>) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| "could not determine user dictionary directory".to_string())?;
    fs::create_dir_all(parent)
        .map_err(|error| format!("could not create {}: {error}", parent.display()))?;
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| format!("system clock error: {error}"))?
        .as_nanos();
    let temporary_path = path.with_extension(format!("json.tmp-{}-{nonce}", std::process::id()));
    let result = (|| {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary_path)
            .map_err(|error| format!("could not create {}: {error}", temporary_path.display()))?;
        let store = UserWordStore {
            version: 1,
            words: words.clone(),
        };
        serde_json::to_writer(&mut file, &store)
            .map_err(|error| format!("could not serialize user dictionary: {error}"))?;
        file.flush()
            .map_err(|error| format!("could not write {}: {error}", temporary_path.display()))?;
        file.sync_all()
            .map_err(|error| format!("could not sync {}: {error}", temporary_path.display()))?;
        drop(file);
        fs::rename(&temporary_path, path)
            .map_err(|error| format!("could not save {}: {error}", path.display()))
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary_path);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    static TEST_MUTEX: Mutex<()> = Mutex::new(());

    fn clear_user_words() {
        let lock = get_user_words();
        let mut map = lock.write().unwrap_or_else(|e| e.into_inner());
        map.clear();
    }

    #[test]
    fn test_prefix_suggestions_and_relevance() {
        let _guard = TEST_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        clear_user_words();

        let suggestions = SuggestionEngine::suggest("আমা", 9);
        println!("Suggestions for আমা: {:?}", suggestions);
        assert!(!suggestions.is_empty());
        assert!(suggestions.len() <= 9);
        // "আমার" and "আমাদের" and "আমাকে" should be at the top
        assert_eq!(suggestions[0], "আমার");
        assert!(suggestions.contains(&"আমাদের".to_string()));
        assert!(suggestions.contains(&"আমাকে".to_string()));

        let bangla = SuggestionEngine::suggest("বাংল", 9);
        println!("Suggestions for বাংল: {:?}", bangla);
        assert!(!bangla.is_empty());
        assert_eq!(bangla[0], "বাংলা");
        assert!(bangla.contains(&"বাংলাদেশ".to_string()));
    }

    #[test]
    fn test_user_learning_boosts_relevance() {
        let _guard = TEST_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        clear_user_words();

        let prefix = "আমা";
        let before = SuggestionEngine::suggest(prefix, 9);
        assert_ne!(before[0], "আমানত");

        // Learn "আমানত" repeatedly
        for _ in 0..10 {
            SuggestionEngine::learn_word("আমানত").unwrap();
        }

        let after = SuggestionEngine::suggest(prefix, 9);
        println!("After learning আমানত: {:?}", after);
        assert_eq!(after[0], "আমানত");

        clear_user_words();
    }

    #[test]
    fn user_dictionary_reads_versioned_and_legacy_stores_and_recovers_from_corruption() {
        let path = Path::new("user_dict.json");
        let words = HashMap::from([("আমার".to_string(), 3)]);
        let versioned = serde_json::to_vec(&UserWordStore {
            version: 1,
            words: words.clone(),
        })
        .unwrap();
        assert_eq!(parse_user_words(&versioned, path), words);

        let legacy = serde_json::to_vec(&words).unwrap();
        assert_eq!(parse_user_words(&legacy, path), words);
        assert!(parse_user_words(b"{broken", path).is_empty());
    }

    #[test]
    fn user_dictionary_persistence_is_readable_after_atomic_write() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let directory = env::temp_dir().join(format!(
            "ibus-bornona-test-{}-{unique}",
            std::process::id()
        ));
        fs::create_dir(&directory).unwrap();
        let path = directory.join("user_dict.json");
        let words = HashMap::from([("বাংলা".to_string(), 2)]);

        persist_user_words_at(&path, &words).unwrap();
        assert_eq!(
            parse_user_words(&fs::read(&path).unwrap(), &path),
            words
        );

        fs::remove_file(path).unwrap();
        fs::remove_dir(directory).unwrap();
    }
}

//! Add the given text as list of reactions to the given message or
//! previous message.

use std::collections::HashMap;

use lazy_static::lazy_static;

/// Text to emoji compatible text.
pub fn text_to_emoji(text: &str) -> Option<String> {
    lazy_static! {
        /// Alternatives for the [`char`]s.
        pub static ref ALTERNATIVES: HashMap<char, &'static [char]> = [
            ('a', ['4'].as_slice()),
            ('b', &['8']),
            ('e', &['3']),
            ('g', &['9']),
            ('i', &['1', '!']),
            ('l', &['1']),
            ('o', &['0']),
            ('s', &['5', '$', 'z']),
            ('t', &['7']),
            ('u', &['v']),
            ('z', &['s']),
        ].into_iter().collect();

        /// Allowed frequency for the [`char`]s.
        pub static ref ALLOWED_FREQUENCY: HashMap<char, usize> = [
            ('a', 2),
            ('b', 2),
            ('i', 2)
        ].into_iter().collect();
    }

    let mut used_characters = HashMap::new();
    text.to_lowercase()
        .chars()
        .filter(|c| !c.is_whitespace())
        .map(|c| {
            if used_characters.contains_key(&c) {
                if used_characters.get(&c).unwrap() < ALLOWED_FREQUENCY.get(&c).unwrap_or(&1) {
                    *used_characters.get_mut(&c).unwrap() += 1;
                    Some(c)
                } else {
                    let alternative =
                        *ALTERNATIVES
                            .get(&c)?
                            .iter()
                            .find(|c| match used_characters.get(*c) {
                                Some(num_allowed) => {
                                    num_allowed < ALLOWED_FREQUENCY.get(c).unwrap_or(&1)
                                }
                                None => true,
                            })?;

                    *used_characters.entry(alternative).or_insert(0) += 1;

                    Some(alternative)
                }
            } else {
                used_characters.insert(c, 1);
                Some(c)
            }
        })
        .collect()
}

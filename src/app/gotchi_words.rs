//! What a Pokégotchi thinks, dreams and gets up to: the flavour text for
//! its speech bubble and the journal. `{n}` is its name.

use crate::cards::Rng;
use crate::gotchi::Mood;

/// Things it does while you're away (a few are picked each time).
pub const AWAY: &[&str] = &[
    "{n} napped in a sunny corner of the screen.",
    "{n} chased a cloud that looked a lot like a Wurmple.",
    "{n} rearranged the berry stash. Twice.",
    "{n} practised its best battle pose.",
    "{n} sat by the window, waiting for you.",
    "{n} found a shiny pebble and hid it somewhere safe.",
    "{n} hummed the Route 201 tune, slightly off-key.",
    "{n} counted the pixels on the screen. It lost count at 400.",
    "{n} tried to catch its own tail.",
    "{n} had a long chat with nobody in particular.",
    "{n} watched the rare grass sway in the breeze.",
    "{n} wrote you a letter, then ate it.",
    "{n} built a tiny fort out of berry leaves.",
    "{n} pretended the buttons were a drum kit.",
    "{n} went on an adventure to the other side of the screen.",
    "{n} looked at old photos of your hunts.",
    "{n} practised Sweet Scent. Nothing came.",
    "{n} stared at the clock until it blinked.",
];

/// Its thoughts, any time.
pub const THOUGHTS: &[&str] = &[
    "Is a shiny's shine catching?",
    "Today feels like a lucky day.",
    "What if the next one is THE one?",
    "I like it when you're here.",
    "Sweet Scent smells nice.",
    "Do Pokémon in the tall grass get lonely?",
    "Sparkles are my favourite colour.",
    "I wonder what berries taste like in Hoenn.",
    "If I squint, everything's shiny.",
    "One day I'll have a Pokégotchi of my own.",
    "Does Professor Rowan ever sleep?",
    "I bet a Bidoof is thinking about me too.",
    "Rare grass is the best grass.",
    "We make a good team, you and me.",
    "I'm going to count to a hundred. 1… 2… …what was I doing?",
    "Is it snack o'clock yet?",
    "I'm not sleepy. You're sleepy.",
    "Hoping for a holo in your next pack!",
];

/// Thoughts while you hunt (`{t}` the target, `{c}` the count).
pub const HUNTING: &[&str] = &[
    "Still hoping for {t}…",
    "{c} encounters! You're so patient.",
    "Come on, {t}, show your sparkles!",
    "I can feel it. {t} is close.",
    "Every encounter is a berry closer to dinner.",
    "Is that… no. Not shiny. Next!",
];

/// Thoughts late at night and early in the morning.
pub const NIGHT: &[&str] = &["It's late. Hunters need sleep too!", "The moon's out. Shinies come out at night, right?", "*yawn* Just one more encounter…"];
pub const MORNING: &[&str] = &["Good morning! Berries for breakfast?", "Rise and shine! Get it? Shine?", "A brand new day of hunting!"];

/// Dreams while asleep.
pub const DREAMS: &[&str] = &[
    "dreams of a mountain of berries",
    "dreams about the shiny you'll find tomorrow",
    "mumbles something about Sweet Scent",
    "dreams it's a Legendary for a day",
    "smiles in its sleep",
    "dreams of rare grass that goes on forever",
];

/// Little moments of its own while the app is open. The first few can
/// turn up a berry.
pub const FUN_BERRY: &[&str] = &["{n} found a berry behind the screen!", "{n} shook the device and a berry fell out!", "A berry rolled out from under {n}'s bed!"];
pub const FUN: &[&str] = &[
    "{n} did a little dance.",
    "{n} sneezed. Bless you!",
    "{n} spotted a Starly flying past the window.",
    "{n} hiccuped and surprised itself.",
    "{n} tried on a tiny hat. Very dapper.",
    "{n} challenged its reflection to a staring contest. It lost.",
    "{n} learned a new trick: spinning really fast.",
    "{n} tidied up without being asked!",
    "{n} hid behind the needs bars. You can still see its ears.",
];

/// When you come back.
pub const WELCOME: &[&str] = &["{n} perked up when you came back!", "{n} ran to the front of the screen to say hi!", "{n} was so happy to see you again!"];

fn pick<'a>(rng: &mut Rng, list: &'a [&'a str]) -> &'a str {
    list[rng.below(list.len())]
}

pub fn fill(s: &str, name: &str) -> String {
    s.replace("{n}", name)
}

/// A thought for right now.
pub fn thought(rng: &mut Rng, mood: Mood, hour: u32, target: Option<&str>, count: u64) -> String {
    // how it feels comes first when something's low
    let felt = match mood {
        Mood::Hungry => Some(["My tummy's rumbling…", "Is that a berry I smell?", "Food. Food would be nice."]),
        Mood::Sleepy => Some(["So… sleepy…", "Five more minutes…", "Lights off soon?"]),
        Mood::Messy => Some(["Could use a bath.", "Is it me, or is it messy in here?", "Sparkly clean, please?"]),
        Mood::Bored => Some(["Play with me?", "Higher or Lower? Pleeease?", "I'm bored. Let's do something!"]),
        Mood::Grumpy => Some(["Hmph.", "Not a great day…", "I could use some looking after."]),
        _ => None,
    };
    if let Some(l) = felt {
        if rng.below(3) > 0 {
            return pick(rng, &l).to_string();
        }
    }
    let roll = rng.below(10);
    if roll < 2 && (hour >= 23 || hour < 4) {
        return pick(rng, NIGHT).to_string();
    }
    if roll < 2 && (5..10).contains(&hour) {
        return pick(rng, MORNING).to_string();
    }
    if roll < 5 {
        if let Some(t) = target.filter(|t| !t.is_empty() && *t != "No target yet") {
            let c = crate::theme::thousands(count);
            return pick(rng, HUNTING).replace("{t}", t).replace("{c}", &c);
        }
    }
    pick(rng, THOUGHTS).to_string()
}

/// A few things it got up to over `secs` away, oldest first.
pub fn away(rng: &mut Rng, name: &str, secs: u64) -> Vec<String> {
    let n = if secs > 8 * 3600 { 3 } else if secs > 2 * 3600 { 2 } else { 1 };
    let mut out: Vec<String> = Vec::new();
    while out.len() < n {
        let line = fill(pick(rng, AWAY), name);
        if !out.contains(&line) {
            out.push(line);
        }
    }
    out
}

/// "3 hours", "40 minutes", "2 days".
pub fn span(secs: u64) -> String {
    let m = secs / 60;
    let (v, unit) = if m < 60 {
        (m.max(1), "minute")
    } else if m < 60 * 48 {
        (m / 60, "hour")
    } else {
        (m / 1440, "day")
    };
    format!("{v} {unit}{}", if v == 1 { "" } else { "s" })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lines_fill_in_and_never_repeat_while_away() {
        let mut rng = Rng::new(3);
        let lines = away(&mut rng, "Eevee", 10 * 3600);
        assert_eq!(lines.len(), 3);
        assert!(lines.iter().all(|l| l.contains("Eevee") && !l.contains("{n}")));
        let t = thought(&mut rng, Mood::Happy, 12, Some("Mudkip"), 1300);
        assert!(!t.contains('{'));
        assert_eq!(span(90), "1 minute");
        assert_eq!(span(3 * 3600 + 5), "3 hours");
        assert_eq!(span(3 * 86400), "3 days");
    }
}

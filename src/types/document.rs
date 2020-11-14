use std::convert::TryInto;
use std::fmt;

use itertools::Itertools;
use crate::types::URIReference;

#[derive(Default)]
pub struct Document {
    items: Vec<Item>,
}

impl Document {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_item(&mut self, item: Item) -> &mut Self {
        self.items.push(item);
        self
    }

    pub fn add_items<I>(&mut self, items: I) -> &mut Self
    where
        I: IntoIterator<Item = Item>,
    {
        self.items.extend(items);
        self
    }

    pub fn add_blank_line(&mut self) -> &mut Self {
        self.add_item(Item::Text(Text::blank()))
    }

    pub fn add_text(&mut self, text: &str) -> &mut Self {
        let text = text
            .lines()
            .map(Text::new_lossy)
            .map(Item::Text);

        self.add_items(text);

        self
    }

    pub fn add_link<'a, U>(&mut self, uri: U, label: impl AsRef<str> + Into<String>) -> &mut Self
    where
        U: TryInto<URIReference<'a>>,
    {
        let uri = uri
            .try_into()
            .map(URIReference::into_owned)
            .or_else(|_| ".".try_into()).expect("Northstar BUG");
        let label = LinkLabel::from_lossy(label);
        let link = Link { uri, label: Some(label) };
        let link = Item::Link(link);

        self.add_item(link);

        self
    }

    pub fn add_link_without_label(&mut self, uri: URIReference<'static>) -> &mut Self {
        let link = Link {
            uri,
            label: None,
        };
        let link = Item::Link(link);

        self.add_item(link);
    
        self
    }

    pub fn add_preformatted(&mut self, preformatted_text: &str) -> &mut Self {
        self.add_preformatted_with_alt("", preformatted_text)
    }

    pub fn add_preformatted_with_alt(&mut self, alt: &str, preformatted_text: &str) -> &mut Self {
        let alt = AltText::new_lossy(alt);
        let lines = preformatted_text
            .lines()
            .map(PreformattedText::new_lossy)
            .collect();
        let preformatted = Preformatted {
            alt,
            lines,
        };
        let preformatted = Item::Preformatted(preformatted);

        self.add_item(preformatted);

        self
    }

    pub fn add_heading(&mut self, level: HeadingLevel, text: impl AsRef<str> + Into<String>) -> &mut Self {
        let text = HeadingText::new_lossy(text);
        let heading = Heading {
            level,
            text,
        };
        let heading = Item::Heading(heading);

        self.add_item(heading);

        self
    }

    pub fn add_unordered_list_item(&mut self, text: &str) -> &mut Self {
        let item = UnorderedListItem::new_lossy(text);
        let item = Item::UnorderedListItem(item);

        self.add_item(item);

        self
    }

    pub fn add_quote(&mut self, text: &str) -> &mut Self {
        let quote = text
            .lines()
            .map(Quote::new_lossy)
            .map(Item::Quote);
        
        self.add_items(quote);

        self
    }
}

impl fmt::Display for Document {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for item in &self.items {
            match item {
                Item::Text(text) => writeln!(f, "{}", text.0)?,
                Item::Link(link) => {
                    let separator = if link.label.is_some() {" "} else {""};
                    let label = link.label.as_ref().map(|label| label.0.as_str())
                        .unwrap_or("");

                    writeln!(f, "=>{}{}{}", link.uri, separator, label)?;
                }
                Item::Preformatted(preformatted) => {
                    writeln!(f, "```{}", preformatted.alt.0)?;

                    for line in &preformatted.lines {
                        writeln!(f, "{}", line.0)?;
                    }

                    writeln!(f, "```")?
                }
                Item::Heading(heading) => {
                    let level = match heading.level {
                        HeadingLevel::H1 => "#",
                        HeadingLevel::H2 => "##",
                        HeadingLevel::H3 => "###",
                    };

                    writeln!(f, "{} {}", level, heading.text.0)?;
                }
                Item::UnorderedListItem(item) => writeln!(f, "* {}", item.0)?,
                Item::Quote(quote) => writeln!(f, "> {}", quote.0)?,
            }
        }

        Ok(())
    }
}

pub enum Item {
    Text(Text),
    Link(Link),
    Preformatted(Preformatted),
    Heading(Heading),
    UnorderedListItem(UnorderedListItem),
    Quote(Quote),
}

#[derive(Default)]
pub struct Text(String);

impl Text {
    pub fn blank() -> Self {
        Self::default()
    }

    pub fn new_lossy(line: impl AsRef<str> + Into<String>) -> Self {
        Self(lossy_escaped_line(line, SPECIAL_STARTS))
    }
}

pub struct Link {
    pub uri: URIReference<'static>,
    pub label: Option<LinkLabel>,
}

pub struct LinkLabel(String);

impl LinkLabel {
    pub fn from_lossy(line: impl AsRef<str> + Into<String>) -> Self {
        let line = strip_newlines(line);
        
        LinkLabel(line)
    }
}

pub struct Preformatted {
    pub alt: AltText,
    pub lines: Vec<PreformattedText>,
}

pub struct PreformattedText(String);

impl PreformattedText {
    pub fn new_lossy(line: impl AsRef<str> + Into<String>) -> Self {
        Self(lossy_escaped_line(line, &[PREFORMATTED_TOGGLE_START]))
    }
}

pub struct AltText(String);

impl AltText {
    pub fn new_lossy(alt: &str) -> Self {
        let alt = strip_newlines(alt);
        
        Self(alt)
    }
}

pub struct Heading {
    pub level: HeadingLevel,
    pub text: HeadingText,
}

pub enum HeadingLevel {
    H1,
    H2,
    H3,
}

impl Heading {
    pub fn new_lossy(level: HeadingLevel, line: &str) -> Self {
        Self {
            level,
            text: HeadingText::new_lossy(line),
        }
    }
}

pub struct HeadingText(String);

impl HeadingText {
    pub fn new_lossy(line: impl AsRef<str> + Into<String>) -> Self {
        let line = strip_newlines(line);

        Self(lossy_escaped_line(line, &[HEADING_START]))
    }
}

pub struct UnorderedListItem(String);

impl UnorderedListItem {
    pub fn new_lossy(text: &str) -> Self {
        let text = strip_newlines(text);
        
        Self(text)
    }
}

pub struct Quote(String);

impl Quote {
    pub fn new_lossy(text: &str) -> Self {
        Self(lossy_escaped_line(text, &[QUOTE_START]))
    }
}


const LINK_START: &str = "=>";
const PREFORMATTED_TOGGLE_START: &str = "```";
const HEADING_START: &str = "#";
const UNORDERED_LIST_ITEM_START: &str = "*";
const QUOTE_START: &str = ">";

const SPECIAL_STARTS: &[&str] = &[
    LINK_START,
    PREFORMATTED_TOGGLE_START,
    HEADING_START,
    UNORDERED_LIST_ITEM_START,
    QUOTE_START,
];

fn starts_with_any(s: &str, starts: &[&str]) -> bool {
    for start in starts {
        if s.starts_with(start) {
            return true;
        }
    }

    false
}

fn lossy_escaped_line(line: impl AsRef<str> + Into<String>, escape_starts: &[&str]) -> String {
    let line_ref = line.as_ref();
    let contains_newline = line_ref.contains('\n');
    let has_special_start = starts_with_any(line_ref, escape_starts);

    if !contains_newline && !has_special_start {
        return line.into();
    }

    let mut line = String::new();

    if has_special_start {
        line.push(' ');
    }

    if let Some(line_ref) = line_ref.split('\n').next() {
        line.push_str(line_ref);
    }

    line
}

fn strip_newlines(text: impl AsRef<str> + Into<String>) -> String {
    if !text.as_ref().contains(&['\r', '\n'][..]) {
        return text.into();
    }

    text.as_ref()
        .lines()
        .filter(|part| !part.is_empty())
        .join(" ")
}

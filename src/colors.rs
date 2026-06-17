use facet::Facet;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

#[derive(Facet, Debug, Clone, PartialEq)]
pub struct ColorSchemeCollection {
    pub version: String,
    pub latte: Theme,
    pub frappe: Theme,
    pub macchiato: Theme,
    pub mocha: Theme,
}

impl ColorSchemeCollection {
    pub fn from_json_file(path: impl AsRef<Path>) -> Self {
        let json = fs::read_to_string(&path).expect("Unable to read file");
        let res = facet_json::from_str::<Self>(&json);
        if res.is_err() {
            let error = res.unwrap_err();
            panic!("JSON decoding failed because: {error}")
        }

        res.unwrap()
    }
}

#[derive(Facet, Debug, Clone, PartialEq)]
pub struct Theme {
    pub name: String,
    pub emoji: String,
    pub order: u32,
    pub dark: bool,
    pub colors: HashMap<String, PaletteColor>,
    #[facet(rename = "ansiColors")]
    pub ansi_colors: HashMap<String, AnsiColorPair>,
}

#[derive(Facet, Debug, Clone, PartialEq)]
pub struct PaletteColor {
    pub name: String,
    pub order: u32,
    pub hex: String,
    pub rgb: Rgb,
    pub hsl: Hsl,
    pub oklch: Oklch,
    pub accent: bool,
}

#[derive(Facet, Debug, Clone, PartialEq)]
pub struct AnsiColorPair {
    pub name: String,
    pub order: u32,
    pub normal: AnsiColorDetails,
    pub bright: AnsiColorDetails,
}

#[derive(Facet, Debug, Clone, PartialEq)]
pub struct AnsiColorDetails {
    pub name: String,
    pub hex: String,
    pub rgb: Rgb,
    pub hsl: Hsl,
    pub oklch: Oklch,
    pub code: u32,
}

#[derive(Facet, Debug, Clone, PartialEq)]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

#[derive(Facet, Debug, Clone, PartialEq)]
pub struct Hsl {
    pub h: f64,
    pub s: f64,
    pub l: f64,
}

#[derive(Facet, Debug, Clone, PartialEq)]
pub struct Oklch {
    pub l: f64,
    pub c: f64,
    pub h: f64,
}

pub fn extract_colors(file_path: impl AsRef<Path>) -> (Accents, Neutrals) {
    let color_scheme_collection = ColorSchemeCollection::from_json_file(file_path);

    let mut accents = Vec::new();
    let mut neutrals = Vec::new();

    let themes = {
        let ColorSchemeCollection {
            latte,
            frappe,
            macchiato,
            mocha,
            ..
        } = color_scheme_collection;

        let mut map = HashMap::new();

        map.insert("latte", latte);
        map.insert("frappe", frappe);
        map.insert("macchiato", macchiato);
        map.insert("mocha", mocha);

        map
    };

    themes
        .into_values()
        .map(|theme| {
            theme
                .colors
                .into_values()
                .map(|PaletteColor { accent, oklch, .. }| (accent, oklch))
        })
        .flatten()
        .map(|(accent, Oklch { l, c, h })| (accent, Oklch { l, c, h }))
        .for_each(|(accent, color)| {
            if accent {
                accents.push(color)
            } else {
                neutrals.push(color)
            }
        });

    let accents = Accents(accents);
    let neutrals = Neutrals(neutrals);

    (accents, neutrals)
}

#[derive(Debug, Clone, PartialEq)]
pub struct Accents(pub Vec<Oklch>);

#[derive(Debug, Clone, PartialEq)]
pub struct Neutrals(pub Vec<Oklch>);

use facet::Facet;
use image::Primitive;
use itertools::Itertools;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

#[derive(Facet, Debug, Clone, PartialEq)]
struct ColorSchemeCollection {
    latte: Theme,
    frappe: Theme,
    macchiato: Theme,
    mocha: Theme,
}

impl ColorSchemeCollection {
    fn from_json_file(path: impl AsRef<Path>) -> Self {
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
struct Theme {
    colors: HashMap<String, PaletteColor>,
}

#[derive(Facet, Debug, Clone, PartialEq)]
struct PaletteColor {
    hex: String,
    oklch: Oklch,
    accent: bool,
}

#[derive(Facet, Debug, Clone, PartialEq)]
struct Oklch {
    l: f32,
    c: f32,
    h: f32,
}

pub(crate) fn extract_colors(file_path: impl AsRef<Path>) -> (Accents, Neutrals) {
    let color_scheme_collection = ColorSchemeCollection::from_json_file(file_path);

    let mut accents = HashMap::new();
    let mut neutrals = HashMap::new();

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
            theme.colors.into_values().map(
                |PaletteColor {
                     accent, oklch, hex, ..
                 }| (accent, oklch, hex),
            )
        })
        .flatten()
        .map(|(accent, Oklch { l, c, h }, hex)| (accent, Oklch { l, c, h }, hex))
        .for_each(|(accent, color, hex)| {
            if accent {
                accents.insert(hex, color);
            } else {
                neutrals.insert(hex, color);
            }
        });

    let accents = Accents(
        accents
            .into_values()
            .map(|Oklch { l, c, h }| palette::Oklch {
                l,
                chroma: c,
                hue: palette::OklabHue::from(h),
            })
            .collect(),
    );
    let neutrals = Neutrals(
        neutrals
            .into_values()
            .map(|Oklch { l, c, h }| palette::Oklch {
                l,
                chroma: c,
                hue: palette::OklabHue::from(h),
            })
            .collect(),
    );

    (accents, neutrals)
}

pub enum Accent {
    Solid(palette::Oklch),
    Duo {
        from: palette::Oklch,
        t: f32,
        to: palette::Oklch,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct Accents(Vec<palette::Oklch>);

impl Accents {
    pub fn find_closest(&self, needle: palette::Oklch) -> Accent {
        todo!("Accents::find_closest")
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Neutrals(Vec<palette::Oklch>);

impl Neutrals {
    pub fn find_closest(&self, needle: palette::Oklch) -> palette::Oklch {
        self.0
            .iter()
            .cloned()
            .min_by(|a, b| {
                let dist_a = (needle.l - a.l).powi(2) + (needle.chroma - a.chroma).powi(2);
                let dist_b = (needle.l - b.l).powi(2) + (needle.chroma - b.chroma).powi(2);

                dist_a
                    .partial_cmp(&dist_b)
                    .expect("Encountered NaN in Neutrals::find_closest")
            })
            .expect("Neutrals had 0 colors")
    }
}

use crate::noise::NoiseMaps;
use facet::Facet;
use image::Primitive;
use itertools::Itertools;
use kiddo::{ImmutableKdTree, KdTree, SquaredEuclidean};
use palette::{GetHue, Mix, Oklch};
use std::collections::HashMap;
use std::fs;
use std::ops::Deref;
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
    oklch: MyOklch,
    accent: bool,
}

#[derive(Facet, Debug, Clone, PartialEq)]
struct MyOklch {
    l: f32,
    c: f32,
    h: f32,
}

pub(crate) fn extract_colors(file_path: impl AsRef<Path>) -> (Accents, Neutrals) {
    println!("Extracting Colors");

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

    for (accent, color, hex) in themes
        .into_values()
        .map(|theme| {
            theme.colors.into_values().map(
                |PaletteColor {
                     accent, oklch, hex, ..
                 }| (accent, oklch, hex),
            )
        })
        .flatten()
        .map(|(accent, MyOklch { l, c, h }, hex)| (accent, MyOklch { l, c, h }, hex))
    {
        if accent {
            accents.insert(hex.clone(), color.clone());
        }
        neutrals.insert(hex, color);
    }

    println!("Generating palette");
    let solid_accents = accents
        .into_values()
        .map(|MyOklch { l, c, h }| Oklch {
            l,
            chroma: c,
            hue: palette::OklabHue::from(h),
        })
        .map(|c| Accent::Solid(c))
        .collect::<Vec<_>>();
    let duo_accents = solid_accents
        .iter()
        .cartesian_product(solid_accents.iter())
        .cartesian_product(1..10u8)
        .flat_map(|((a, b), layer)| match (a, b) {
            (Accent::Solid(a), Accent::Solid(b)) if a != b => {
                let t = layer as f32 / 10.0;
                Some(Accent::Duo {
                    from: *a,
                    factor: t,
                    to: *b,
                })
            }
            (Accent::Solid(a), Accent::Solid(b)) if a == b => None,
            _ => unreachable!(),
        })
        .collect::<Vec<_>>();
    let accent_colors = solid_accents
        .into_iter()
        .chain(duo_accents.into_iter())
        .collect::<Vec<_>>();
    let accents_tree_slice = accent_colors
        .iter()
        .map(|accent| {
            let Oklch { l, chroma, hue } = accent.get();
            [l, chroma, hue.into_degrees()]
        })
        .collect::<Vec<_>>();
    let accents_tree = ImmutableKdTree::new_from_slice(&accents_tree_slice);
    let accents = Accents {
        tree: accents_tree,
        colors: accent_colors,
    };

    let neutrals = Neutrals(
        neutrals
            .into_values()
            .map(|MyOklch { l, c, h }| Oklch {
                l,
                chroma: c,
                hue: palette::OklabHue::from(h),
            })
            .collect(),
    );

    (accents, neutrals)
}

#[derive(Debug, Clone, PartialEq, Copy)]
pub enum Accent {
    Solid(Oklch),
    Duo { from: Oklch, factor: f32, to: Oklch },
}

impl Accent {
    pub fn get(&self) -> Oklch {
        match self {
            &Accent::Solid(c) => c,
            &Accent::Duo {
                from,
                factor: t,
                to,
            } => from.mix(to, t),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Accents {
    tree: ImmutableKdTree<f32, 3>,
    colors: Vec<Accent>,
}

impl Accents {
    pub fn find_closest(&self, needle: Oklch) -> Accent {
        let Oklch { l, chroma, hue } = needle;
        let idx = self
            .tree
            .approx_nearest_one::<SquaredEuclidean>(&[l, chroma, hue.into_degrees()])
            .item;
        let idx: usize = idx.try_into().expect("Not running on a 64 bit platform?");

        *self
            .colors
            .get(idx)
            .expect("Either accents had 0 colors or index out of bound")
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Neutrals(Vec<Oklch>);

impl Neutrals {
    pub fn find_closest(&self, needle: Oklch) -> Oklch {
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

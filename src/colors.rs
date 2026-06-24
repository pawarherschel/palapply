use facet::Facet;
use itertools::Itertools;
use kiddo::{ImmutableKdTree, SquaredEuclidean};
use palette::{Mix, Oklch};
use std::collections::HashMap;

const PALETTE_JSON: &str = include_str!("../palette.json");

#[derive(Facet, Debug, Clone, PartialEq)]
struct ColorSchemeCollection {
    latte: Theme,
    frappe: Theme,
    macchiato: Theme,
    mocha: Theme,
}

impl ColorSchemeCollection {
    fn from_json_str(json: &str) -> Self {
        let res = facet_json::from_str::<Self>(json);
        match res {
            Ok(scheme) => scheme,
            Err(error) => panic!("JSON decoding failed because: {error}"),
        }
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

pub(crate) fn extract_colors() -> (Accents, Neutrals) {
    println!("Extracting Colors");

    let color_scheme_collection = ColorSchemeCollection::from_json_str(PALETTE_JSON);

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
        .map(|c| Accent::Solid {
            color: c,
            final_degree: c.hue.into_degrees(),
        })
        .collect::<Vec<_>>();
    let duo_accents = solid_accents
        .iter()
        .cartesian_product(solid_accents.iter())
        .cartesian_product(1..10u8)
        .flat_map(|((a, b), layer)| match (a, b) {
            (Accent::Solid { color: a, .. }, Accent::Solid { color: b, .. }) if a != b => {
                let t = layer as f32 / 10.0;
                let final_degree = a.mix(*b, t).hue.into_degrees();
                Some(Accent::Duo {
                    from: *a,
                    factor: t,
                    to: *b,
                    final_degree,
                })
            }
            (Accent::Solid { color: a, .. }, Accent::Solid { color: b, .. }) if a == b => None,
            _ => unreachable!(),
        })
        .collect::<Vec<_>>();
    let accent_colors = solid_accents
        .into_iter()
        .chain(duo_accents.into_iter())
        .collect::<Vec<_>>();
    let hue_lut: [Accent; 360] = (-179..=180i16)
        .map(|h| {
            let hue_needle = h as f32;
            let closest = accent_colors
                .iter()
                .min_by(|a, b| {
                    let hue_a = a.final_degree();
                    let hue_b = b.get().hue.into_degrees();

                    let dist_a = spherical_hue_distance(hue_needle, hue_a);
                    let dist_b = spherical_hue_distance(hue_needle, hue_b);

                    dist_a.partial_cmp(&dist_b).unwrap_or_else(|| panic!(
                    "Encountered a NaN in Accents::find_closest? dist_a: {dist_a} dist_b: {dist_b}"
                ))
                })
                .unwrap();

            *closest
        })
        .collect_array()
        .expect("Array len != hue lut?");
    let accents = Accents {
        lut: hue_lut,
        colors: accent_colors,
    };

    let neutral_colors = neutrals
        .into_values()
        .map(|MyOklch { l, c, h }| Oklch {
            l,
            chroma: c,
            hue: palette::OklabHue::from(h),
        })
        .collect::<Vec<_>>();
    let neutrals_tree_slice = neutral_colors
        .iter()
        .map(|color| {
            let &Oklch { l, chroma, .. } = color;
            [l, chroma]
        })
        .collect::<Vec<_>>();
    let accents_tree = ImmutableKdTree::new_from_slice(&neutrals_tree_slice);
    let neutrals = Neutrals {
        tree: accents_tree,
        colors: neutral_colors,
    };
    println!("Finished generating palette");

    (accents, neutrals)
}

#[derive(Debug, Clone, PartialEq, Copy)]
pub enum Accent {
    Solid {
        color: Oklch,
        final_degree: f32,
    },
    Duo {
        from: Oklch,
        factor: f32,
        to: Oklch,
        final_degree: f32,
    },
}

impl Accent {
    fn final_degree(&self) -> f32 {
        match *self {
            Accent::Solid { final_degree, .. } => final_degree,
            Accent::Duo { final_degree, .. } => final_degree,
        }
    }
}

impl Accent {
    pub fn get(&self) -> Oklch {
        match self {
            &Accent::Solid { color: c, .. } => c,
            &Accent::Duo {
                from,
                factor: t,
                to,
                ..
            } => from.mix(to, t),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Accents {
    lut: [Accent; 360],
    colors: Vec<Accent>,
}

impl Accents {
    pub fn find_closest(&self, needle: Oklch) -> Accent {
        let hue_needle = (needle.hue.into_degrees() as usize + 180) % 360;
        *self
            .lut
            .get(hue_needle)
            .unwrap_or_else(|| panic!("Hue lookup failed with needle {needle:?}"))
    }
}

fn spherical_hue_distance(hue_a_deg: f32, hue_b_deg: f32) -> f32 {
    let diff = (hue_a_deg - hue_b_deg).abs();

    diff.min(360.0 - diff)
}

pub struct Neutrals {
    tree: ImmutableKdTree<f32, 2>,
    colors: Vec<Oklch>,
}

impl Neutrals {
    pub fn find_closest(&self, needle: Oklch) -> Oklch {
        let query_point = [needle.l, needle.chroma];

        let idx: usize = self
            .tree
            .nearest_one::<SquaredEuclidean>(&query_point)
            .item
            .try_into()
            .expect("Not running on a 64bit computer?");

        self.colors
            .get(idx)
            .expect("Either neutrals is empty or array index out of bound")
            .clone()
    }
}

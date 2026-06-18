use crate::colors::Accent;
use crate::noise::NoiseMaps;
use get::Get;
use glam::IVec2;
use image::{GenericImageView, RgbaImage};
use palette::rgb::Rgb;
use palette::{Alpha, FromColor, IntoColor, Mix, Oklch, Oklcha, Srgba};
use rayon::prelude::{IntoParallelIterator, ParallelBridge, ParallelIterator};
use std::path::PathBuf;

mod colors;
mod dim;
mod get;
mod noise;

fn main() {
    println!("Hello, world!");

    let json_path = PathBuf::from("palette.json");
    let image_path = PathBuf::from("test.png");
    let save_folder = PathBuf::from("target");

    let (accents, neutrals) = colors::extract_colors(json_path);

    let image = image::open(image_path).expect("Unable to load image");
    let (image_width, image_height) = image.dimensions();

    let pixels = image.pixels().collect::<Vec<_>>();

    let output = pixels
        // .into_iter()
        .into_par_iter()
        .map(|(x, y, image::Rgba([r, g, b, a]))| {
            (IVec2::new(x as i32, y as i32), Srgba::new(r, g, b, a))
        })
        .map(|(coord, srgba)| (coord, Srgba::<f32>::from(srgba)))
        .map(|(coord, it)| (coord, Oklcha::from_color(it)));

    let get_amount: &(dyn Get + Send + Sync) =
        &NoiseMaps::new_par(image_width, image_height, 1.0 / 10.0);

    println!("Running fragment pipeline");
    let fragment_pipeline = output
        .map(|(coord, Alpha { color, alpha })| {
            let mut neutral_base = neutrals.find_closest(color);
            neutral_base.hue = color.hue;
            let Oklch { l, chroma, hue } = neutral_base;
            (coord, Oklcha::new(l, chroma, hue, alpha))
        })
        .map(|(coord, Alpha { color, alpha })| {
            let accent_base = accents.find_closest(color);
            let hue = match accent_base {
                Accent::Solid(Oklch { hue, .. }) => hue,
                Accent::Duo { from, factor, to } => {
                    let amount = get_amount.get(factor, coord);
                    let Oklch { hue, .. } = from.mix(to, amount);
                    hue
                }
            };
            let Oklch { l, chroma, .. } = color;
            (coord, Oklcha::new(l, chroma, hue, alpha))
        });

    let image_components = fragment_pipeline
        .map(|(coord, it)| (coord, it.into_color()))
        .map(|(coord, srgba): (_, Srgba)| (coord, srgba.into_format()))
        .map(
            |(
                coord,
                Srgba {
                    color: Rgb {
                        red, green, blue, ..
                    },
                    alpha,
                },
            ): (_, Srgba<u8>)| (coord, [red, green, blue, alpha]),
        );

    let image_pixels = image_components.map(|(_, it)| it).collect::<Vec<_>>();
    let image_pixels = image_pixels.as_flattened();

    println!("Writing image");
    let image_out = RgbaImage::from_vec(image_width, image_height, Vec::from(image_pixels))
        .expect("unable to get image buffer from pixel components");
    image_out
        .save({
            let mut folder = save_folder.clone();
            folder.push("output.png");
            folder
        })
        .expect("failed to save image");

    println!("Goodbye, world!");
}

use crate::colors::Accent;
use crate::noise::NoiseMaps;
use get::Get;
use glam::IVec2;
use image::{GenericImageView, RgbaImage};
use indicatif::ProgressIterator;
use indicatif::{ParallelProgressIterator, ProgressBar, ProgressStyle};
use palette::rgb::Rgb;
use palette::{Alpha, FromColor, IntoColor, Mix, Oklch, Oklcha, Srgba};
use rayon::prelude::{IntoParallelIterator, ParallelBridge, ParallelIterator};
use std::path::PathBuf;
use std::{fs, time};

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
    let pixels_len = pixels.len();

    let output = pixels
        .into_par_iter()
        .map(|(x, y, image::Rgba([r, g, b, a]))| {
            let coord = IVec2::new(x as i32, y as i32);
            (coord, Srgba::new(r, g, b, a))
        })
        .map(|(coord, srgba)| (coord, Srgba::<f32>::from(srgba)))
        .map(|(coord, it)| (coord, Oklcha::from_color(it)));

    let get_amount: &(dyn Get + Send + Sync) = if (0..10)
        .map(|layer| format!("target/maps/{}.png", layer))
        .map(|file_name| fs::File::open(file_name))
        .map(|file| file.map(|file| file.metadata()).flatten())
        .all(|metadata| metadata.is_ok())
    {
        &NoiseMaps::new_cached(image_width, image_height, 1.0 / 10.0)
    } else {
        &NoiseMaps::new_par(image_width, image_height, 1.0 / 10.0)
    };

    println!("Running fragment pipeline");

    let style = ProgressStyle::with_template(
        "{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {percent}% (ETA: {eta})",
    )
    .unwrap()
    .progress_chars("#>-");
    let bar = ProgressBar::new(pixels_len as u64);
    bar.set_style(style);
    bar.enable_steady_tick(time::Duration::from_millis(100));

    let fragment_pipeline = output
        .progress_with(bar)
        .map(|(coord, Alpha { color, alpha })| {
            let mut neutral_base = neutrals.find_closest(color);
            neutral_base.hue = color.hue;
            let Oklch { l, chroma, hue } = neutral_base;
            (coord, Oklcha::new(l, chroma, hue, alpha))
        })
        .map(|(coord, Alpha { color, alpha })| {
            let accent_base = accents.find_closest(color);
            let hue = match accent_base {
                Accent::Solid { final_degree, .. } => final_degree,
                Accent::Duo {
                    from, factor, to, ..
                } => {
                    let amount = get_amount.get(factor, coord);
                    let Oklch { hue, .. } = from.mix(to, amount);
                    hue.into_degrees()
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

    let image_pixels = image_components.map(|(_coord, it)| it).collect::<Vec<_>>();
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

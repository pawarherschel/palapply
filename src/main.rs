use crate::colors::Accent;
use crate::noise::NoiseMaps;
use get::Get;
use glam::IVec2;
use image::{GenericImageView, RgbaImage};
use indicatif::{ParallelProgressIterator, ProgressBar, ProgressStyle};
use palette::rgb::Rgb;
use palette::{Alpha, FromColor, IntoColor, Mix, Oklch, Oklcha, Srgba};
use rayon::prelude::{IntoParallelIterator, ParallelIterator};
use clap::Parser;
use std::path::{Path, PathBuf};
use std::{fs, time};

mod colors;
mod dim;
mod get;
mod noise;

#[derive(Parser)]
#[command(version, about = "Catppuccin-themed image posterizer")]
struct Cli {
    #[arg(short, long)]
    input: PathBuf,
    #[arg(short, long)]
    output: PathBuf,
    #[arg(long)]
    maps_dir: Option<PathBuf>,
    #[arg(long, num_args = 2, value_names = ["WIDTH", "HEIGHT"])]
    genmaps_only: Option<Vec<u32>>,
}

fn main() {
    let cli = Cli::parse();

    let (accents, neutrals) = colors::extract_colors();

    let image = image::open(&cli.input).expect("Unable to load image");
    let (image_width, image_height) = image.dimensions();

    let pixels = image.pixels().collect::<Vec<_>>();
    let pixels_len = pixels.len();

    let noise_scale = 1.0 / 1.0;

    let maps_folder = cli.maps_dir.unwrap_or_else(|| {
        cli.output.parent()
            .unwrap_or_else(|| Path::new("."))
            .join("maps")
    });

    if let Some(ref dims) = cli.genmaps_only {
        NoiseMaps::new_par(dims[0], dims[1], noise_scale, &maps_folder);
        return;
    }

    let output = pixels
        .into_par_iter()
        .map(|(x, y, image::Rgba([r, g, b, a]))| {
            let coord = IVec2::new(x as i32, y as i32);
            (coord, Srgba::new(r, g, b, a))
        })
        .map(|(coord, srgba)| (coord, Srgba::<f32>::from(srgba)))
        .map(|(coord, it)| (coord, Oklcha::from_color(it)));

    let get_amount: &(dyn Get + Send + Sync) = if (0..10)
        .map(|layer| maps_folder.join(format!("{}.png", layer)))
        .map(|file_name| fs::File::open(file_name))
        .map(|file| file.map(|file| file.metadata()).flatten())
        .all(|metadata| metadata.is_ok())
    {
        &NoiseMaps::new_cached(image_width, image_height, noise_scale, &maps_folder)
    } else {
        &NoiseMaps::new_par(image_width, image_height, noise_scale, &maps_folder)
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
        .save(&cli.output)
        .expect("failed to save image");
}

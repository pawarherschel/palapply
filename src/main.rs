use image::{GenericImageView, RgbaImage};
use palette::rgb::Rgb;
use palette::{Alpha, FromColor, IntoColor, Oklch, Oklcha, Srgba};
use rayon::prelude::{IntoParallelIterator, ParallelBridge, ParallelIterator};
use std::path::PathBuf;

mod colors;

fn main() {
    println!("Hello, world!");

    let json_path = PathBuf::from("palette.json");
    let image_path = PathBuf::from("test.png");
    let save_folder = PathBuf::from("target");

    let (_accents, neutrals) = colors::extract_colors(json_path);

    let image = image::open(image_path).expect("Unable to load image");
    let (image_width, image_height) = image.dimensions();

    let pixels = image.pixels().collect::<Vec<_>>();

    let output = pixels
        .into_par_iter()
        .map(|(x, y, image::Rgba([r, g, b, a]))| Srgba::new(r, g, b, a))
        .map(|srgba| Srgba::<f32>::from(srgba))
        .map(|it| Oklcha::from_color(it))
        .map(|Alpha { color, alpha }| {
            let mut neutral_base = neutrals.find_closest(color);
            neutral_base.hue = color.hue;
            let Oklch { l, chroma, hue } = neutral_base;
            Oklcha::new(l, chroma, hue, alpha)
        });

    let image_components = output
        .map(|it| it.into_color())
        .map(|srgba: Srgba| srgba.into_format())
        .flat_map(
            |Srgba {
                 color: Rgb {
                     red, green, blue, ..
                 },
                 alpha,
             }: Srgba<u8>| [red, green, blue, alpha],
        );

    let image_pixels = image_components.collect();

    println!("Writing image");
    let image_out = RgbaImage::from_vec(image_width, image_height, image_pixels)
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

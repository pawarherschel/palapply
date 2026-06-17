use image::{GenericImageView, RgbaImage};
use palette::encoding::Srgb;
use palette::rgb::{Rgb, Rgba};
use palette::{Alpha, FromColor, IntoColor, Oklch, Oklcha, Srgba};
use std::path::PathBuf;

mod colors;

fn main() {
    println!("Hello, world!");

    let json_path = PathBuf::from("palette.json");
    let image_path = PathBuf::from("test.png");
    let save_folder = PathBuf::from("target");

    println!("Extracting colors");
    let (accents, neutrals) = colors::extract_colors(json_path);
    // let palette = accents
    //     .0
    //     .into_iter()
    //     .map(|colors::Oklch { l, c, h }: colors::Oklch| Oklch::from_components((l, c, h)))
    //     .cartesian_product(
    //         neutrals
    //             .0
    //             .into_iter()
    //             .map(|colors::Oklch { l, c, h }: colors::Oklch| Oklch::from_components((l, c, h))),
    //     )
    //     .map(|(accent, neutral)| {});

    println!("Reading image");
    let image = image::open(image_path).expect("Unable to load image");
    let (image_width, image_height) = image.dimensions();

    println!("Applying transforms");
    let output = image
        .pixels()
        .map(|(x, y, image::Rgba([r, g, b, a]))| Srgba::new(r, g, b, a))
        .map(|srgba| Srgba::<f32>::from(srgba))
        .map(|it| Oklcha::from_color(it))
        .collect::<Vec<_>>();

    println!("Creating final image");
    let image_components = output
        .into_iter()
        .map(|it| it.into_color())
        .map(|srgba: Srgba| srgba.into_format())
        .flat_map(
            |Srgba {
                 color: Rgb {
                     red, green, blue, ..
                 },
                 alpha,
             }: Srgba<u8>| [red, green, blue, alpha],
        )
        .collect();

    println!("Writing image");
    let image_out = RgbaImage::from_vec(image_width, image_height, image_components)
        .expect("unable to get image buffer from pixel components");
    image_out
        .save({
            let mut folder = save_folder.clone();
            folder.push("output.png");
            folder
        })
        .expect("failed to save image");
}

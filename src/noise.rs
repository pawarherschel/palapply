use crate::dim::{Dim1, Dim2};
use crate::get::Get;
use bluenoise::WrappingBlueNoise;
use glam::{IVec2, Vec2};
use image::{ImageBuffer, Luma};
use itertools::Itertools;
use rand_pcg::Pcg64Mcg;
use rand_pcg::rand_core::SeedableRng;
use rayon::iter::ParallelIterator;
use rayon::prelude::IntoParallelIterator;
use std::collections::HashSet;
use std::f32::consts;
use std::fs;
use std::ops::Index;
use std::path::Path;
use std::sync::atomic::{AtomicU8, Ordering};

pub struct NoiseMaps {
    height: f32,
    width: f32,
    layers: Vec<Vec<bool>>,
    fraction_factor: f32,
}

impl NoiseMaps {
    pub fn new_par(width: u32, height: u32, fraction_factor: f32) -> Self {
        println!("Generating noise maps");

        let w = width as f32 * fraction_factor;
        let h = height as f32 * fraction_factor;

        let mut sets = Vec::with_capacity(10);
        let generated = AtomicU8::new(0);

        let black = HashSet::new();
        let white = (0..w as u32)
            .cartesian_product(0..h as u32)
            .map(|(x, y)| IVec2::new(x as i32, y as i32))
            .collect::<HashSet<_>>();

        sets.push(black);
        generated.fetch_add(1, Ordering::Relaxed);
        println!(
            "Completed generating map {} of 10",
            generated.load(Ordering::Relaxed)
        );

        fn map_range(value: f32, in_min: f32, in_max: f32, out_min: f32, out_max: f32) -> f32 {
            out_min + (value - in_min) * (out_max - out_min) / (in_max - in_min)
        }
        let custom_layers = (1..9u8)
            .into_par_iter()
            .map(|layer| {
                let layer_f = f32::from(layer);
                let fraction = map_range(layer_f, 0.0, 9.0, 0.0, 1.0);

                let rng = Pcg64Mcg::seed_from_u64(67);
                let no_of_samples = w.min(h) * fraction;

                let min_radius = calculate_min_radius(fraction, w, h);
                let (min_radius, invert) = if min_radius < 1.0 {
                    println!("min_radius < 1.0, inverting");
                    let fraction = 1.0 - fraction;
                    (calculate_min_radius(fraction, w, h), true)
                } else {
                    (min_radius, false)
                };

                let mut generator = WrappingBlueNoise::from_rng(w, h, min_radius, rng);
                let generator = generator.with_samples(no_of_samples as u32);

                let points = generator
                    .map(|Vec2 { x, y }| IVec2::new(x as i32, y as i32))
                    .collect::<HashSet<_>>();

                let ret = if invert {
                    white.clone().difference(&points).cloned().collect()
                } else {
                    points
                };
                generated.fetch_add(1, Ordering::Relaxed);
                println!(
                    "Completed generating map {} of 10",
                    generated.load(Ordering::Relaxed)
                );
                ret
            })
            .collect::<Vec<_>>();
        sets.extend(custom_layers);

        sets.push(white);
        generated.fetch_add(1, Ordering::Relaxed);
        println!(
            "Completed generating map {} of 10",
            generated.load(Ordering::Relaxed)
        );

        for (idx, map) in sets.iter().enumerate() {
            let mut imgbuf = ImageBuffer::from_pixel(w as u32, h as u32, Luma([0u8]));

            for coord in map {
                let x = coord.x as u32 % imgbuf.width();
                let y = coord.y as u32 % imgbuf.height();

                imgbuf.put_pixel(x, y, Luma([255u8]));
            }

            let dir_path = Path::new("target/maps");
            fs::create_dir_all(dir_path).expect("Failed to create target/maps directory");

            let file_path = dir_path.join(format!("{}.png", idx));
            imgbuf
                .save(&file_path)
                .expect("Failed to save noise map image asset");
        }

        let layers = sets
            .into_iter()
            .map(|set| {
                (0..w as u32)
                    .cartesian_product(0..h as u32)
                    .map(|(x, y)| IVec2::new(x as i32, y as i32))
                    .map(|coord| set.contains(&coord))
                    .collect()
            })
            .collect();

        Self {
            width: w,
            height: h,
            layers,
            fraction_factor,
        }
    }

    pub fn get_layer(&self, layer: u8) -> &Vec<bool> {
        self.layers
            .get(layer as usize)
            .expect("Layer not within the range [0, 10)")
    }
}

impl Get for NoiseMaps {
    fn get(&self, factor: f32, coord: IVec2) -> f32 {
        let IVec2 { x, y } = coord;
        let full_width = self.width / self.fraction_factor;
        let full_height = self.height / self.fraction_factor;
        let coord = IVec2::new(
            (x as f32 % full_width) as i32,
            (y as f32 % full_height) as i32,
        );
        if self
            .get_layer((factor * 10.0).floor() as u8)
            .get((coord.y as f32 * self.width + coord.x as f32).floor() as usize)
            .is_some()
        {
            1.0
        } else {
            0.0
        }
    }
}

fn calculate_min_radius(fraction: f32, width: f32, height: f32) -> f32 {
    let (height, width) = (Dim1(height), Dim1(width));
    let rectangle_area = |h: Dim1, w: Dim1| h * w;
    let radius_of_circle_from_area = |area: Dim2| Dim1((area * consts::FRAC_1_PI).0.sqrt());

    let side_len = Dim1(height.0.min(width.0));
    let square_canvas_area = rectangle_area(side_len, side_len);

    let unit_len = side_len * fraction;
    let unit_square_area = rectangle_area(unit_len, unit_len);

    let balanced_area = (square_canvas_area / unit_square_area) * Dim2(1.0);

    let unit_circle_radius = radius_of_circle_from_area(balanced_area);

    let ret = unit_circle_radius.0;

    ret
}

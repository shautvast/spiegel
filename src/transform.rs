use std::{error::Error, result::Result};

use image::{GenericImageView, ImageBuffer, Pixel, Rgb, RgbImage};
use imageproc::point::Point;

use crate::{quantizer, samples};
use crate::samples::SAMPLES;
use crate::samples::ColorSample;

pub fn apply(src: &RgbImage) -> Result<RgbImage, Box<dyn Error>> {
    let gauss = imageproc::filter::gaussian_blur_f32(&src, 2.0);
    let median = imageproc::filter::median_filter(&gauss, 2, 2);
    let quantized = quantizer::quantize(&median, 256);
    let out = apply_samples_to_image(quantized);

    Ok(out)
}


fn apply_samples_to_image(mut src: RgbImage) -> RgbImage {
    let mut imgbuf = RgbImage::new(src.width(), src.height());
    unsafe {
        for y in 0..src.height() {
            for x in 0..src.width() {
                let pixel = &src.unsafe_get_pixel(x, y);
                if imgbuf.unsafe_get_pixel(x, y).channels() == [0, 0, 0] {
                    if let Some(sample) = get_closest(pixel) {
                        fill(&mut src, sample, &mut imgbuf, pixel, x, y);
                    }
                }
            }
        }
    }
    imgbuf
}

fn fill(
    src: &mut ImageBuffer<Rgb<u8>, Vec<u8>>,
    sample: &ColorSample,
    dest: &mut ImageBuffer<Rgb<u8>, Vec<u8>>,
    color: &Rgb<u8>,
    px: u32,
    py: u32,
) {
    if color.channels() == [0, 0, 0] {
        return;
    }
    let height = sample.image.height();
    let width = sample.image.width();
    let mut points = List::new();
    if is_same(src.get_pixel(px, py), &color) {
        points.push(Point { x: px, y: py });
    }

    while !points.is_empty() {
        if let Some(point) = points.pop() {
            let orig_pixel = src.get_pixel(point.x, point.y);
            let x = point.x;
            let y = point.y;
            if src.get_pixel(x, y).channels() != [0, 0, 0] {
                if is_same(orig_pixel, &color) {
                    let mut xx = x;
                    let mut yy = y;
                    while xx >= width {
                        xx -= width;
                    }
                    while yy >= height {
                        yy -= height;
                    }
                    dest.put_pixel(x, y, *sample.image.get_pixel(xx, yy));
                    src.put_pixel(x, y, Rgb([0, 0, 0]));
                    if x > 1 {
                        points.push(Point::new(x - 1, y));
                    }
                    if y > 1 {
                        points.push(Point::new(x, y - 1));
                    }
                    if x < src.width() - 1 {
                        points.push(Point::new(x + 1, y));
                    }
                    if y < src.height() - 1 {
                        points.push(Point::new(x, y + 1));
                    }
                }
            }
        } else {
            println!("break");
            break;
        }
    }
}

fn is_same(p1: &Rgb<u8>, p2: &Rgb<u8>) -> bool {
    let p1 = p1.channels();
    let p2 = p2.channels();
    i16::abs(p1[0] as i16 - p2[0] as i16) < 4
        && i16::abs(p1[1] as i16 - p2[1] as i16) < 4
        && i16::abs(p1[2] as i16 - p2[2] as i16) < 4
}

fn get_closest(
    pixel: &Rgb<u8>,
) -> Option<&'static ColorSample> {
    let mut closest = None;
    let mut min_diff: f32 = 4294967295.0; //0xFFFFFFFF
    for sample in SAMPLES.iter() {
        let r = hex::decode(&sample[0..2]).unwrap()[0];
        let g = hex::decode(&sample[2..4]).unwrap()[0];
        let b = hex::decode(&sample[4..6]).unwrap()[0];
        let diff = get_distance(r, g, b, pixel);
        if diff < min_diff {
            closest = Some(sample);
            min_diff = diff;
        }
    }
    if let Some(closest) = closest {
        if samples::contains(closest) {
            return samples::get(closest);
        } else {
            crate::app::add_sample( closest);
            // wait for image to ownload
            while !samples::contains(closest) {
                //...
            }
            return samples::get(closest);
        }
    }
    None
}

fn get_distance(r: u8, g: u8, b: u8, c2: &Rgb<u8>) -> f32 {
    let red_dif = r as f32 - c2.channels()[0] as f32;
    let green_dif = g as f32 - c2.channels()[1] as f32;
    let blue_dif = b as f32 - c2.channels()[2] as f32;
    return f32::sqrt(red_dif * red_dif + green_dif * green_dif + blue_dif * blue_dif);
}

impl PartialEq for ColorSample {
    fn eq(&self, other: &Self) -> bool {
        self.r == other.r && self.g == other.g && self.b == other.b
    }

    fn ne(&self, other: &Self) -> bool {
        !self.eq(other)
    }
}

#[derive(Debug)]
struct List {
    head: Option<Box<Node>>,
}

impl List {
    fn new() -> Self {
        Self { head: None }
    }
    fn push(&mut self, point: Point<u32>) {
        let new_node = Box::new(Node {
            value: point,
            next: self.head.take(),
        });

        self.head = Some(new_node);
    }

    fn pop(&mut self) -> Option<Point<u32>> {
        self.head.take().map(|node| {
            self.head = node.next;
            node.value
        })
    }

    fn is_empty(&self) -> bool {
        self.head.is_none()
    }
}

#[derive(Debug)]
struct Node {
    value: Point<u32>,
    next: Option<Box<Node>>,
}


#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test() {
        let mut list = List::new();

        list.push(Point::new(1, 1));
        list.push(Point::new(2, 2));
        assert_eq!(2, list.pop().unwrap().x);
        assert_eq!(1, list.pop().unwrap().x);
    }
}


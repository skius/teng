use std::fmt;
use std::fmt::{Debug, Formatter};
use std::ops::{Index, IndexMut};

pub struct Display<T> {
    width: usize,
    height: usize,
    default: T,
    pixels: Vec<T>,
}

impl<T: Debug> Debug for Display<T> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "Display {{ width: {}, height: {}, pixels: {:?} }}", self.width, self.height, self.pixels)
    }
}

impl<T: Clone> Display<T> {
    pub fn new(width: usize, height: usize, default: T) -> Self {
        Self {
            width,
            height,
            default: default.clone(),
            pixels: vec![default; width * height],
        }
    }

    pub fn clear(&mut self) {
        for pixel in self.pixels.iter_mut() {
            *pixel = self.default.clone();
        }
    }

    pub fn fill(&mut self, value: T) {
        for pixel in self.pixels.iter_mut() {
            *pixel = value.clone();
        }
    }

    pub fn resize_discard(&mut self, width: usize, height: usize) {
        self.pixels.resize(width * height, self.default.clone());
        self.width = width;
        self.height = height;
    }

    pub fn resize_keep(&mut self, width: usize, height: usize) {
        let mut new_pixels = vec![self.default.clone(); width * height];
        let min_width = self.width.min(width);
        for y in 0..self.height.min(height) {
            let src_start = y * self.width;
            let dst_start = y * width;
            let src_end = src_start + min_width;
            let dst_end = dst_start + min_width;
            new_pixels[dst_start..dst_end].clone_from_slice(&self.pixels[src_start..src_end]);
        }
        self.pixels = new_pixels;
        self.width = width;
        self.height = height;
    }
}

impl<T> Display<T> {
    fn get_index(&self, x: usize, y: usize) -> usize {
        y * self.width + x
    }

    pub fn height(&self) -> usize {
        self.height
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn get(&self, x: usize, y: usize) -> Option<&T> {
        self.pixels.get(self.get_index(x, y))
    }

    pub fn get_mut(&mut self, x: usize, y: usize) -> Option<&mut T> {
        let idx = self.get_index(x, y);
        self.pixels.get_mut(idx)
    }

    pub fn set(&mut self, x: usize, y: usize, value: T) {
        if let Some(pixel) = self.get_mut(x, y) {
            *pixel = value;
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = (usize, usize, &T)> {
        self.pixels.iter().enumerate().map(|(idx, pixel)| {
            let x = idx % self.width;
            let y = idx / self.width;
            (x, y, pixel)
        })
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = (usize, usize, &mut T)> {
        self.pixels.iter_mut().enumerate().map(|(idx, pixel)| {
            let x = idx % self.width;
            let y = idx / self.width;
            (x, y, pixel)
        })
    }
}

impl<T> Index<(usize, usize)> for Display<T> {
    type Output = T;

    fn index(&self, (x, y): (usize, usize)) -> &Self::Output {
        &self.pixels[self.get_index(x, y)]
    }
}

impl<T> IndexMut<(usize, usize)> for Display<T> {
    fn index_mut(&mut self, (x, y): (usize, usize)) -> &mut Self::Output {
        let idx = self.get_index(x, y);
        &mut self.pixels[idx]
    }
}

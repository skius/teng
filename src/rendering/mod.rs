//! Rendering module for terminal-based output.
//!
//! This module provides the core rendering components for the `teng` library,
//! enabling you to display graphics and text in a terminal environment.
//!
//! **Sub-modules:**
//!
//! *   [`color`](crate::rendering::color): Defines the [`Color`] enum for specifying colors.
//! *   [`display`](crate::rendering::display): Defines the [`Display`] struct, a 2D pixel buffer.
//! *   [`pixel`](crate::rendering::pixel): Defines the [`Pixel`] struct, the basic unit of rendering.
//! *   [`render`](crate::rendering::render): Provides the [`Render`] trait for objects that can be rendered.
//! *   [`renderer`](crate::rendering::renderer): Defines the [`Renderer`] trait and implementations for rendering to the terminal.
//!
//! **Key Concepts:**
//!
//! *   **Pixels:**  The fundamental unit of rendering, represented by the [`Pixel`] struct.  Pixels define a character and its foreground and background colors.
//! *   **Display Buffer:** The [`Display`] struct is a 2D grid of pixels that acts as an in-memory representation of the terminal display.
//! *   **Renderer:** The [`Renderer`] trait defines the interface for rendering operations.  [`DisplayRenderer`] is a concrete implementation that renders to the terminal using `crossterm`.
//! *   **Renderable Objects:** Anything that implements the [`Render`] trait can be drawn to the display using a `Renderer`.  This includes strings, characters, pixels, and sprites.
//!
//! **Rendering Process (Simplified):**
//!
//! 1.  Create a `DisplayRenderer` (which manages the terminal output).
//! 2.  Get a mutable reference to the `Renderer` within your `Component::render()` method.
//! 3.  Use methods like `Renderer::render_pixel()` and `Render::render()` to draw pixels and renderable objects to the `Renderer`.
//! 4.  The `Renderer` updates its internal `Display` buffer.
//! 5.  Call `Renderer::flush()` to write the contents of the `Display` buffer to the terminal, efficiently updating only the changed pixels.

pub mod color;
pub mod display;
pub mod pixel;
pub mod render;
pub mod renderer;

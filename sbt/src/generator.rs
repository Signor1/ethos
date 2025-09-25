use alloc::string::String;
use core::fmt::Write;
use stylus_sdk::alloy_primitives::FixedBytes;

use crate::base64::base64_encode;

const SVG_WIDTH: i32 = 1000;
const SVG_HEIGHT: i32 = 1000;
const BACKGROUND_COLOR: &str = "#1a1a1a";

// Hexagon center
const CENTER_X: i32 = 500;
const CENTER_Y: i32 = 500;

// Hexagon parameters
const MIN_SIZE: usize = 150;
const MAX_SIZE: usize = 250;
const MIN_STROKE_WIDTH: usize = 12;
const MAX_STROKE_WIDTH: usize = 24;

// Color palette
const COLORS: &[&str] = &[
    "#1BA3E8", // Arbitrum blue
    "#FF6B35", // Orange
    "#4ECDC4", // Teal
    "#45B7D1", // Light blue
    "#96CEB4", // Mint
    "#FECA57", // Yellow
    "#FF9FF3", // Pink
    "#54A0FF", // Blue
    "#5F27CD", // Purple
    "#00D2D3", // Cyan
];

pub struct SBTGenerator {
    seed: FixedBytes<32>,
}

impl SBTGenerator {
    pub fn new(seed: FixedBytes<32>) -> Self {
        Self { seed }
    }

    // Main function that generates the complete metadata
    pub fn metadata(&self) -> String {
        let svg = self.svg();
        let base64_svg = base64_encode(&svg);

        let metadata = format!(
            r#"{{"name":"Ethos SBT","description":"Ethos SBT on Arbitrum","image":"data:image/svg+xml;base64,{}"}}"#,
            base64_svg
        );
        let base64_metadata = base64_encode(&metadata);

        format!(r#"data:application/json;base64,{}"#, base64_metadata)
    }

    fn svg(&self) -> String {
        let size = self.map_byte(self.seed[0], MIN_SIZE, MAX_SIZE) as i32;
        let stroke_width = self.map_byte(self.seed[1], MIN_STROKE_WIDTH, MAX_STROKE_WIDTH) as i32;
        let color_index = (self.seed[2] as usize) % COLORS.len();
        let color = COLORS[color_index];

        let mut svg = String::new();

        // SVG header
        write!(
                svg,
                r#"<svg width="{}" height="{}" viewBox="0 0 {} {}" xmlns="http://www.w3.org/2000/svg">"#,
                SVG_WIDTH, SVG_HEIGHT, SVG_WIDTH, SVG_HEIGHT
            ).unwrap();

        // Background
        write!(
            svg,
            r#"<rect width="100%" height="100%" fill="{}"/>"#,
            BACKGROUND_COLOR
        )
        .unwrap();

        // Generate main hexagon path
        let hexagon_path = self.generate_hexagon_path(CENTER_X, CENTER_Y, size);

        // Main hexagon
        write!(
                svg,
                r#"<path d="{}" fill="none" stroke="{}" stroke-width="{}" stroke-linejoin="round" stroke-linecap="round"/>"#,
                hexagon_path, color, stroke_width
            ).unwrap();

        // Optional: Add inner hexagon for more visual interest
        if size > 180 {
            // Only add if main hexagon is large enough
            let inner_size = size - 40;
            let inner_stroke = stroke_width / 2;
            let inner_path = self.generate_hexagon_path(CENTER_X, CENTER_Y, inner_size);

            write!(
                    svg,
                    r#"<path d="{}" fill="none" stroke="{}" stroke-width="{}" stroke-linejoin="round" stroke-linecap="round" opacity="0.6"/>"#,
                    inner_path, color, inner_stroke
                ).unwrap();
        }

        write!(svg, r#"</svg>"#).unwrap();

        svg
    }

    // Generate hexagon path using integer math only
    fn generate_hexagon_path(&self, cx: i32, cy: i32, size: i32) -> String {
        // Pre-calculated hexagon points using integer approximations
        // Avoiding floating point trigonometry
        let points = [
            (cx + size, cy),                         // Right
            (cx + size / 2, cy + (size * 87) / 100), // Bottom-right (87/100 ≈ sin(60°))
            (cx - size / 2, cy + (size * 87) / 100), // Bottom-left
            (cx - size, cy),                         // Left
            (cx - size / 2, cy - (size * 87) / 100), // Top-left
            (cx + size / 2, cy - (size * 87) / 100), // Top-right
        ];

        let mut path = String::new();

        // Move to first point
        write!(path, "M {} {}", points[0].0, points[0].1).unwrap();

        // Line to all other points
        for point in points.iter().skip(1) {
            write!(path, " L {} {}", point.0, point.1).unwrap();
        }

        // Close the path
        write!(path, " Z").unwrap();

        path
    }

    fn map_byte(&self, byte: u8, min: usize, max: usize) -> usize {
        min + ((byte as usize * (max - min)) / 255)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hexagon_generation() {
        let seed = FixedBytes::<32>::random();
        let generator = SBTGenerator::new(seed);
        let svg = generator.svg();

        // Basic checks
        assert!(svg.contains("<path"));
        assert!(svg.contains("stroke="));
        assert!(svg.contains("500"));
    }

    #[test]
    fn test_hexagon_centered() {
        let seed = FixedBytes::<32>::from([0u8; 32]);
        let generator = SBTGenerator::new(seed);
        let path = generator.generate_hexagon_path(500, 500, 150);

        // Should contain coordinates around center (500, 500)
        assert!(path.contains("500"));
        assert!(path.starts_with("M "));
        assert!(path.contains(" L "));
        assert!(path.ends_with(" Z"));
    }

    #[test]
    fn test_proper_sizing() {
        let seed = FixedBytes::<32>::from([255u8; 32]); // Max values
        let generator = SBTGenerator::new(seed);
        let svg = generator.svg();

        // Should use larger stroke widths
        assert!(svg.contains("stroke-width=\"24\"") || svg.contains("stroke-width=\"12\""));

        // Should not contain tiny values
        assert!(!svg.contains("stroke-width=\"4\""));
        assert!(!svg.contains("stroke-width=\"1\""));
    }

    #[test]
    fn test_no_floating_point() {
        let seed = FixedBytes::<32>::from([128u8; 32]);
        let generator = SBTGenerator::new(seed);
        let svg = generator.svg();

        // Should not contain any decimal points (indicating floating point)
        assert!(!svg.contains(".0"));
        assert!(!svg.contains(".5"));
    }
}

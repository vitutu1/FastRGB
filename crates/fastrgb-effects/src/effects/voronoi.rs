//! `Voronoi` cellular tessellation effect.

use fastrgb_core::{CanvasFrame, Hsl, ParameterDescriptor, Rgb};

use crate::rng::FastRng;
use crate::traits::{Effect, EffectCategory, EffectContext, EffectMetadata};

struct VoronoiPalette {
    name: &'static str,
    colors: [(f32, f32, f32); 6],
}

const PALETTES: &[VoronoiPalette] = &[
    VoronoiPalette {
        name: "Beach",
        colors: [
            (203.0, 68.0, 84.0),
            (186.0, 94.0, 44.0),
            (180.0, 81.0, 73.0),
            (45.0, 47.0, 44.0),
            (28.0, 67.0, 85.0),
            (151.0, 94.0, 75.0),
        ],
    },
    VoronoiPalette {
        name: "Contemporary",
        colors: [
            (210.0, 76.0, 23.0),
            (43.0, 100.0, 50.0),
            (195.0, 57.0, 53.0),
            (9.0, 82.0, 57.0),
            (22.0, 81.0, 78.0),
            (178.0, 45.0, 53.0),
        ],
    },
    VoronoiPalette {
        name: "Dark Carnival",
        colors: [
            (355.0, 75.0, 19.0),
            (343.0, 81.0, 23.0),
            (355.0, 100.0, 53.0),
            (206.0, 27.0, 15.0),
            (186.0, 23.0, 18.0),
            (172.0, 39.0, 26.0),
        ],
    },
    VoronoiPalette {
        name: "Neon",
        colors: [
            (186.0, 82.0, 42.0),
            (331.0, 66.0, 45.0),
            (38.0, 72.0, 62.0),
            (216.0, 64.0, 44.0),
            (228.0, 38.0, 60.0),
            (166.0, 37.0, 41.0),
        ],
    },
    VoronoiPalette {
        name: "Ocean",
        colors: [
            (193.0, 44.0, 17.0),
            (195.0, 31.0, 52.0),
            (194.0, 42.0, 30.0),
            (197.0, 45.0, 45.0),
            (193.0, 40.0, 70.0),
            (190.0, 50.0, 90.0),
        ],
    },
    VoronoiPalette {
        name: "Outrun",
        colors: [
            (286.0, 46.0, 22.0),
            (340.0, 67.0, 59.0),
            (49.0, 78.0, 62.0),
            (30.0, 78.0, 56.0),
            (227.0, 51.0, 56.0),
            (224.0, 77.0, 70.0),
        ],
    },
    VoronoiPalette {
        name: "Space",
        colors: [
            (293.0, 84.0, 90.0),
            (271.0, 71.0, 36.0),
            (280.0, 81.0, 18.0),
            (269.0, 70.0, 26.0),
            (230.0, 42.0, 41.0),
            (218.0, 100.0, 47.0),
        ],
    },
    VoronoiPalette {
        name: "Vaporwave",
        colors: [
            (271.0, 89.0, 64.0),
            (242.0, 80.0, 75.0),
            (185.0, 41.0, 57.0),
            (298.0, 100.0, 71.0),
            (299.0, 93.0, 78.0),
            (49.0, 85.0, 65.0),
        ],
    },
];

#[derive(Debug, Clone)]
struct Site {
    x: f32,
    y: f32,
    dx: f32,
    dy: f32,
    color_idx: usize,
    rand_hue: f32,
}

/// Animated Voronoi diagram with multiple distance metrics and FastRGB color themes.
#[derive(Debug)]
pub struct VoronoiEffect {
    sites: Vec<Site>,
    rng: FastRng,
}

impl Default for VoronoiEffect {
    fn default() -> Self {
        Self {
            sites: Vec::new(),
            rng: FastRng::seeded(0x5090_401D),
        }
    }
}

impl VoronoiEffect {
    /// Creates a new [`VoronoiEffect`].
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    fn spawn_site(&mut self) -> Site {
        Site {
            x: self.rng.range_f32(0.0, 320.0),
            y: self.rng.range_f32(0.0, 200.0),
            dx: self.rng.range_f32(-1.0, 1.0),
            dy: self.rng.range_f32(-1.0, 1.0),
            color_idx: self.rng.gen_range_usize(12),
            rand_hue: self.rng.range_f32(0.0, 360.0),
        }
    }
}

impl Effect for VoronoiEffect {
    fn metadata(&self) -> EffectMetadata {
        EffectMetadata {
            id: "voronoi",
            name: "Voronoi",
            description: "Drifting cellular Voronoi tessellation with Euclidean, Manhattan, and Chebyshev metrics.",
            publisher: "Derek Huber",
            category: EffectCategory::Basic,
            uses_screen: false,
        }
    }

    fn descriptors(&self) -> Vec<ParameterDescriptor> {
        vec![
            ParameterDescriptor::combobox(
                "colorMode",
                "Color Mode",
                vec![
                    "Dark Carnival".to_owned(),
                    "Beach".to_owned(),
                    "Contemporary".to_owned(),
                    "Neon".to_owned(),
                    "Ocean".to_owned(),
                    "Outrun".to_owned(),
                    "Space".to_owned(),
                    "Vaporwave".to_owned(),
                    "Rainbow".to_owned(),
                    "Random".to_owned(),
                    "4-Custom Colors".to_owned(),
                ],
                "Dark Carnival",
            ),
            ParameterDescriptor::color("color1", "Custom Color 1", Rgb::new(0xFF, 0x00, 0x00)),
            ParameterDescriptor::color("color2", "Custom Color 2", Rgb::new(0x00, 0x01, 0xFF)),
            ParameterDescriptor::color("color3", "Custom Color 3", Rgb::new(0x04, 0xFF, 0x00)),
            ParameterDescriptor::color("color4", "Custom Color 4", Rgb::new(0x9B, 0x00, 0xFF)),
            ParameterDescriptor::combobox(
                "distanceFormula",
                "Distance Formula",
                vec![
                    "Euclidean".to_owned(),
                    "Manhattan".to_owned(),
                    "Chebyshev".to_owned(),
                ],
                "Euclidean",
            ),
            ParameterDescriptor::number("siteCount", "Site Count", 2.0, 50.0, 1.0, 20.0),
            ParameterDescriptor::number("speed", "Speed", 1.0, 5.0, 1.0, 2.0),
            ParameterDescriptor::boolean("drawSites", "Draw Sites", false),
            ParameterDescriptor::color("siteColor", "Site Color", Rgb::BLACK),
        ]
    }

    fn reset(&mut self) {
        self.sites.clear();
    }

    fn render(&mut self, ctx: &EffectContext<'_>, canvas: &mut CanvasFrame) {
        let color_mode = ctx.params.str_or("colorMode", "Dark Carnival");
        let custom = [
            ctx.params.color_or("color1", Rgb::new(0xFF, 0x00, 0x00)),
            ctx.params.color_or("color2", Rgb::new(0x00, 0x01, 0xFF)),
            ctx.params.color_or("color3", Rgb::new(0x04, 0xFF, 0x00)),
            ctx.params.color_or("color4", Rgb::new(0x9B, 0x00, 0xFF)),
        ];
        let formula = ctx.params.str_or("distanceFormula", "Euclidean");
        let site_count = (ctx.params.f32_or("siteCount", 20.0).round() as usize).clamp(2, 50);
        let speed = ctx.params.f32_or("speed", 2.0);
        let draw_sites = ctx.params.bool_or("drawSites", false);
        let site_color = ctx.params.color_or("siteColor", Rgb::BLACK);
        let dt_scale = (ctx.dt_seconds * 60.0).max(0.05);

        while self.sites.len() < site_count {
            let s = self.spawn_site();
            self.sites.push(s);
        }
        self.sites.truncate(site_count);

        for site in &mut self.sites {
            site.dx = (site.dx + self.rng.range_f32(-0.5, 0.5)).clamp(-speed, speed);
            site.dy = (site.dy + self.rng.range_f32(-0.5, 0.5)).clamp(-speed, speed);
            site.x += site.dx * dt_scale * 0.45;
            site.y += site.dy * dt_scale * 0.45;

            if site.x < 0.0 || site.x > 320.0 || site.y < 0.0 || site.y > 200.0 {
                site.x = self.rng.range_f32(0.0, 320.0);
                site.y = self.rng.range_f32(0.0, 200.0);
            }
        }

        let palette = PALETTES.iter().find(|p| p.name == color_mode);
        let site_colors: Vec<_> = self
            .sites
            .iter()
            .map(|s| {
                let rgb = if let Some(p) = palette {
                    let (h, sat, l) = p.colors[s.color_idx % p.colors.len()];
                    Hsl::new(h, sat, l).to_rgb()
                } else if color_mode == "Rainbow" {
                    Hsl::new((s.x / 320.0) * 360.0, 100.0, 50.0).to_rgb()
                } else if color_mode == "4-Custom Colors" {
                    custom[s.color_idx % 4]
                } else {
                    Hsl::new(s.rand_hue, 100.0, 50.0).to_rgb()
                };
                rgb.to_rgba(255)
            })
            .collect();

        // Render at 2x2 block resolution (160x100) for silky 60+ FPS Voronoi regardless of site count.
        let w = canvas.width() as usize;
        let h = canvas.height() as usize;
        let pixels = canvas.pixels_mut();
        let use_manhattan = formula == "Manhattan";
        let use_chebyshev = formula == "Chebyshev";

        for by in (0..h).step_by(2) {
            let py = by as f32 + 1.0;
            for bx in (0..w).step_by(2) {
                let px = bx as f32 + 1.0;
                let mut best_idx = 0usize;
                let mut best_dist = f32::MAX;

                for (idx, site) in self.sites.iter().enumerate() {
                    let dx = (px - site.x).abs();
                    let dy = (py - site.y).abs();
                    let dist = if use_manhattan {
                        dx + dy
                    } else if use_chebyshev {
                        dx.max(dy)
                    } else {
                        dx * dx + dy * dy
                    };
                    if dist < best_dist {
                        best_dist = dist;
                        best_idx = idx;
                    }
                }

                let c = site_colors[best_idx];
                pixels[by * w + bx] = c;
                if bx + 1 < w {
                    pixels[by * w + bx + 1] = c;
                }
                if by + 1 < h {
                    pixels[(by + 1) * w + bx] = c;
                    if bx + 1 < w {
                        pixels[(by + 1) * w + bx + 1] = c;
                    }
                }
            }
        }

        if draw_sites {
            let sc = site_color.to_rgba(255);
            for site in &self.sites {
                canvas.fill_circle(site.x, site.y, 2.5, sc);
            }
        }
    }
}

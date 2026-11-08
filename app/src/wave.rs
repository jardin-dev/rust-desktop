//! Portage fidèle de `@Resources/LuaScript/WaterWave.lua`.
//!
//! Le script d'origine entretient deux cartes de hauteurs et fait avancer une
//! équation d'onde « leapfrog » à chaque frame. On garde exactement les mêmes
//! constantes (spread, scale, exposant) et le même découpage en barres.

pub const BAR_COUNT: usize = 256;

const SCALE: f32 = 7.0;
const EXP_SCALE_FACTOR: f32 = 2.8;
const SPREAD: f32 = 9.0;
/// Le Lua force la raideur à 1 dès que la valeur configurée est <= 1.
const STIFFNESS: f32 = 1.0;

pub struct Wave {
    map_a: Vec<f32>,
    map_b: Vec<f32>,
    use_a: bool,
    width: f32,
    height: f32,
    points: Vec<[f32; 2]>,
    parent: Vec<usize>,
}

impl Wave {
    pub fn new(width: f32, height: f32, band_count: usize) -> Self {
        let mut map_a = vec![0.0; BAR_COUNT];
        let map_b = vec![0.0; BAR_COUNT];

        // Le skin démarre avec une petite encoche au centre (-3 sur 15 barres).
        for value in map_a
            .iter_mut()
            .skip(BAR_COUNT / 2 - 7)
            .take(15)
        {
            *value = -3.0;
        }

        let parent = (0..BAR_COUNT)
            .map(|i| (i * band_count / BAR_COUNT).min(band_count.saturating_sub(1)))
            .collect();

        let mut wave = Self {
            map_a,
            map_b,
            use_a: true,
            width,
            height,
            points: Vec::with_capacity(BAR_COUNT * 3 + 1),
            parent,
        };
        wave.rebuild_points();
        wave
    }

    /// Fait avancer la simulation d'un pas et renvoie la courbe à dessiner.
    pub fn update(&mut self, bands: &[f32]) -> &[[f32; 2]] {
        let (source, dest) = if self.use_a {
            (&self.map_a, &mut self.map_b)
        } else {
            (&self.map_b, &mut self.map_a)
        };

        for i in 0..BAR_COUNT {
            let left = if i == 0 { 0 } else { i - 1 };
            let right = if i == BAR_COUNT - 1 {
                BAR_COUNT - 1
            } else {
                i + 1
            };

            let band = bands
                .get(self.parent[i])
                .copied()
                .unwrap_or(0.0)
                .clamp(0.0, 1.0);

            // `dest[i]` conserve ici sa valeur de la frame précédente (comme
            // dans le script Lua : `dest[i] = dest[i] + band`). Le remettre
            // à zéro à chaque passe faisait retomber l'onde au bout d'une
            // frame, d'où l'animation qui disparaissait.
            let mut value = dest[i] + band.powf(EXP_SCALE_FACTOR);
            value = (source[left] + source[right]) / STIFFNESS - value;
            value -= value / SPREAD;
            dest[i] = value;
        }
        self.use_a = !self.use_a;

        self.rebuild_points();
        &self.points
    }

    fn rebuild_points(&mut self) {
        let current = if self.use_a {
            &self.map_a
        } else {
            &self.map_b
        };

        let height = self.height;
        let scale = self.width / BAR_COUNT as f32 / 2.0;

        // Le Lua appelle toCurve(..., -Scale) : map(v, Scale, -Scale, height*2, 0).
        // Les valeurs positives descendent donc vers le bas de la fenêtre.
        let project = |v: f32| height + (v / SCALE) * height;

        self.points.clear();
        let mut y_val = project(current[0]);
        self.points.push([0.0, y_val]);

        for i in 0..BAR_COUNT {
            let y_next = project(current[(i + 1).min(BAR_COUNT - 1)]);
            let x_end = i as f32 / (BAR_COUNT - 1) as f32 * self.width;
            let y_end = (y_val + y_next) * 0.5;

            let start = *self.points.last().unwrap_or(&[0.0, y_val]);
            for step in 1..=3 {
                let t = step as f32 / 3.0;
                let inv = 1.0 - t;
                let x = inv * inv * start[0]
                    + 2.0 * inv * t * (x_end - scale)
                    + t * t * x_end;
                let y = inv * inv * start[1]
                    + 2.0 * inv * t * y_val
                    + t * t * y_end;
                self.points.push([x, y]);
            }

            y_val = y_next;
        }
    }
}

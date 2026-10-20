use rand;

struct Limits {
    x_max: f32,
    x_min: f32,
    y_min: f32,
    y_max: f32,
}

impl Limits {
    fn new(x_max: f32, x_min: f32, y_min: f32, y_max: f32) -> Self {
        Self {
            x_max,
            x_min,
            y_min,
            y_max,
        }
    }
}

struct Ball(f32, f32);

impl Ball {
    fn new(limits: &Limits, display: fn(f32, f32)) {
        let x = rand::random_range(limits.x_min..limits.x_max);
        let y = rand::random_range(limits.y_min..limits.y_max);

        display(x, y);
    }
}

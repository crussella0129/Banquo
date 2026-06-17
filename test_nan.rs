fn main() {
    let n = 3.5;
    let t = std::f32::consts::FRAC_PI_2;
    let x = t.cos().powf(2.0 / n);
    println!("cos: {}, x: {}", t.cos(), x);
}

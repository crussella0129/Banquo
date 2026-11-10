//! The public data contracts: `Surface` storage and pygame `Rect.clip` semantics.

use banquo_canvas::{IRect, Rgba8, Surface};

fn all_pixels(s: &Surface) -> Vec<Option<Rgba8>> {
    (0..s.height() as i32)
        .flat_map(|y| (0..s.width() as i32).map(move |x| (x, y)))
        .map(|(x, y)| s.get(x, y))
        .collect()
}

#[test]
fn surface_new_fill_get_set() {
    let alpha = Surface::new(4, 3, true);
    assert_eq!(alpha.as_bytes().len(), 48);
    assert!(alpha.as_bytes().iter().all(|&b| b == 0));

    let opaque = Surface::new(4, 3, false);
    assert!(all_pixels(&opaque).iter().all(|&p| p
        == Some(Rgba8 {
            r: 0,
            g: 0,
            b: 0,
            a: 255
        })));

    let c = Rgba8::new(10, 20, 30, 40);
    let mut alpha = alpha;
    let mut opaque = opaque;
    alpha.fill(c);
    opaque.fill(c);
    assert!(all_pixels(&alpha)
        .iter()
        .all(|&p| p == Some(Rgba8::new(10, 20, 30, 40))));
    assert!(all_pixels(&opaque)
        .iter()
        .all(|&p| p == Some(Rgba8::new(10, 20, 30, 255))));

    let mark = Rgba8::new(200, 100, 50, 25);
    let before = all_pixels(&alpha);
    alpha.set(3, 2, mark);
    let after = all_pixels(&alpha);
    assert_eq!(alpha.get(3, 2), Some(mark));
    let unchanged = before.iter().zip(&after).filter(|(b, a)| b == a).count();
    assert_eq!(unchanged, 11);

    assert_eq!(alpha.get(4, 0), None);
    let bytes = alpha.as_bytes().to_vec();
    alpha.set(-1, 0, mark);
    assert_eq!(alpha.as_bytes(), &bytes[..]);
}

#[test]
fn clip_follows_pygame() {
    assert_eq!(
        IRect::new(-5, -3, 10, 10).clip(IRect::new(0, 0, 20, 20)),
        IRect::new(0, 0, 5, 7)
    );
    assert_eq!(
        IRect::new(30, 30, 5, 5).clip(IRect::new(0, 0, 20, 20)),
        IRect::new(30, 30, 0, 0)
    );
    assert!(!IRect::new(0, 0, 10, 10).colliderect(IRect::new(10, 0, 5, 5)));
    assert_eq!(
        Surface::new(20, 20, true).clip_rect(IRect::new(-5, -3, 10, 10)),
        IRect::new(0, 0, 5, 7)
    );
}

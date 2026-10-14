//! Kubelka-Munk Spectral Theory & Mixbox-Compatible Subtractive Pigment Mixing
//! Reference: Sochorova & Jamriska (SIGGRAPH Talks / JCGT), "Mixbox: Practical Pigment Mixing for Digital Painting"

#![forbid(unsafe_code)]

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PigmentCoeffs {
    pub absorption_k: [f32; 3],
    pub scattering_s: [f32; 3],
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct PigmentConcentration(pub [f32; 4]);

#[must_use]
pub fn evaluate_kubelka_munk(k: [f32; 3], s: [f32; 3], thickness: f32, bg_refl: [f32; 3]) -> [f32; 3] {
    let mut out = [0.0_f32; 3];
    let d = thickness.max(1e-4);
    for ch in 0..3 {
        let sc = s[ch].max(1e-4);
        let kc = k[ch].max(1e-5);
        let a = 1.0 + (kc / sc);
        let b = (a * a - 1.0).max(1e-6).sqrt();
        let b_s_d = (b * sc * d).clamp(1e-4, 12.0);
        let coth = b_s_d.cosh() / b_s_d.sinh().max(1e-6);

        let rg = bg_refl[ch].clamp(0.0, 1.0);
        let num = 1.0 - rg * (a - b * coth);
        let den = (a - rg + b * coth).max(1e-6);
        out[ch] = (num / den).clamp(0.0, 1.0);
    }
    out
}

#[must_use]
pub fn srgb_to_kubelka_munk(rgb: [u8; 3]) -> PigmentCoeffs {
    let r = (f32::from(rgb[0]) / 255.0).clamp(0.005, 0.995);
    let g = (f32::from(rgb[1]) / 255.0).clamp(0.005, 0.995);
    let b = (f32::from(rgb[2]) / 255.0).clamp(0.005, 0.995);

    let fr = (1.0 - r).powi(2) / (2.0 * r);
    let fg = (1.0 - g).powi(2) / (2.0 * g);
    let fb = (1.0 - b).powi(2) / (2.0 * b);

    let s_vec = [
        (0.15 + 0.75 * r).clamp(0.08, 0.95),
        (0.15 + 0.75 * g).clamp(0.08, 0.95),
        (0.15 + 0.75 * b).clamp(0.08, 0.95),
    ];
    let k_vec = [
        (fr * s_vec[0]).clamp(0.01, 4.0),
        (fg * s_vec[1]).clamp(0.01, 4.0),
        (fb * s_vec[2]).clamp(0.01, 4.0),
    ];

    PigmentCoeffs {
        absorption_k: k_vec,
        scattering_s: s_vec,
    }
}

#[must_use]
pub fn subtractive_pigment_lerp(rgb_a: [u8; 3], rgb_b: [u8; 3], t: f32) -> [u8; 3] {
    let t = t.clamp(0.0, 1.0);
    if t <= 0.0 {
        return rgb_a;
    }
    if t >= 1.0 {
        return rgb_b;
    }

    let p_a = srgb_to_kubelka_munk(rgb_a);
    let p_b = srgb_to_kubelka_munk(rgb_b);

    let mut k_mix = [0.0_f32; 3];
    let mut s_mix = [0.0_f32; 3];
    for i in 0..3 {
        k_mix[i] = p_a.absorption_k[i] * (1.0 - t) + p_b.absorption_k[i] * t;
        s_mix[i] = p_a.scattering_s[i] * (1.0 - t) + p_b.scattering_s[i] * t;
    }

    let blue_ness_a = (f32::from(rgb_a[2]) - f32::from(rgb_a[0])).max(0.0) / 255.0;
    let yellow_ness_b = ((f32::from(rgb_b[0]) + f32::from(rgb_b[1])) * 0.5 - f32::from(rgb_b[2])).max(0.0) / 255.0;
    let blue_ness_b = (f32::from(rgb_b[2]) - f32::from(rgb_b[0])).max(0.0) / 255.0;
    let yellow_ness_a = ((f32::from(rgb_a[0]) + f32::from(rgb_a[1])) * 0.5 - f32::from(rgb_a[2])).max(0.0) / 255.0;
    let comp_weight = 4.0 * t * (1.0 - t) * (blue_ness_a * yellow_ness_b + blue_ness_b * yellow_ness_a);

    if comp_weight > 0.05 {
        k_mix[0] *= 1.0 + 2.8 * comp_weight;
        k_mix[1] *= (1.0 - 0.52 * comp_weight).max(0.15);
        k_mix[2] *= 1.0 + 1.15 * comp_weight;
        s_mix[1] *= 1.0 + 0.35 * comp_weight;
    }

    let refl = evaluate_kubelka_munk(k_mix, s_mix, 1.0, [1.0, 1.0, 1.0]);
    [
        (refl[0] * 255.0).round().clamp(0.0, 255.0) as u8,
        (refl[1] * 255.0).round().clamp(0.0, 255.0) as u8,
        (refl[2] * 255.0).round().clamp(0.0, 255.0) as u8,
    ]
}

#[must_use]
pub fn kubelka_munk_glaze(rgb_pigment: [u8; 3], film_thickness: f32, substrate_rgb: [u8; 3]) -> [u8; 3] {
    let coeffs = srgb_to_kubelka_munk(rgb_pigment);
    let bg = [
        (f32::from(substrate_rgb[0]) / 255.0).clamp(0.01, 0.995),
        (f32::from(substrate_rgb[1]) / 255.0).clamp(0.01, 0.995),
        (f32::from(substrate_rgb[2]) / 255.0).clamp(0.01, 0.995),
    ];
    let refl = evaluate_kubelka_munk(coeffs.absorption_k, coeffs.scattering_s, film_thickness, bg);
    [
        (refl[0] * 255.0).round().clamp(0.0, 255.0) as u8,
        (refl[1] * 255.0).round().clamp(0.0, 255.0) as u8,
        (refl[2] * 255.0).round().clamp(0.0, 255.0) as u8,
    ]
}

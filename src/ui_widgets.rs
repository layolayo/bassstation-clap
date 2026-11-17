use nih_plug_egui::egui::{self, Color32, Pos2, Stroke, Vec2};

/// Draw a live Low-Pass Filter frequency response curve (Frequency + Resonance + Mod Depth).
pub fn draw_filter_graph(ui: &mut egui::Ui, freq: i32, res: i32, mod_depth: i32) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 56.0), egui::Sense::hover());
    let p = ui.painter_at(rect);
    p.rect_filled(rect, 3.0_f32, Color32::from_rgb(18, 22, 28));

    let pad = 4.0_f32;
    let w = (rect.width() - pad * 2.0_f32).max(10.0_f32);
    let h = (rect.height() - pad * 2.0_f32).max(10.0_f32);
    let left = rect.left() + pad;
    let bottom = rect.bottom() - pad;

    let cut_norm = (freq as f32 / 127.0_f32).clamp(0.06, 0.94);
    let res_norm = (res as f32 / 127.0_f32).clamp(0.0, 1.0);
    let mod_norm = (mod_depth as f32 / 127.0_f32).clamp(0.0, 1.0);
    let base_y = bottom - h * 0.55_f32;
    let peak_h = res_norm * h * 0.42_f32;

    // Show Filter Mod Depth shaded sweep range around cutoff
    if mod_norm > 0.01 {
        let cx = left + cut_norm * w;
        let span = mod_norm * w * 0.35_f32;
        let x0 = (cx - span).max(left);
        let x1 = (cx + span).min(rect.right() - pad);
        let mod_rect = egui::Rect::from_min_max(
            Pos2::new(x0, rect.top() + 3.0),
            Pos2::new(x1, bottom),
        );
        p.rect_filled(
            mod_rect,
            2.0_f32,
            Color32::from_rgba_unmultiplied(245, 175, 85, 38),
        );
        let edge_stroke = Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(245, 175, 85, 130));
        p.line_segment([Pos2::new(x0, rect.top() + 3.0), Pos2::new(x0, bottom)], edge_stroke);
        p.line_segment([Pos2::new(x1, rect.top() + 3.0), Pos2::new(x1, bottom)], edge_stroke);
    }

    let steps = 48;
    let mut pts = Vec::with_capacity(steps + 1);
    for i in 0..=steps {
        let x_norm = i as f32 / steps as f32;
        let dist = x_norm - cut_norm;
        let pass = 1.0_f32 / (1.0_f32 + (dist * 10.0_f32).exp());
        let bump = (-((dist * 14.0_f32).powi(2))).exp() * peak_h;
        let y = (bottom - pass * (bottom - base_y) - bump).clamp(rect.top() + 2.0, bottom);
        pts.push(Pos2::new(left + x_norm * w, y));
    }

    p.add(egui::Shape::line(
        pts,
        Stroke::new(1.8_f32, Color32::from_rgb(90, 215, 250)),
    ));
}

/// Draw a live Attack-Decay Envelope curve for BassStation Env 1 / Env 2.
pub fn draw_ad_env_graph(ui: &mut egui::Ui, attack: i32, decay: i32, color: Color32) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 46.0), egui::Sense::hover());
    let p = ui.painter_at(rect);
    p.rect_filled(rect, 3.0_f32, Color32::from_rgb(18, 22, 28));

    let pad = 4.0_f32;
    let w = (rect.width() - pad * 2.0_f32).max(10.0_f32);
    let h = (rect.height() - pad * 2.0_f32).max(10.0_f32);
    let left = rect.left() + pad;
    let bottom = rect.bottom() - pad;

    let atk_w = (attack as f32 / 127.0_f32).max(0.02) * (w * 0.45_f32);
    let dec_w = (decay as f32 / 127.0_f32).max(0.04) * (w * 0.50_f32);

    let pts = [
        Pos2::new(left, bottom),
        Pos2::new(left + atk_w, bottom - h),
        Pos2::new((left + atk_w + dec_w).min(rect.right() - pad), bottom),
    ];

    p.add(egui::Shape::line(pts.to_vec(), Stroke::new(1.8_f32, color)));
}

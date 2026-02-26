/// Joystick ölü bölgesi (piksel cinsinden)
const DEAD_ZONE_PX: f32 = 15.0;

/// Eksen snap toleransı (normalize ölçek, 0-100)
/// Kılavuz çizginin her iki yanında bu kadar sapma sıfırlanır.
/// Düz ileri/geri: |norm_x| < 8 → turn = 0 → left == right
/// Saf dönüş:     |norm_y| < 8 → base = 0 → yerinde dönüş
const AXIS_SNAP_THRESHOLD: f32 = 8.0;

/// Joystick giriş değerleri
#[derive(Debug, Clone, Copy)]
pub struct JoystickInput {
    /// Merkezden yatay sapma (pozitif = sağ)
    pub dx: f32,
    /// Merkezden dikey sapma (pozitif = ileri, Y ekseni çevrilmiş)
    pub dy: f32,
    /// Daire yarıçapı (maksimum sapma)
    pub max_radius: f32,
}

/// Motor hızları
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct MotorSpeeds {
    pub left:  i8, // -100..100
    pub right: i8, // -100..100
}

/// Diferansiyel sürüş algoritması
///
/// Tam ileri/geri: her iki motor aynı yön ve hızda.
/// Tam sağ/sol (yerinde dönüş): motorlar zıt yönde tam güçte.
/// Köşegen: ağırlıklı karışım — ileri giderken hafif dönüş.
///
///   left  = base - turn
///   right = base + turn
///
/// `turn` saf dönüşte `norm_x` (1:1), ileri giderken azaltılır.
pub fn calculate(input: &JoystickInput) -> MotorSpeeds {
    if input.dx.abs() < DEAD_ZONE_PX && input.dy.abs() < DEAD_ZONE_PX {
        return MotorSpeeds { left: 0, right: 0 };
    }

    let norm_x = (input.dx / input.max_radius).clamp(-1.0, 1.0) * 100.0;
    let norm_y = (input.dy / input.max_radius).clamp(-1.0, 1.0) * 100.0;

    // Eksen snap: kılavuz çizgi tolerans bandı
    let norm_x = if norm_x.abs() < AXIS_SNAP_THRESHOLD { 0.0 } else { norm_x };
    let norm_y = if norm_y.abs() < AXIS_SNAP_THRESHOLD { 0.0 } else { norm_y };

    let base = norm_y;
    let turn = norm_x;

    MotorSpeeds {
        left:  (base - turn).clamp(-100.0, 100.0) as i8,
        right: (base + turn).clamp(-100.0, 100.0) as i8,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dead_zone() {
        let input = JoystickInput { dx: 10.0, dy: 5.0, max_radius: 100.0 };
        let s = calculate(&input);
        assert_eq!(s.left,  0);
        assert_eq!(s.right, 0);
    }

    #[test]
    fn test_forward() {
        let input = JoystickInput { dx: 0.0, dy: 100.0, max_radius: 100.0 };
        let s = calculate(&input);
        assert_eq!(s.left,  100);
        assert_eq!(s.right, 100);
    }

    #[test]
    fn test_backward() {
        let input = JoystickInput { dx: 0.0, dy: -100.0, max_radius: 100.0 };
        let s = calculate(&input);
        assert_eq!(s.left,  -100);
        assert_eq!(s.right, -100);
    }

    #[test]
    fn test_turn_right_in_place() {
        // Tam sağa → yerinde dönüş: left = -100, right = +100
        let input = JoystickInput { dx: 100.0, dy: 0.0, max_radius: 100.0 };
        let s = calculate(&input);
        assert_eq!(s.left, -100, "Tam sağda sol motor = -100 olmalı");
        assert_eq!(s.right, 100, "Tam sağda sağ motor = +100 olmalı");
    }

    #[test]
    fn test_turn_left_in_place() {
        // Tam sola → yerinde dönüş: left = +100, right = -100
        let input = JoystickInput { dx: -100.0, dy: 0.0, max_radius: 100.0 };
        let s = calculate(&input);
        assert_eq!(s.left, 100, "Tam solda sol motor = +100 olmalı");
        assert_eq!(s.right, -100, "Tam solda sağ motor = -100 olmalı");
    }

    #[test]
    fn test_speed_clamped() {
        let input = JoystickInput { dx: 200.0, dy: 200.0, max_radius: 100.0 };
        let s = calculate(&input);
        assert!(s.left  <= 100 && s.left  >= -100);
        assert!(s.right <= 100 && s.right >= -100);
    }

    #[test]
    fn test_axis_snap_forward() {
        // Düz ileri + küçük yatay sapma → snap → left == right
        let input = JoystickInput { dx: 5.0, dy: 80.0, max_radius: 100.0 };
        let s = calculate(&input);
        assert_eq!(s.left, s.right, "Eksen snap: düz ileri'de left == right olmalı");
    }

    #[test]
    fn test_axis_snap_pure_turn() {
        // Saf sağa dönüş + küçük dikey sapma → snap → base = 0
        let input = JoystickInput { dx: 80.0, dy: 5.0, max_radius: 100.0 };
        let s = calculate(&input);
        assert_eq!(s.left, -s.right, "Eksen snap: saf dönüşte left == -right olmalı");
    }

    #[test]
    fn test_axis_snap_diagonal_no_snap() {
        // Köşegen hareket — snap eşiği üstünde → normal diferansiyel
        let input = JoystickInput { dx: 50.0, dy: 50.0, max_radius: 100.0 };
        let s = calculate(&input);
        assert_ne!(s.left, s.right, "Köşegen harekette left != right olmalı");
    }
}

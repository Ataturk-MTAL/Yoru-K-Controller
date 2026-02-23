/// Joystick ölü bölgesi (piksel cinsinden)
const DEAD_ZONE_PX: f32 = 15.0;

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
#[derive(Debug, Clone, Copy, Default)]
pub struct MotorSpeeds {
    pub left:  i8, // -100..100
    pub right: i8, // -100..100
}

/// Diferansiyel sürüş algoritması
/// Python `joystick_worker.py` ile birebir aynı:
///   base_speed  = norm_y
///   turn_factor = norm_x * 0.5
///   left        = base_speed - turn_factor
///   right       = base_speed + turn_factor
pub fn calculate(input: &JoystickInput) -> MotorSpeeds {
    if input.dx.abs() < DEAD_ZONE_PX && input.dy.abs() < DEAD_ZONE_PX {
        return MotorSpeeds { left: 0, right: 0 };
    }

    let norm_x = (input.dx / input.max_radius).clamp(-1.0, 1.0) * 100.0;
    let norm_y = (input.dy / input.max_radius).clamp(-1.0, 1.0) * 100.0;

    let base = norm_y;
    let turn = norm_x * 0.5;

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
    fn test_turn_right() {
        // dx pozitif → sağa dön → sol motor daha hızlı
        let input = JoystickInput { dx: 100.0, dy: 0.0, max_radius: 100.0 };
        let s = calculate(&input);
        assert!(s.left < s.right, "Sağa dönüşte sol > sağ olmalı değil");
    }

    #[test]
    fn test_turn_left() {
        let input = JoystickInput { dx: -100.0, dy: 0.0, max_radius: 100.0 };
        let s = calculate(&input);
        assert!(s.right < s.left, "Sola dönüşte sağ > sol olmalı değil");
    }

    #[test]
    fn test_speed_clamped() {
        let input = JoystickInput { dx: 200.0, dy: 200.0, max_radius: 100.0 };
        let s = calculate(&input);
        assert!(s.left  <= 100 && s.left  >= -100);
        assert!(s.right <= 100 && s.right >= -100);
    }
}

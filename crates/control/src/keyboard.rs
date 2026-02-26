use crate::joystick::MotorSpeeds;

/// Klavye tuş durumu (WASD + ok tuşları)
#[derive(Debug, Clone, Copy, Default)]
pub struct KeyboardState {
    pub forward:  bool, // W / ↑
    pub backward: bool, // S / ↓
    pub left:     bool, // A / ←
    pub right:    bool, // D / →
}

/// Klavye girdisinden motor hızlarını hesaplar
/// İleri/geri tam hız (100), dönüş tam hız (100) olarak uygulanır.
/// Saf dönüş (sadece sol/sağ): yerinde dönüş — motorlar zıt yönde.
/// İleri+dönüş: ileri giderken viraj — clamp ile sınırlanır.
pub fn calculate(state: &KeyboardState) -> MotorSpeeds {
    let forward  = if state.forward  { 100i32 } else { 0 };
    let backward = if state.backward { 100i32 } else { 0 };
    let turn_l   = if state.left     { 100i32 } else { 0 };
    let turn_r   = if state.right    { 100i32 } else { 0 };

    let base    = forward - backward;
    let left_v  = (base - turn_r + turn_l).clamp(-100, 100) as i8;
    let right_v = (base + turn_r - turn_l).clamp(-100, 100) as i8;

    MotorSpeeds { left: left_v, right: right_v }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_idle() {
        let s = calculate(&KeyboardState::default());
        assert_eq!(s.left,  0);
        assert_eq!(s.right, 0);
    }

    #[test]
    fn test_forward() {
        let s = calculate(&KeyboardState { forward: true, ..Default::default() });
        assert_eq!(s.left,  100);
        assert_eq!(s.right, 100);
    }

    #[test]
    fn test_backward() {
        let s = calculate(&KeyboardState { backward: true, ..Default::default() });
        assert_eq!(s.left,  -100);
        assert_eq!(s.right, -100);
    }

    #[test]
    fn test_turn_right_in_place() {
        // Saf sağ: yerinde dönüş — motorlar zıt yönde, tam güç
        let s = calculate(&KeyboardState { right: true, ..Default::default() });
        assert_eq!(s.left, -100);
        assert_eq!(s.right, 100);
    }

    #[test]
    fn test_turn_left_in_place() {
        // Saf sol: yerinde dönüş — motorlar zıt yönde, tam güç
        let s = calculate(&KeyboardState { left: true, ..Default::default() });
        assert_eq!(s.left, 100);
        assert_eq!(s.right, -100);
    }

    #[test]
    fn test_forward_right() {
        // İleri + sağ: left = 0, right = 100 (viraj)
        let s = calculate(&KeyboardState { forward: true, right: true, ..Default::default() });
        assert_eq!(s.left, 0);
        assert_eq!(s.right, 100);
    }
}

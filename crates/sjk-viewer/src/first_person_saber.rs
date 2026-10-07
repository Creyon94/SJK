//! First-person saber body visibility, from codemp CG_Player and JoF EJK's
//! CG_ForceFPLSPlayerModel: retain the posed body/arms, hide the head and hoses.

/// The local saber body is visible from its own first-person camera.
pub(crate) fn visible(local: bool, third_person: bool, detached: bool, weapon: Option<u8>) -> bool {
    local && !third_person && !detached && weapon == Some(3)
}

/// JoF EJK's first-person head variants and TIE pilot hoses.
pub(crate) fn hide_surface(name: &str) -> bool {
    name.get(..4)
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case("head"))
        || name.eq_ignore_ascii_case("torso_l_hose")
        || name.eq_ignore_ascii_case("torso_r_hose")
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn saber_body_and_head_policy() {
        assert!(visible(true, false, false, Some(3)));
        assert!(!visible(true, true, false, Some(3)));
        assert!(!visible(true, false, true, Some(3)));
        assert!(!visible(true, false, false, Some(4)));
        for name in ["head", "heada_face", "headb_eyes_mouth", "torso_l_hose"] {
            assert!(hide_surface(name));
        }
        for name in ["r_arm", "l_hand", "torso", "hips", "h"] {
            assert!(!hide_surface(name));
        }
    }
}

use std::collections::BTreeSet;

use crate::config::{CompiledMapping, CompiledProfile, PadButton};

/// Resolves which mappings are active given the currently pressed buttons.
/// Longer combos take priority: if LB+A and A are both matched, only LB+A wins.
pub fn resolve_active_mappings<'a>(
    profile: &'a CompiledProfile,
    pressed: &BTreeSet<PadButton>,
) -> Vec<&'a CompiledMapping> {
    if pressed.is_empty() {
        return Vec::new();
    }

    let mut active: Vec<&CompiledMapping> = profile
        .mappings
        .iter()
        .filter(|mapping| mapping.buttons.is_subset(pressed))
        .collect();

    active.sort_by(|left, right| {
        right
            .buttons
            .len()
            .cmp(&left.buttons.len())
            .then_with(|| left.id.cmp(&right.id))
    });

    let mut resolved: Vec<&CompiledMapping> = Vec::new();
    for candidate in active {
        let dominated = resolved
            .iter()
            .any(|winner| candidate.buttons.is_subset(&winner.buttons));
        if !dominated {
            resolved.push(candidate);
        }
    }

    resolved
}

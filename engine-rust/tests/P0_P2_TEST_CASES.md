# P0-P3 Test Cases

## Purpose
- Keep priority-based conformance cases visible in CI.
- Run `enabled` cases now.
- Unignore backlog cases as engine/data gaps are fixed.

## Cases

| Case ID | Priority | Status | Rust Test |
|---|---|---|---|
| `P0-CRIT-DEF-STAGE-IGNORE` | P0 | enabled | `p0_crit_ignores_positive_def_stage` |
| `P0-CRIT-ATK-STAGE-IGNORE` | P0 | enabled | `p0_spec_crit_ignores_negative_attack_stage` |
| `P0-CRIT-WALL-BYPASS` | P0 | enabled | `p0_spec_crit_bypasses_walls_while_non_crit_does_not` |
| `P0-FIELD-STATUS-ATTACH` | P0 | enabled | `p0_field_status_move_sets_status_on_field` |
| `P0-FIELD-STATUS-NONSTACK-REFRESH` | P0 | enabled | `p0_spec_field_status_non_stack_replaces_existing_copy` |
| `P0-DAMAGE-ROLL-GOLDEN` | P0 | enabled | `p0_spec_damage_roll_matches_golden_fixture` |
| `P0-TRICK-ROOM-ORDER` | P0 | enabled | `p0_spec_trick_room_reverses_action_order` |
| `P0-PRIORITY-VS-TRICK-ROOM` | P0 | enabled | `p0_spec_priority_still_overrides_speed_order_under_trick_room` |
| `P0-REFLECT-DAMAGE` | P0 | enabled | `p0_spec_reflect_reduces_physical_damage` |
| `P0-REFLECT-CATEGORY-BOUNDARY` | P0 | enabled | `p0_spec_reflect_does_not_reduce_special_damage` |
| `P0-LIGHT-SCREEN-DAMAGE` | P0 | enabled | `p0_spec_light_screen_reduces_special_damage` |
| `P0-LIGHT-SCREEN-CATEGORY-BOUNDARY` | P0 | enabled | `p0_spec_light_screen_does_not_reduce_physical_damage` |
| `P0-TAILWIND-SPEED` | P0 | enabled | `p0_spec_tailwind_changes_action_order_by_speed` |
| `P0-TOXIC-RESIDUAL` | P0 | enabled | `p0_spec_toxic_damage_scales_each_turn` |
| `P0-TOXIC-SWITCH-RESET` | P0 | enabled | `p0_spec_toxic_resets_counter_after_switch` |
| `P0-TOXIC-SWITCH-COUNTER-CLEARED` | P0 | enabled | `p0_spec_toxic_counter_data_is_cleared_on_switch_out` |
| `P0-PROTECT-CHAIN-PROB` | P0 | enabled | `p0_spec_protect_chain_probability_is_one_third_then_one_ninth` |
| `P0-PROTECT-CHAIN-SUCCESS-COUNTER` | P0 | enabled | `p0_spec_protect_chain_success_increments_counter` |
| `P0-PROTECT-RESET-ON-NONPROTECT` | P0 | enabled | `p0_spec_non_protect_move_resets_protect_chain_counter` |
| `P0-PROTECT-BLOCKS-DAMAGE` | P0 | enabled | `p0_spec_protect_blocks_incoming_damage_when_used_first` |
| `P0-PROTECT-FAIL-ALLOWS-DAMAGE` | P0 | enabled | `p0_spec_failed_protect_does_not_block_incoming_damage` |
| `P0-SLEEP-SWITCH` | P0 | enabled | `p0_spec_sleep_persists_when_switched_out` |
| `P0-SLEEP-WAKE-ON-COUNTER-ZERO` | P0 | enabled | `p0_spec_sleep_wakes_and_allows_action_when_counter_reaches_zero` |
| `P0-SLEEP-SWITCH-TURN-COUNTER` | P0 | enabled | `p0_spec_sleep_turn_counter_persists_through_switch` |
| `P0-SWITCH-CLEANUP` | P0 | enabled | `p0_spec_switch_clears_volatile_data_and_stages_while_preserving_non_volatile_status` |
| `P0-MANUAL-NOOP-GATE` | P0 | enabled | `p0_manual_effects_must_not_be_silent_noop` |
| `P0-WIN-SIMULTANEOUS-FAINT` | P0 | enabled | `p0_spec_simultaneous_faint_resolution_rule` |
| `P0-WIN-SIMULTANEOUS-FAINT-SPEED-TIE` | P0 | enabled | `p0_spec_simultaneous_faint_speed_tie_is_draw` |
| `P0-WIN-TIMEOUT-RULE` | P0 | enabled | `p0_spec_timeout_resolution_rule` |
| `P0-WIN-TIMEOUT-TOTAL-HP-TIEBREAK` | P0 | enabled | `p0_spec_timeout_uses_total_hp_as_final_tiebreaker` |
| `P0-WIN-TIMEOUT-EXACT-TIE` | P0 | enabled | `p0_spec_timeout_returns_none_on_exact_tie` |
| `P0-WIN-SINGLE-ALIVE` | P0 | enabled | `p0_spec_winner_is_alive_side_when_only_one_side_has_remaining_creatures` |
| `P0-TOXIC-MIN-DAMAGE` | P0 | enabled | `p0_spec_toxic_damage_has_minimum_of_one` |
| `P1-LEARNSET-MOVE-REF` | P1 | enabled | `p1_spec_learnset_moves_must_exist_in_move_db` |
| `P1-TARGET-LITERAL-LINT` | P1 | enabled | `p1_spec_effect_targets_use_supported_literals` |
| `P1-STATUS-ID-LINT` | P1 | enabled | `p1_spec_effect_status_ids_use_supported_canonical_ids` |
| `P1-ABILITY-STATUS-FIELD` | P1 | enabled | `p1_spec_ability_status_field_interaction_matrix` |
| `P1-ENDTURN-ORDER` | P1 | enabled | `p1_spec_end_turn_effect_ordering` |
| `P1-MANUAL-REASON-TAXONOMY` | P1 | enabled | `p1_spec_manual_reason_uses_approved_taxonomy` |
| `P1-TAILWIND-SIDE-SCOPE` | P1 | enabled | `p1_spec_tailwind_only_boosts_the_owner_side` |
| `P1-TARGET-DEFAULT-OPPONENT` | P1 | enabled | `p1_spec_action_without_target_defaults_to_opponent` |
| `P1-MOVE-PRIORITY-RANGE` | P1 | enabled | `p1_spec_move_priorities_remain_within_supported_bounds` |
| `P1-MANUAL-REASON-NONEMPTY` | P1 | enabled | `p1_spec_manual_effects_have_non_empty_reason` |
| `P1-SWITCH-ACTIVE-SLOT-REJECT` | P1 | enabled | `p1_spec_switch_to_active_slot_is_rejected` |
| `P1-SWITCH-WITHOUT-SLOT-REJECT` | P1 | enabled | `p1_spec_switch_without_slot_is_rejected` |
| `P1-USE-ITEM-NO-ITEM-REJECT` | P1 | enabled | `p1_spec_use_item_without_item_is_rejected` |
| `P1-USE-ITEM-WITH-ITEM-LOG` | P1 | enabled | `p1_spec_use_item_with_item_emits_use_log` |
| `P2-CASE-REGISTRY-INTEGRITY` | P2 | enabled | `p2_case_registry_integrity` |
| `P2-CASE-DOC-SYNC` | P2 | enabled | `p2_case_registry_is_synced_with_markdown_table` |
| `P2-CASE-ID-PREFIX-CHECK` | P2 | enabled | `p2_case_id_prefix_matches_priority_bucket` |
| `P2-CASE-DOC-UNIQUE` | P2 | enabled | `p2_markdown_table_case_ids_are_unique` |
| `P2-CASE-DOC-BIDIRECTIONAL-SYNC` | P2 | enabled | `p2_case_registry_and_markdown_table_are_bidirectionally_synced` |
| `P2-CASE-DOC-TEST-FN-EXISTS` | P2 | enabled | `p2_markdown_table_test_names_exist_in_source` |
| `P2-CASE-DOC-ROW-COUNT-MATCH` | P2 | enabled | `p2_markdown_table_row_count_matches_case_registry` |
| `P2-CASE-DOC-PRIORITY-COLUMN-CHECK` | P2 | enabled | `p2_markdown_priority_column_matches_case_id_prefix` |
| `P2-DOUBLE-MODEL-SMOKE` | P2 | enabled | `p2_spec_double_battle_model_smoke` |
| `P3-SEED-DETERMINISM` | P3 | enabled | `p3_spec_same_seed_produces_identical_battle_state` |
| `P3-UNKNOWN-MOVE-GUARD` | P3 | enabled | `p3_spec_unknown_move_is_logged_and_skipped` |
| `P3-INVALID-SWITCH-SLOT-GUARD` | P3 | enabled | `p3_spec_invalid_switch_slot_is_rejected_without_changing_active_slot` |
| `P3-FAINTED-SWITCH-SLOT-GUARD` | P3 | enabled | `p3_spec_switch_to_fainted_slot_is_rejected_without_changing_active_slot` |
| `P3-TIMEOUT-NON-2P-NONE` | P3 | enabled | `p3_spec_timeout_winner_is_none_when_player_count_is_not_two` |
| `P3-WINNER-ALL-FAINT-NON-2P-NONE` | P3 | enabled | `p3_spec_determine_winner_returns_none_for_three_way_all_faint` |
| `P3-UNKNOWN-PLAYER-ACTION-GUARD` | P3 | enabled | `p3_spec_unknown_player_action_is_skipped_without_affecting_valid_actions` |
| `P3-MISSING-MOVE-ID-GUARD` | P3 | enabled | `p3_spec_missing_move_id_is_logged_and_skipped` |

## How to run

- Enabled only:
  - `cargo test --test spec_priority_cases`
- Include ignored backlog cases:
  - `cargo test --test spec_priority_cases -- --ignored`

## Promotion rule

1. Fix engine/data gap for one case.
2. Remove `#[ignore]` from that case.
3. Add/adjust assertions if spec wording changed.
4. Keep the table above synchronized.

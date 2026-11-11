# Front-end audit (spec #150, FE12)

Each user story of [the front-end spec](front-end-spec.md), with the test that covers it or the reason it is not covered. Paths are under `crates/`. The whole workspace suite passed on the tree this note was written against. The closing comment on #162 gives its counts.

The acceptance walk, `sf-engine/tests/front_end_acceptance.rs` (`the_front_end_from_power_on_to_exit`), runs every screen and play mode in one session with no untranslated branch.

## The attract and first launch

| # | Story | Covered by |
| --- | --- | --- |
| 1 | One executable; the attract plays as the cabinet's | `sf-engine/tests/front_end_menus.rs` (`attract_is_the_faithful_power_on_machine_with_free_play_over_insert_coin`). The release `sf-game.exe` was run by hand from an empty folder holding only `roms\sf` (FE11, #161). |
| 2 | `FREE PLAY` over `INSERT COIN` | `front_end_menus.rs` (same test) |
| 3 | Any Start opens the menu at any point of the attract | `front_end_menus.rs` (`start_opens_the_main_menu_in_every_attract_phase_and_esc_resumes_the_attract`, `start_waits_for_the_ram_clear_and_every_start_source_opens_the_menu`). Pads: `sf-game/tests/pad_play.rs` (Options) |
| 4 | The attract pauses behind the menu and resumes | `front_end_menus.rs` (`start_opens_..._and_esc_resumes_the_attract`) |
| 5 | 3×, windowed, full volume on first launch | `sf-game/tests/shipping.rs` (`a_first_launch_writes_the_three_files_with_the_defaults`); `sf-game` smoke check |
| 6 | ROM-missing screen with `RETRY` and `EXIT` | `sf-engine/tests/front_end_rom_missing.rs`; `shipping.rs` (`retry_finds_files_added_since_and_carries_on_into_the_attract`). Also checked by hand on the release binary: files added while the screen was up, then Enter on `RETRY` |

## Menus

| # | Story | Covered by |
| --- | --- | --- |
| 7 | Framed Main Menu in the ROM's font, frame, header and cursor | `front_end_menus.rs` (`main_menu_rows_cursor_greyed_items_preset_line_and_sounds`, `every_menu_draws_its_header_in_the_rom_header_font_inside_the_gold_frame`) |
| 8 | The preset line on every framed screen | `front_end_menus.rs` (`main_menu_rows_...`) |
| 9 | Single Player rows, two greyed | `front_end_menus.rs` |
| 10 | Multi Player rows, two greyed | `front_end_menus.rs` |
| 11 | Greyed items dim and skipped | `front_end_menus.rs`; `front_end_settings.rs` (`the_tab_pair_cycles_five_tabs_and_greyed_rows_are_skipped`); `front_end_select.rs` (`greyed_tiles_..._are_skipped`) |
| 12 | Any player's stick, buttons or pad drives one cursor | `front_end_menus.rs` (both sticks); `pad_play.rs` (either pad) |
| 13 | Esc is back, never quit; only Exit closes | `front_end_menus.rs` (`exit_quits_and_nothing_else_does`) |
| 14 | Hard cuts between screens | **No test of its own.** Each session step presents exactly one screen, and there are no transition frames to test. |
| 15 | Cursor and confirm sounds, silent menus | `sf-game/tests/menu_sound.rs` (`attract_music_stops_under_the_menu_and_cursor_and_confirm_sound`) |
| 16 | The close button quits at once | **No automated test.** The game's loop in `sf-game/src/main.rs` ends when the window is closed, and the smoke checks open and close windows. |

## The select

| # | Story | Covered by |
| --- | --- | --- |
| 17 | Arcade offers Ryu and Ken, bosses greyed `SF1+` | `front_end_arcade.rs` (`the_arcade_select_offers_ryu_and_ken_and_greys_the_bosses`) |
| 18 | Local offers all twelve | `front_end_local.rs` (`every_fighter_on_either_side_is_bound_and_plays_a_clean_first_round`) |
| 19 | Training asks CPU or DUMMY, then the select | `front_end_training.rs` (`training_against_a_cpu_...`, `a_training_dummy_...`) |
| 20 | The chosen portrait layout | `front_end_select.rs` (`arcade_has_one_cursor`); the Local layout with both busts: `local_shows_both_busts_...` |
| 21 | Busts decoded from the ROM; names for a bad bust | `front_end_select.rs` (`the_busts_decode_from_the_rom_face_one_way_and_a_damaged_set_shows_names`); `vs_busts.rs` (equal to the native VS screens and the MAME captures) |
| 22 | Grid movement; Enter, LP or Cross; Backspace, LK or Circle | `front_end_select.rs`; `pad_play.rs` (Cross picks though it is also LK) |
| 23 | P2's own blue cursor, moving at once | `front_end_select.rs` (`local_shows_both_busts_and_two_cursors_...`) |
| 24 | Shared tiles, lock and unlock, both confirmed moves on | `front_end_select.rs` (same test) |
| 25 | Mirror matches only where the ladder probe is clean | `front_end_select.rs` (`a_mirror_match_is_offered_only_where_the_ladder_report_found_it_clean`); `boss_ladder_report.rs` (opt-in; writes the report) |
| 26 | Ban lists grey fighters | `front_end_select.rs` (`greyed_tiles_for_arcade_the_ban_list_and_faithful_1987_are_skipped`) |
| 27 | Faithful 1987: Ryu and Ken, two-country screen | `front_end_select.rs` (`faithful_1987_local_is_ryu_against_ken_on_the_two_country_screen`) |
| 28 | The real country select after the portrait select | `front_end_arcade.rs` (`arcade_reaches_the_first_fight_from_each_country_as_ryu_and_ken`); `front_end_local.rs` |

## Fights

| # | Story | Covered by |
| --- | --- | --- |
| 29 | Black during the hidden drive, then a cut | `front_end_local.rs` (`local_walk_...`: the `drive` screen shows black); `the_driver_reaches_the_country_select_from_any_attract_phase` |
| 30 | The real engine under the preset, with the Gameplay tab's difficulty and round time | `front_end_local.rs` (`local_fights_use_the_configured_difficulty_and_round_time`); `front_end_settings.rs` (`leaving_settings_saves_and_difficulty_and_round_time_reach_every_fight`) |
| 31 | The pause box from Space or a pad's Start | `front_end_training.rs` (`pause_opens_from_every_start_...`); `pad_play.rs` (Options; no attack button opens it) |
| 32 | Either player pauses; `QUIT TO MENU` at once | `front_end_training.rs` (`pause_opens_...`, `overlays_restart_round_and_quit_to_menu`) |
| 33 | Training's `PRESET` row | `front_end_training.rs` (`the_preset_row_is_trainings_...`) |
| 34 | `OVERLAYS` cycles four modes | `front_end_training.rs` (`overlays_restart_round_and_quit_to_menu`) |
| 35 | `RESTART ROUND` bit for bit | `front_end_training.rs` (same test); `front_end_acceptance.rs` |
| 36 | The dummy stands idle | `front_end_training.rs` (`a_training_dummy_never_moves_...`) |
| 37 | Local and Training stop at the match end; the after-match box | `front_end_local.rs` (`local_walk_...`); `front_end_training.rs` |
| 38 | `REMATCH` keeps the fighters and the country | `front_end_local.rs` (`local_walk_..._rematch_...`) |
| 39 | The attract resumes as it was | `front_end_local.rs` (`reselect_returns_to_the_select_and_the_attract_stays_as_it_was`); `front_end_training.rs` (`overlays_...`: snapshot equality) |
| 40 | Untranslated branch: notice, Main Menu, log | `front_end_local.rs` (`an_untranslated_branch_shows_the_notice_returns_to_the_menu_and_logs`); the log's `untranslated` line: `shipping.rs` (`a_file_that_fails_to_parse_is_logged_and_left_alone`) |

## Arcade

| # | Story | Covered by |
| --- | --- | --- |
| 41 | The 1987 ladder as Ryu or Ken | `front_end_arcade.rs` (first fight from every country, both fighters). The ladder beyond the first fight is `full_1p.rs`'s. |
| 42 | Free continue on Start, the ROM's limit | `front_end_arcade.rs` (`arcade_run_..._free_continue_...`); `front_end_acceptance.rs` |
| 43 | Countdown out: game over, ranking, the Main Menu | `front_end_arcade.rs` (same test); `front_end_acceptance.rs` |
| 44 | The ranking entry stays on the attract | `front_end_arcade.rs` (the new entry in the attract's ranking table) |

## Settings

| # | Story | Covered by |
| --- | --- | --- |
| 45 | Five tabs, underlined, L1/R1 or Q/E | `front_end_settings.rs` (`the_tab_pair_cycles_five_tabs_...`); `pad_play.rs` (R1) |
| 46 | `NAME .... VALUE`, saved on leaving | `front_end_settings.rs` (`every_live_row_changes_and_reads_back`, `leaving_settings_saves_...`) |
| 47 | `RESET TAB` on every tab | `front_end_settings.rs` (`reset_tab_restores_that_tabs_defaults_only`); `front_end_acceptance.rs` (all five tabs); `pad_play.rs` (Controls: devices and bindings) |
| 48 | The `PRESET` row | `front_end_settings.rs` |
| 49 | Every core toggle, the frame counts | `front_end_settings.rs` (`the_gameplay_rows_wording_and_values`) |
| 50 | A toggle edit flips to `CUSTOM` | `front_end_settings.rs` (`a_toggle_edit_flips_the_preset_to_custom_...`) |
| 51 | Difficulty and round time in every play mode | `front_end_settings.rs` (`leaving_settings_saves_and_difficulty_and_round_time_reach_every_fight`) |
| 52 | Roster rows, three greyed | `front_end_settings.rs` |
| 53 | Integer scale, borderless fullscreen | `sf-game` smoke check (2× windowed; fullscreen at the monitor's largest integer scale) |
| 54 | Volumes, `JUKEBOX` greyed | `sf-game/tests/volume_gains.rs`; `front_end_settings.rs` |
| 55 | Device and scheme rows | `front_end_settings.rs` (`the_gameplay_rows_wording_and_values`: the Controls rows); `sf-host/tests/pads_and_bindings.rs` (`each_side_follows_its_device`) |
| 56 | `REBIND P1`: in order, Esc keeps, duplicates refused | `pad_play.rs` (`the_rebind_walk_binds_keys_and_pad_buttons_...`) |
| 57 | `REBIND MENU` | The walk is one screen for all three lists. `REBIND MENU` opens it on the menu list, and `pads_and_bindings.rs` covers the menu list's file round trip. No test walks `REBIND MENU` itself. |
| 58 | A preset change keeps the chosen fighters | `front_end_settings.rs` (`..._a_preset_change_keeps_the_fighters`); `front_end_training.rs` (live preset) |

## Pads

| # | Story | Covered by |
| --- | --- | --- |
| 59 | DualShock 4 over USB with nothing installed | **Not verified.** No pad was connected to the build machine (the `sf-game` smoke check printed `pads: none connected`). Confirming it is FE13, a person's session with two pads. The mapping itself: `pads_and_bindings.rs`, `pad_play.rs` |
| 60 | Pads by the Controls tab or connection order | `pads_and_bindings.rs` (`pads_are_numbered_by_connection_order_...`, `each_side_follows_its_device`) |
| 61 | A disconnect pauses with the notice; any device resumes; that side idles | `pad_play.rs` (`two_pads_drive_the_menus_and_a_fight_and_a_pulled_pad_pauses_it`); `pads_and_bindings.rs` (`a_pad_side_reads_its_pad_and_the_keyboard_never_takes_over`) |
| 62 | Online never pauses on a disconnect | **Recorded, not built.** There is no online play yet. The rule is in the design doc, section 9, round three, item 6. |

## The 1987 cabinet

| # | Story | Covered by |
| --- | --- | --- |
| 63 | The `1987 CABINET` DIP screen at factory defaults | `front_end_cabinet.rs` (`the_dip_screen_lists_the_verified_settings_at_factory_defaults`) |
| 64 | `POWER ON` boots the original machine with those words | `front_end_cabinet.rs` (`power_on_boots_the_machine_with_the_rows_dip_words`, `an_attract_loop_in_the_cabinet_is_the_power_on_machine_...`). The power-on machine's equality to the reference is the comparison fixtures'. |
| 65 | Test mode boots into the service screen | `front_end_cabinet.rs` (`coins_starts_the_service_credit_and_test_mode_work_in_the_cabinet`) |
| 66 | Nothing from the overhaul settings reaches it | `front_end_cabinet.rs` (`the_walk_into_the_cabinet_...`: `MatchConfig::faithful()`, the cabinet's own difficulty) |
| 67 | A one-second Esc hold, the hint, the hold bar | `front_end_cabinet.rs` (`the_walk_...`: 59 frames do nothing, 60 return; `an_attract_loop_...`: the bar only drawn) |
| 68 | The DIP words persist | `front_end_cabinet.rs` (`the_walk_...`); `front_end_settings.rs` (`the_settings_and_match_config_files_round_trip_...`) |

## Persistence and the log

| # | Story | Covered by |
| --- | --- | --- |
| 69 | The settings files; "last menu positions" in the settings file | The files: `front_end_settings.rs`, `shipping.rs`, `pads_and_bindings.rs`. Menu positions (added after FE12): `front_end_menus.rs` (`menus_open_where_they_were_left_and_the_positions_survive_the_settings_file`) and `front_end_acceptance.rs`. The game writes them to the file when it ends; that save at exit is not under test. |
| 70 | A file that fails to parse is logged and replaced by defaults | `shipping.rs` (`a_file_that_fails_to_parse_is_logged_and_left_alone`); `front_end_settings.rs`; `sf-game` smoke check |
| 71 | ROM lookup order | `shipping.rs` (`the_set_is_found_in_each_folder_in_order`); `sf-game` smoke check (each folder) |
| 72 | One log file, overwritten, with the five kinds of line | `shipping.rs` (the log overwritten, and the `settings`, `parse` and `untranslated` lines). The release binary's log, read by hand, showed the `settings`, `pads` and `roms` lines. The `pads` seen or lost lines come from `sf-host/src/pads.rs`, which has no real pad behind it here. |

## Developer stories

| # | Story | Covered by |
| --- | --- | --- |
| 73 | Pure screens tested without a window | Every `front_end_*.rs` suite is headless. `front_end_rom_missing.rs` and `front_end_settings.rs` drive the screens directly. |
| 74 | A headless session from power-on | `sf_engine::front::GameSession`; `front_end_acceptance.rs` |
| 75 | One host crate for both binaries | `sf-host` (audio, bindings, pads, input, window, files, log, ROM lookup); `sf-dev` and `sf-game` both build on it |
| 76 | The launcher unchanged, with pads | `sf-dev` smoke check and its test suite pass; `sf_host::input::LauncherPads` adds `PAD 1` and `PAD 2` in every play loop |
| 77 | Every comparison fixture passes; the attract matches power-on | The whole workspace suite; `front_end_menus.rs` (`attract_is_the_faithful_power_on_machine_...`); `front_end_cabinet.rs` (a 6,000-frame loop) |

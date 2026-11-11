# Open issue crosswalk

The design and reference notes cite historical ticket numbers. This index points to the current open work in the project tracker.

| Historical ticket | Current issue |
| --- | --- |
| 91 | [#1](https://github.com/johnstreetfighter/sf1-port/issues/1) Spec: Definitive Street Fighter (1987) Overhaul & Toggleable Multi-Era Suite |
| 191 | [#2](https://github.com/johnstreetfighter/sf1-port/issues/2) Spec: Bonus challengers by 1987 lore (28 fighters in three roster waves) |
| 233 | [#3](https://github.com/johnstreetfighter/sf1-port/issues/3) Spec: Roster key art in the 1987 advertising's two styles |
| 237 | [#4](https://github.com/johnstreetfighter/sf1-port/issues/4) KA4: Key art that matches the reference illustrations' drawing (post complete game) |
| 269 | [#5](https://github.com/johnstreetfighter/sf1-port/issues/5) Spec: Era styles and gauges on the real fighters |
| 270 | [#6](https://github.com/johnstreetfighter/sf1-port/issues/6) ES1: Character Select Style card and Arcade CPU era matching |
| 271 | [#7](https://github.com/johnstreetfighter/sf1-port/issues/7) ES2: 960 Sub-HP scale, 0x40 move slots, compositor trails/tints, and SF2 (WW-ST) on the real fighters |
| 272 | [#8](https://github.com/johnstreetfighter/sf1-port/issues/8) ES3: SFZ / Alpha (SFZ1-SFZ3 A-ISM, X-ISM, V-ISM) on the real fighters |
| 273 | [#9](https://github.com/johnstreetfighter/sf1-port/issues/9) ES4: SF3 (NG, 2I, 3S) Parry and Super Arts on the real fighters |
| 274 | [#10](https://github.com/johnstreetfighter/sf1-port/issues/10) ES5: SF4 (SF4-USF4) Focus, FADC, Revenge, and Ultras on the real fighters |
| 275 | [#11](https://github.com/johnstreetfighter/sf1-port/issues/11) ES6: SF5 (S1, AE, CE) V-System, Crush Counter, and V-Shift on the real fighters |
| 276 | [#12](https://github.com/johnstreetfighter/sf1-port/issues/12) ES7: SF6 Drive System and Burnout on the real fighters |
| 277 | [#13](https://github.com/johnstreetfighter/sf1-port/issues/13) ES8: Cross-era matches, No Gauges, and Unified Gauge across all eras |
| 278 | [#14](https://github.com/johnstreetfighter/sf1-port/issues/14) ES9: What-If era styles (SF2-SF6) for Retsu, Geki, Joe, Mike, and Lee |
| 279 | [#15](https://github.com/johnstreetfighter/sf1-port/issues/15) ES10: What-If era styles (SF2-SF6) for Gen, Birdie, Eagle, Adon, and Sagat |
| 280 | [#16](https://github.com/johnstreetfighter/sf1-port/issues/16) ES11: Era-aware CPU AI and retire the standalone CombatSession prototype |
| 281 | [#17](https://github.com/johnstreetfighter/sf1-port/issues/17) ES12: Birdie Alpha / SF5 chain and Break Time food props (waiting on Nov 27 PixelLab renewal) |
| 282 | [#18](https://github.com/johnstreetfighter/sf1-port/issues/18) ES13: Gen Crane (Ki) stance idle and unique normals (waiting on Nov 27 PixelLab renewal) |
| 283 | [#19](https://github.com/johnstreetfighter/sf1-port/issues/19) Spec: Character Endings, 2v1 Dramatic Battle, and 1987 World Tour on the real engine |
| 284 | [#20](https://github.com/johnstreetfighter/sf1-port/issues/20) ED1: Composite boss ending views in BackgroundBank, 10 boss epilogues, stand-in Ryu montage swap, and CONTEXT.md glossary |
| 285 | [#21](https://github.com/johnstreetfighter/sf1-port/issues/21) ED2: Character-specific post-match win quotes and stand-in Ryu win-card swap on event 0xb (0x4014c) for all 12 fighters |
| 288 | [#22](https://github.com/johnstreetfighter/sf1-port/issues/22) ED5: WorldTourScreen over the 0x3f812 travel map, 18-node 5-country branching graph, Fighter/Style switching, RPG modal, and world_tour.toml |
| 289 | [#23](https://github.com/johnstreetfighter/sf1-port/issues/23) ED6: World Tour real-fight mission mechanics (RivalBout, SurvivalGauntlet, TimeAttack, RuleModifierTrial, EraStyleMastery) and RPG combat modifiers |
| 290 | [#24](https://github.com/johnstreetfighter/sf1-port/issues/24) ED7: World Tour 2v1 DramaticBoss country finales (jp-4-boss, us-4-boss, cn-4-boss, en-4-boss, and th-2-era Grand Finale ending + staff roll flow) |
| 291 | [#25](https://github.com/johnstreetfighter/sf1-port/issues/25) ED8: Retire the standalone arcade.rs, dramatic.rs, and world_tour.rs prototype sessions after full real-engine test coverage |
| 292 | [#26](https://github.com/johnstreetfighter/sf1-port/issues/26) Spec: Bonus Challengers — Wave 1 (Super Street Fighter: Rising) on the real engine |
| 293 | [#27](https://github.com/johnstreetfighter/sf1-port/issues/27) BC1: Widen RosterFighter (0..40), DataBank (0x10_0000..=0x37_ffff), RosterSettings/BanFormat, and CONTEXT.md glossary |
| 294 | [#28](https://github.com/johnstreetfighter/sf1-port/issues/28) BC2: 40-slot center-grid SelectScreen (framed_wide, 64x108 panels with stacked Style card, 26x30 even-row tiles, and MenuInput::Filter) |
| 295 | [#29](https://github.com/johnstreetfighter/sf1-port/issues/29) BC3: Evil Ryu (0x1c_0000) complete DataBank moveset, native SF1/SF1+ and SF2-SF6 Era Styles, ROM stand-in catalogue, and move_list.rs |
| 296 | [#30](https://github.com/johnstreetfighter/sf1-port/issues/30) BC4: M. Bison (0x1d_0000) complete DataBank moveset, native SF1/SF1+ and SF2-SF6 Era Styles, ROM stand-in catalogue, and move_list.rs |
| 297 | [#31](https://github.com/johnstreetfighter/sf1-port/issues/31) BC5: Wave 1 stages (Ayutthaya Storm & Shadaloo Temple), YM2151 FM themes (0x41 & 0x42 + pack override), composite VS/endings, and 1P Arcade ladder |
| 298 | [#32](https://github.com/johnstreetfighter/sf1-port/issues/32) BC6: Thailand Final Round 3 Ryu -> Evil Ryu Satsui Awakening (0x3ec58) and Secret Arcade Intrusions under the Exhibition Rule |
| 299 | [#33](https://github.com/johnstreetfighter/sf1-port/issues/33) BC7: Wave 1 in 2v1 Dramatic Battle (Presets 7 & 8) and 1987 World Tour (th-3-satsui & th-4-shadaloo), and retire bonus*.rs prototypes |
| 300 | [#34](https://github.com/johnstreetfighter/sf1-port/issues/34) BC8 (Nov 27 PixelLab Art): Evil Ryu 1987 sprite sheet (ROM re-palette + bespoke Satsui frames) and 6x6 VS bust in sf-art |
| 301 | [#35](https://github.com/johnstreetfighter/sf1-port/issues/35) BC9 (Nov 27 PixelLab Art): M. Bison full 1987 sprite sheet and 6x6 VS bust in sf-art |
| 302 | [#36](https://github.com/johnstreetfighter/sf1-port/issues/36) BC10 (Nov 27 PixelLab Art & Wave 1 Gate): Wave 1 stage backdrops, 14 drawn-to-size 26x30 select portraits, and flip RELEASED_WAVES = 1 |
| 312 | [#37](https://github.com/johnstreetfighter/sf1-port/issues/37) Spec: Training Drills, Combo Trials, Match Replay Takeover, Utility AI, and Control Schemes on the real engine |
| 313 | [#38](https://github.com/johnstreetfighter/sf1-port/issues/38) TRD1: Deterministic ControlScheme (Classic, Pneumatic, Modern + 20% SP shortcut scaling), Settings wiring, and CONTEXT.md glossary |
| 314 | [#39](https://github.com/johnstreetfighter/sf1-port/issues/39) TRD2: Style-Aware UtilityAi (UtilityState in CoreToggleState) driving CPU_STRUCT (0xffc664) across SF1+-SF6 styles |
| 315 | [#40](https://github.com/johnstreetfighter/sf1-port/issues/40) TRD3: Live Training Drills (AntiAir, WhiffPunish, ThrowTech, Defense) in TrainingBox, DrillController, and Coach strip |
| 316 | [#41](https://github.com/johnstreetfighter/sf1-port/issues/41) TRD4: Per-Fighter and Per-Era Combo Trials (TRIAL 1..=5 in TrainingBox and ComboTrialSpec) with live Coach strip progression |
| 317 | [#42](https://github.com/johnstreetfighter/sf1-port/issues/42) TRD5:.sf1replay (MatchReplay) binary serializer, auto-save ring in GameSession, ReplaysScreen, MenuItem::Replays, and sf-host persistence |
| 318 | [#43](https://github.com/johnstreetfighter/sf1-port/issues/43) TRD6: Real-engine ReplayPlayback, 60-frame FightSnapshot keyframe scrubbing, and 10-Second Replay Takeover |
| 319 | [#44](https://github.com/johnstreetfighter/sf1-port/issues/44) Spec: Bonus Challengers — Wave 2 (Hyper Street Fighter: Bloodlines) on the real engine |
| 320 | [#45](https://github.com/johnstreetfighter/sf1-port/issues/45) BW1: Airborne special dispatch (SPECIAL_AIRBORNE_ONLY), shared Wave 2 stage owners, and BanFormat Wave 2 rules |
| 321 | [#46](https://github.com/johnstreetfighter/sf1-port/issues/46) BW2: Gou Hibiki (0x1e_0000) & Dan (0x1f_0000) complete DataBank movesets, SF1-SF6 Era Styles, ROM stand-in catalogues, and ComboTrialSpec |
| 322 | [#47](https://github.com/johnstreetfighter/sf1-port/issues/47) BW3: Gouken (0x20_0000), Akuma (0x21_0000), & Goutetsu (0x22_0000) complete DataBank movesets, SF1-SF6 Era Styles, ROM stand-in catalogues, and ComboTrialSpec |
| 323 | [#48](https://github.com/johnstreetfighter/sf1-port/issues/48) BW4: Chun-Li (0x23_0000), Guile (0x24_0000), Dhalsim (0x25_0000), Oro (0x26_0000), & Dorai (0x27_0000) complete DataBank movesets, SF1-SF6 Era Styles, and ROM stand-in catalogues |
| 324 | [#49](https://github.com/johnstreetfighter/sf1-port/issues/49) BW5: 6 Wave 2 Bloodlines stages, original YM2151 FM.score themes (0x43-0x48), 24x24 composite VS screens, 10 endings, and win quotes |
| 325 | [#50](https://github.com/johnstreetfighter/sf1-port/issues/50) BW6: Wave 2 Secret Arcade Intrusions (Akuma Shun Goku Satsu Ambush, Master's Trial, and Bloodline Grudge) with priority rules in tasks.rs |
| 326 | [#51](https://github.com/johnstreetfighter/sf1-port/issues/51) BW7: Wave 2 in 2v1 Dramatic Battle (Presets 9-14) and 1987 World Tour (Nodes 21-25, including 1977 Gou Hibiki vs. Sagat flashback) |
| 327 | [#52](https://github.com/johnstreetfighter/sf1-port/issues/52) BW8 (Nov 27 PixelLab Art): Ansatsuken & Saikyo 1987 sprite sheets and 6x6 VS busts (Gou Hibiki, Dan, Gouken, Akuma, Goutetsu) in sf-art |
| 328 | [#53](https://github.com/johnstreetfighter/sf1-port/issues/53) BW9 (Nov 27 PixelLab Art): World Warriors & Legends 1987 sprite sheets and 6x6 VS busts (Chun-Li, Guile, Dhalsim, Oro, Dorai) in sf-art |
| 329 | [#54](https://github.com/johnstreetfighter/sf1-port/issues/54) BW10 (Nov 27 PixelLab Art): 6 Wave 2 Bloodlines stage backdrops (gouken, gou-hibiki, dorai, guile, dhalsim, oro) in sf-art |
| 330 | [#55](https://github.com/johnstreetfighter/sf1-port/issues/55) BW11 (Nov 27 PixelLab Art & Wave 2 Gate): 10 drawn-to-size 26x30 select portraits (slots 14..24) and flip RELEASED_WAVES = 2 |
| 331 | [#56](https://github.com/johnstreetfighter/sf1-port/issues/56) Spec: Bonus Challengers — Wave 3 (Ultra Street Fighter: Final Round) on the real engine |
| 332 | [#57](https://github.com/johnstreetfighter/sf1-port/issues/57) UW1: Wave 3 archetype mechanics (TRACKER_MOTION_360 & ROUTINE_COMMAND_THROW, SPECIAL_WALL_DIVE & Vega claw state, Zeku zeku_young_form 0x32_0000/0x32_8000 toggle, ROUTINE_PROJECTILE_AT_EDGE, shared stage owners, and BanFormat rules) |
| 333 | [#58](https://github.com/johnstreetfighter/sf1-port/issues/58) UW2: Bushinryu & Metro City Heroes (Guy 0x28_0000, Cody 0x29_0000, Mike Haggar 0x2e_0000, Zeku 0x32_0000/0x32_8000) complete DataBank movesets, SF1-SF6 Era Styles, ROM stand-in catalogues, and ComboTrialSpec |
| 334 | [#59](https://github.com/johnstreetfighter/sf1-port/issues/59) UW3: Mad Gear Lieutenants & Enforcers (Rolento 0x2a_0000, Sodom 0x2b_0000, Poison 0x34_0000, Hugo 0x35_0000) complete DataBank movesets, SF1-SF6 Era Styles, ROM stand-in catalogues, and ComboTrialSpec |
| 335 | [#60](https://github.com/johnstreetfighter/sf1-port/issues/60) UW4: Mad Gear Bosses & Red Cyclone (Zangief 0x2f_0000, Belger 0x33_0000, Abigail 0x36_0000, Damnd 0x37_0000) complete DataBank movesets, SF1-SF6 Era Styles, ROM stand-in catalogues, and ComboTrialSpec |
| 336 | [#61](https://github.com/johnstreetfighter/sf1-port/issues/61) UW5: Shadaloo Dragnet & Soul Power (Rose 0x2c_0000, Charlie Nash 0x2d_0000, Vega 0x30_0000, Balrog 0x31_0000) complete DataBank movesets, SF1-SF6 Era Styles, ROM stand-in catalogues, and ComboTrialSpec |
| 337 | [#62](https://github.com/johnstreetfighter/sf1-port/issues/62) UW6: 8 Wave 3 stages, original YM2151 FM.score themes (0x49-0x50), 40x40 composite VS screens, 16 endings, and win quotes |
| 338 | [#63](https://github.com/johnstreetfighter/sf1-port/issues/63) UW7: Wave 3 Secret Arcade Intrusions (Metro City Turf War, Bushinryu Succession, and Shadaloo Dragnet) with priority rules in tasks.rs |
| 339 | [#64](https://github.com/johnstreetfighter/sf1-port/issues/64) UW8: Wave 3 in 2v1 Dramatic Battle (Presets 15-22) and 1987 World Tour (Nodes 26-33, including Metro City 1987 chapter) |
| 340 | [#65](https://github.com/johnstreetfighter/sf1-port/issues/65) UW9 (Nov 27 PixelLab Art): Bushinryu & Metro City Heroes 1987 sprite sheets and 6x6 VS busts (Guy, Cody, Mike Haggar, Zeku) in sf-art |
| 341 | [#66](https://github.com/johnstreetfighter/sf1-port/issues/66) UW10 (Nov 27 PixelLab Art): Mad Gear Lieutenants & Enforcers 1987 sprite sheets and 6x6 VS busts (Rolento, Sodom, Poison, Hugo) in sf-art |
| 342 | [#67](https://github.com/johnstreetfighter/sf1-port/issues/67) UW11 (Nov 27 PixelLab Art): Mad Gear Bosses & Red Cyclone 1987 sprite sheets and 6x6 VS busts (Zangief, Belger, Abigail, Damnd) in sf-art |
| 343 | [#68](https://github.com/johnstreetfighter/sf1-port/issues/68) UW12 (Nov 27 PixelLab Art): Shadaloo Dragnet & Soul Power 1987 sprite sheets, 6x6 VS busts (Rose, Charlie Nash, Vega, Balrog), and 8 Wave 3 stage backdrops in sf-art |
| 344 | [#69](https://github.com/johnstreetfighter/sf1-port/issues/69) UW13 (Nov 27 PixelLab Art & Wave 3 Gate): 16 drawn-to-size 26x30 select portraits (slots 24..40) and flip RELEASED_WAVES = 3 (8x5 = 40 select grid) |
| 347 | [#70](https://github.com/johnstreetfighter/sf1-port/issues/70) QA: Hardware verification of GP2040-CE leverless controller (Haute42 S16) across XInput, DInput, PS4, Switch, and SOCD modes |
| 348 | [#71](https://github.com/johnstreetfighter/sf1-port/issues/71) Alternate announcer with a Japanese accent (Capcom homage) |
| 349 | [#72](https://github.com/johnstreetfighter/sf1-port/issues/72) ANNOUNCER VOICE setting, dual-pack loading, and Settings Fight! audition |
| 350 | [#73](https://github.com/johnstreetfighter/sf1-port/issues/73) Offline --voice japanese synthesis and audition sheet pipeline |
| 351 | [#74](https://github.com/johnstreetfighter/sf1-port/issues/74) Intercept classic 1987 announcer lines (0x8f-0x9b) onto the third MSM5205 for JAPANESE |
| 353 | [#75](https://github.com/johnstreetfighter/sf1-port/issues/75) Spec: Individual character themes and larger native score storage |
| 354 | [#76](https://github.com/johnstreetfighter/sf1-port/issues/76) CT1: Carry stable theme identity through existing Ryu playback |
| 355 | [#77](https://github.com/johnstreetfighter/sf1-port/issues/77) CT2: Play oversized native scores from a larger separate library |
| 356 | [#78](https://github.com/johnstreetfighter/sf1-port/issues/78) CT3: Retain interrupted themes and pending cues through library changes |
| 357 | [#79](https://github.com/johnstreetfighter/sf1-port/issues/79) CT4: Select and rotate character themes in real matches |
| 358 | [#80](https://github.com/johnstreetfighter/sf1-port/issues/80) CT5: Preserve theme choices through replay and rollback |
| 359 | [#81](https://github.com/johnstreetfighter/sf1-port/issues/81) CT6: Verify Alpha 2 and review Dan's first native proof |
| 360 | [#82](https://github.com/johnstreetfighter/sf1-port/issues/82) CT7: Verify Alpha 3 and review Evil Ryu's first native proof |
| 361 | [#83](https://github.com/johnstreetfighter/sf1-port/issues/83) CT8: Verify Final Fight CPS1 and review Guy's Slum 1 proof |
| 362 | [#84](https://github.com/johnstreetfighter/sf1-port/issues/84) CT9: Verify arcade Revenge and review Damnd's first native proof |
| 363 | [#85](https://github.com/johnstreetfighter/sf1-port/issues/85) CT10: Verify 2nd Impact and review Oro's first native proof |
| 364 | [#86](https://github.com/johnstreetfighter/sf1-port/issues/86) CT11: Arrange Ken's original World Warrior theme |
| 365 | [#87](https://github.com/johnstreetfighter/sf1-port/issues/87) CT12: Arrange Chun-Li's original World Warrior theme |
| 366 | [#88](https://github.com/johnstreetfighter/sf1-port/issues/88) CT13: Arrange Guile's original World Warrior theme |
| 367 | [#89](https://github.com/johnstreetfighter/sf1-port/issues/89) CT14: Arrange Dhalsim's original World Warrior theme |
| 368 | [#90](https://github.com/johnstreetfighter/sf1-port/issues/90) CT15: Arrange Zangief's original World Warrior theme |
| 369 | [#91](https://github.com/johnstreetfighter/sf1-port/issues/91) CT16: Arrange Vega's original World Warrior theme |
| 370 | [#92](https://github.com/johnstreetfighter/sf1-port/issues/92) CT17: Arrange Balrog's original World Warrior theme |
| 371 | [#93](https://github.com/johnstreetfighter/sf1-port/issues/93) CT18: Arrange M. Bison's original World Warrior theme |
| 372 | [#94](https://github.com/johnstreetfighter/sf1-port/issues/94) CT19: Complete Dan's arcade Alpha 2 arrangement |
| 373 | [#95](https://github.com/johnstreetfighter/sf1-port/issues/95) CT20: Arrange Akuma's arcade Alpha 2 stage theme |
| 374 | [#96](https://github.com/johnstreetfighter/sf1-port/issues/96) CT21: Arrange Rose's arcade Alpha 2 stage theme |
| 375 | [#97](https://github.com/johnstreetfighter/sf1-port/issues/97) CT22: Arrange Charlie Nash's arcade Alpha 2 stage theme |
| 376 | [#98](https://github.com/johnstreetfighter/sf1-port/issues/98) CT23: Arrange Gen's arcade Alpha 2 theme for Dorai |
| 377 | [#99](https://github.com/johnstreetfighter/sf1-port/issues/99) CT24: Complete Black Power for Evil Ryu |
| 378 | [#100](https://github.com/johnstreetfighter/sf1-port/issues/100) CT25: Arrange The Road for Gouken |
| 379 | [#101](https://github.com/johnstreetfighter/sf1-port/issues/101) CT26: Arrange Feel the Cool for Goutetsu |
| 380 | [#102](https://github.com/johnstreetfighter/sf1-port/issues/102) CT27: Arrange Performance for Gou Hibiki |
| 381 | [#103](https://github.com/johnstreetfighter/sf1-port/issues/103) CT28: Arrange No More Swingin' for Zeku |
| 382 | [#104](https://github.com/johnstreetfighter/sf1-port/issues/104) CT29: Complete Slum 1 for Guy |
| 383 | [#105](https://github.com/johnstreetfighter/sf1-port/issues/105) CT30: Arrange Subway Park 1 for Cody |
| 384 | [#106](https://github.com/johnstreetfighter/sf1-port/issues/106) CT31: Arrange Slum 2 for Haggar |
| 385 | [#107](https://github.com/johnstreetfighter/sf1-port/issues/107) CT32: Arrange Subway Park 2 for Sodom |
| 386 | [#108](https://github.com/johnstreetfighter/sf1-port/issues/108) CT33: Arrange Industrial Area 2 for Rolento |
| 387 | [#109](https://github.com/johnstreetfighter/sf1-port/issues/109) CT34: Arrange West Side 1 for Hugo |
| 388 | [#110](https://github.com/johnstreetfighter/sf1-port/issues/110) CT35: Arrange Bay Area 3 for Abigail |
| 389 | [#111](https://github.com/johnstreetfighter/sf1-port/issues/111) CT36: Arrange Bay Area 1 for Poison |
| 390 | [#112](https://github.com/johnstreetfighter/sf1-port/issues/112) CT37: Complete DAMND for Damnd |
| 391 | [#113](https://github.com/johnstreetfighter/sf1-port/issues/113) CT38: Arrange BELGER for Belger |
| 392 | [#114](https://github.com/johnstreetfighter/sf1-port/issues/114) CT39: Complete Oro's arcade 2nd Impact arrangement |
| 393 | [#115](https://github.com/johnstreetfighter/sf1-port/issues/115) CT40: Accept and install the complete local character-theme pack |
| 398 | [#116](https://github.com/johnstreetfighter/sf1-port/issues/116) Spec for discovering SF1+ combos with an engine timing sweep |
| 399 | [#117](https://github.com/johnstreetfighter/sf1-port/issues/117) Find and replay Ryu's crouching light kick into Hadoken |
| 400 | [#118](https://github.com/johnstreetfighter/sf1-port/issues/118) Sweep an SF1+ fighter's normals against both defender stances |
| 401 | [#119](https://github.com/johnstreetfighter/sf1-port/issues/119) Sweep designer-authored inputs for a signature move |
| 402 | [#120](https://github.com/johnstreetfighter/sf1-port/issues/120) Dramatic Battle: native era styles and Team Era Gauge |

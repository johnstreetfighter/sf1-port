# SC20: SF6 stronger Fair CPU measuring report

2026-11-25. Version `1.5.32`. Ticket [SC20 #212](https://github.com/johnstreetfighter/sf1-port/issues/212).

96 of 96 new-versus-old cells meet the nominal win and exact weighted-idle bars; 72 of 72 adjacent-level cells meet the nominal higher-level win bar. 0 win bars miss under the all-attempts denominator, including 0 additional misses beyond nominally passing cells.

**Readiness:** All recorded win/activity bars pass, including the all-attempts win checks. Separately, 218 engine-stopped attempts remain an engine evidence gap; untranslated-branch work is tracked in [#270](https://github.com/johnstreetfighter/sf1-port/issues/270). These attempts are included in the all-attempts win denominator and are not completed matches. These measurements do not supply the required maintainer playtest or whole-cast enablement approval. The developer-only stronger CPU remains off by default. Its CPU wire version is unchanged. [SC24 #216](https://github.com/johnstreetfighter/sf1-port/issues/216) retains the whole-cast enablement decision and maintainer approval.

## Source and implementation

Every final row uses measured source [1018d72](https://github.com/johnstreetfighter/sf1-port/commit/1018d722f308930758e12f092df29e0675dfd822) (`1018d722f308930758e12f092df29e0675dfd822`), frozen executable SHA-256 `44c0d3a5bca73ed126dba846f90518e6af18efa0387037bb3033f55e7b4bb5da`. Delivery source: `1018d722f308930758e12f092df29e0675dfd822`.

The eleven remaining tournament fighters in SF6/Standard use the shared situations, their own game plans and route books, pressure, focus, habits and four difficulty levels. Ryu retains his established table and behavior.

- Deliberate native Drive Rush, Drive Impact, OD and affordable Super Arts use actual own controls, costs and gauges. Parry and Reversal remain delayed reactions. Easy keeps self-started paid commands out of its ordinary motions, including accidental cancel Rush inputs.
- Fighter maps use native SF6 Ken/Retsu and each what-if kit. Geki has Execution Drop and teleport; Lee and Birdie charge; Birdie has Murderer Chain; Gen changes one way to Crane and uses Jasen. Retsu SA3 is tested as the strikes the native engine implements.
- Only the new SF6 cast gains the existing blocked-punish path. Impact uses a free close decision window; Geki teleport and Gen charge priorities provide real opportunities alongside higher-scoring alternatives. The sixteen snapshot slots, default-off developer switch and CPU wire version are preserved.

The comparison baseline is the committed version 1.5.31 source [095562d](https://github.com/johnstreetfighter/sf1-port/commit/095562d769850796f5fd74cf80c515f311559d3a), after the preceding SC19 SF5 rollout and Adon Jaguar Tooth native fix. This report does not infer another native bug fix from CPU scores or classify an unfinished row as a proven native hang.

## Method and complete accounting

The grid covers all twelve tournament fighters in SF6/Standard. Each pair has 8 primary cells (four levels on both sides), 6 adjacent-level cells, and 1 old/old Very Difficult control: 180 cells and 43,200 attempted matches. Eleven newly covered fighters contribute 165 cells; the established Ryu pair contributes 15 regression cells.

**43,200 attempted; 42,982 completed; 218 engine-stopped; 0 unfinished; 0 completed draws.** Seeds 1-240 occur exactly once per cell across raw match rows and explicit failure lines. Worker threads: 2.

| Group | Cells | Nominal pass / miss | Attempted | Completed | Stopped | Unfinished | Completed draws | Pessimistic win misses |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| primary | 96 | 96 / 0 | 23040 | 22957 | 83 | 0 | 0 | 0 |
| adjacent | 72 | 72 / 0 | 17280 | 17155 | 125 | 0 | 0 | 0 |
| old-old | 12 | not graded | 2880 | 2870 | 10 | 0 | 0 | not graded |

Completed wins/losses/draws require `ended=yes`. Unfinished rows are separate, including a `winner=-` row, and prevent a nominal pass. Primary nominal bars require wins at least half of all recorded rows and strictly lower idle; adjacent levels require higher-level wins greater than half. The separate pessimistic win bar includes all 240 attempted seeds, including every stop and unfinished match, without relabeling them as completed losses or draws. A pessimistic win pass does not waive idle or unfinished failures.

Idle and attack rates use all recorded live frames. Each two-decimal metric is parsed as exact Decimal hundredths, then multiplied by integer frame counts. Strict idle decisions compare integer weighted sums before any division; mathematically equal idle cannot pass from float rounding. Division and two-place rounding are for display only. Stops have no live rates. Missing/duplicate seeds, invalid identity/metrics, unexpected grids and partial files are rejected; cached summary JSON does not establish any result.

Levels 1-4 mean Easy, Normal, Difficult and Very Difficult. First/second means new/old in primary rows, higher/lower in adjacent rows, and P1/P2 in old/old rows.

Equivalent grid commands reconstructed from manifests, not an invented invocation history:

```text
rtk proxy python target/sc20/measure_sc20.py final --frozen "D:\sf1-port-storage\build\original-target\sc20\bin\measurement-1018d722f308-current.json" --out "C:\Users\George\Projects\sf1-port\target\sc20\final-1018d72" --threads 2
rtk proxy python target/sc20/measure_sc20.py controls --frozen "D:\sf1-port-storage\build\original-target\sc20\bin\measurement-095562d76985-baseline.json" --out "C:\Users\George\Projects\sf1-port\target\sc20\baseline-controls" --threads 2
rtk proxy python target/sc20/measure_sc20.py controls --frozen "D:\sf1-port-storage\build\original-target\sc20\bin\measurement-1018d722f308-current.json" --out "C:\Users\George\Projects\sf1-port\target\sc20\current-controls-1018d72" --threads 2
```

## Switch-off and established-era compatibility

The 388 controls cover 12 SF6 switch-off pairs, 14 established Ryu SF6 level/adjacent cells, 118 SF5 cells (all 59 pairs on both stronger sides), 120 SF4 cells, 96 SF3 cells and 28 representative SF2/SFZ cells. Every control file accounts for 40 attempted seeds. Both complete grids were revalidated from raw data and all 388 TSVs, including stop lines and unfinished rows, compared byte for byte.

**388 of 388 control files match byte for byte; 0 differ.** Compatibility here means unchanged measured output for these cases. It does not turn their baseline stops or unfinished rows into successes.

| Controls | Attempted | Completed | Stopped | Unfinished | Completed draws |
| --- | --- | --- | --- | --- | --- |
| Baseline | 15520 | 15324 | 196 | 0 | 0 |
| Current | 15520 | 15324 | 196 | 0 | 0 |

The SF6 switch-off subset is reported separately so failures outside the new rollout are visible:

| SF6 switch-off subset | Cells | Attempted | Completed | Stopped | Unfinished | Completed draws |
| --- | --- | --- | --- | --- | --- | --- |
| Baseline | 12 | 480 | 480 | 0 | 0 | 0 |
| Current | 12 | 480 | 480 | 0 | 0 | 0 |

### All 388 paired control files

| Raw file | Baseline completed / stopped / unfinished | Current completed / stopped / unfinished | Raw SHA-256 (identical) |
| --- | --- | --- | --- |
| ryu__sf6__old__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | c2bad074b3dcbeaf7b86818f1833c43265c6313a72ec55d27a7aea00709b9785 |
| ken__sf6__old__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 3df49539d37fa8e68dd47c22a4cc95d11c90c68617546f3059bee1d52f3cdb32 |
| retsu__sf6__old__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | aee7a11f1d48ebb83073597111108880e55915a646440d9bae0bebd12578138a |
| geki__sf6__old__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | d990531d3dad95a97374300ea98892fe0c28d56203c96d1b5ba8b170457562cb |
| joe__sf6__old__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | b1b029766dcd69f1226d7043f2bd3aae83f90a84b9ba4238ad713218d8481851 |
| mike__sf6__old__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | aec737b81a4e2e3c82f8ef5e8501a0b6c5e099c72b192aa460758ceb9c55522b |
| lee__sf6__old__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 257a969eb776a46039521ac93edab9b6271514ea2f70079306d36261b638366f |
| gen__sf6__old__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 89c2155d9b020d97212c3b3b7b29ea21af5c321eece722ad37d245937e660310 |
| birdie__sf6__old__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | a3a06f3fffe2f336d4c30659a6d095d2a752508cca93820175f328d603a35a7e |
| eagle__sf6__old__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 0324fbdb2fe3d43efed059ebe84c0ecd56328452f46e94bb60d42377ab4a2074 |
| adon__sf6__old__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 60cf1d9e7b7b9b248fe4954c72a8a957b761784a794c7c70f16e95c5bac44194 |
| sagat__sf6__old__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 4218b0055c4228bedd2b662ed4904811c8700c2e59dbf68d19d7864ea5d9153b |
| ryu__sf6__new__old__1.tsv | 40 / 0 / 0 | 40 / 0 / 0 | f1e7ee52bce2ccf2d28d5fd290ad7b28f95b51454d3ec156029d508c6ac0be90 |
| ryu__sf6__old__new__1.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 653291c50f0003a036553517056ffc322b8522c83dd393baf3492d75a940f314 |
| ryu__sf6__new__old__2.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 46462994dcfaa1f7aed0be4f7ef218f6e9535def7e401d180856df40585c142e |
| ryu__sf6__old__new__2.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 33856b774a0cd7c0ef0197acaa4926c9850bfca8e7ee0e4b425f007be5f2b9be |
| ryu__sf6__new__old__3.tsv | 40 / 0 / 0 | 40 / 0 / 0 | a48feab4df019b2d7f14704395c0bb880ccfe53c2d5f1d9a5d6f74fd6944f3bc |
| ryu__sf6__old__new__3.tsv | 40 / 0 / 0 | 40 / 0 / 0 | dab34b2321f54dfbbf60352723f89c88fc4676f87ccc96c473e341dfe5ba27bc |
| ryu__sf6__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 1cc4392fab667f4d65aa2284938cc56f22cd75791325cce979ae2f5adf64e3c6 |
| ryu__sf6__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | e9f86413d5b133d5a0275a3ec194920b6b5d1ddcf74bc51b405e42794e381373 |
| ryu__sf6__new__new__2-1.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 025cfaeba69f8df94cb96e4b501582b11f5dd646858164f6273108e57cef79e2 |
| ryu__sf6__new__new__1-2.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 15507cc38aab49168a5db49f9a69298844230e19e096016ae60a5aceeeeb89c8 |
| ryu__sf6__new__new__3-2.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 0c8393088fe80577c019de1623a57c0ca9d73a73a544efb4408fd294fd40fa50 |
| ryu__sf6__new__new__2-3.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 3799d3d3ff2d958344599d125c2624e5bf67f18554c3a8f88b646f771205d69a |
| ryu__sf6__new__new__4-3.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 3d538593fabefecfc61747f9115444521ead62ff96cf8b5442f966780416bfb1 |
| ryu__sf6__new__new__3-4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 6cd0d1768cd8b9ac91ca0b19b1a9613faf808ba7ea3a4f8f839d39a532619514 |
| ryu__sf5-skill1trigger1__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 25f4d4d053f3b61f77b600f614c8e2440457603d85b4fae9787a6e0da0a44395 |
| ryu__sf5-skill1trigger1__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 2e0a95c0c5e204d047e80fa45f249e845a6aac1ed8d92036ab7dc78ccd617f57 |
| ryu__sf5ae-skill1trigger1__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 23291714924f60dfa0c1b104ccadc0f33b80e8a462c48a236fb8d7a39d5ef2bb |
| ryu__sf5ae-skill1trigger1__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | f0af39496512791321b04d5cad33685ff22af49fb140a21698ee53d71707344d |
| ryu__sf5ae-skill1trigger2__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 775022e22480fd5def63bcd435cdf8a0f375bdc93ab3fe34c0877e0d5989879a |
| ryu__sf5ae-skill1trigger2__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 7aed515211f7e1123e0531eab17a363b85196c347e2a3364843805d2420cc2cb |
| ryu__sf5ce-skill1trigger1__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 6eb306047e729e0c898dec963bbb34b24f40f6d559779544554f550829427b35 |
| ryu__sf5ce-skill1trigger1__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | a4fe678e31f70a39ca4fc59adb991803992fa8bacab128b1290614a481235533 |
| ryu__sf5ce-skill1trigger2__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 949d1905918dbd55fafc28cbc5ce000791411433d6e9769cca2fdeb7654ba8ed |
| ryu__sf5ce-skill1trigger2__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 3b4165a65ecf5a6fd7635751c5f4c9f1d0f032d03518dc42107b72aeb993f5e9 |
| ryu__sf5ce-skill2trigger1__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | fb928b38d5b535732df6f7f1d3cc920c18c4b4a05622596574f4baf91085caec |
| ryu__sf5ce-skill2trigger1__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 746cc6dd8927b8dc1800e9946a98083afd12b0f15f09911c1bb5d5e1091ea02e |
| ryu__sf5ce-skill2trigger2__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 3bdb568c5503ec2dea7da4c724ec56c73700f18a2ff7d63d27353060fd4a90bb |
| ryu__sf5ce-skill2trigger2__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 52be4a1e5b2d1ffd1956d4ac54c657e1caa9beacdee9361e6b25886c47e6486e |
| ken__sf5-skill1trigger1__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 76516cc93f822835bdbde271a21f07ba76f30902775677602dad84aa3fbd0a47 |
| ken__sf5-skill1trigger1__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | b4c83bee864f0a4bfb45b7ee50d180965961a9fb45d4f59184bb0047b088793d |
| ken__sf5ae-skill1trigger1__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | ecc998d236bd8901f2ffa3d6fa19c812741e2fa3f7ee888639f723623ed4fc81 |
| ken__sf5ae-skill1trigger1__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 6f4e4524d1169730619a327b95caf16f5eaa31a33704d55037b31809d7c4aa1a |
| ken__sf5ae-skill1trigger2__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 0791d2700ccbbe9a8888407ee051eaafde344ebe75b94be1a49fe011f895998d |
| ken__sf5ae-skill1trigger2__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | b837851d7ab7a920d6ba1d41e1348d48974eff3784bb660cecb9dd7f008f8a8c |
| ken__sf5ce-skill1trigger1__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 6ba65240feab674d04a8d5940df4c9850c0acb9823751fbe233034b414bc39aa |
| ken__sf5ce-skill1trigger1__old__new__4.tsv | 39 / 1 / 0 | 39 / 1 / 0 | eff681ac9ffff502ee5ad0e271fd2260df6ceb1f594680d1c4315e4d4b70baa0 |
| ken__sf5ce-skill1trigger2__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 9b73a3a3616ae0359a863ef3da55ddc2358266b7cebebb383ae4e84c7b9f9624 |
| ken__sf5ce-skill1trigger2__old__new__4.tsv | 39 / 1 / 0 | 39 / 1 / 0 | 4f21ae1f111a7d5a930cb7dfe4150114d31734e9986109cddd75297d5e25a355 |
| ken__sf5ce-skill2trigger1__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 76fec1c140dfd222fdf2b39788d25c51aca626b8f00cc3e703fecc8b30346d74 |
| ken__sf5ce-skill2trigger1__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | a80ffe6e9efb510047426665f3a55420e4b139228bd975c25c2913e6266e09b4 |
| ken__sf5ce-skill2trigger2__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 9084a2839e375d1a0f947d1a2213ecdbffdc80eb353fd63389353b33bfc56a3d |
| ken__sf5ce-skill2trigger2__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 975f8771698e6ca6486dc8b6e000b6d1260c361b3bdaeb710245a79494c6afbe |
| retsu__sf5ce-skill1trigger1__new__old__4.tsv | 35 / 5 / 0 | 35 / 5 / 0 | 51fc25ad38d0f656dc4ef90e61ac0e3e78d1cefa1c26744c36f9e43ca368a71c |
| retsu__sf5ce-skill1trigger1__old__new__4.tsv | 34 / 6 / 0 | 34 / 6 / 0 | 3f2aa90663482b3be9770a1d2618ec3626af80ea82726298c179f822415f9418 |
| retsu__sf5ce-skill1trigger2__new__old__4.tsv | 35 / 5 / 0 | 35 / 5 / 0 | d3ae6db2936b62629b226c755a3f9f2e3ac17dd90b173b4928854e391551dc7a |
| retsu__sf5ce-skill1trigger2__old__new__4.tsv | 34 / 6 / 0 | 34 / 6 / 0 | 66d098b6aa839c9650bda64044bc71bc8a8757a2805067cb53253e84b8694c72 |
| retsu__sf5ce-skill2trigger1__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | b90bc3df7da60c703efc6039ec8d8cb8e9b8044a44b5979fbe07feb32e65c8c5 |
| retsu__sf5ce-skill2trigger1__old__new__4.tsv | 39 / 1 / 0 | 39 / 1 / 0 | fa9c41d040429270d0a4fa3ff2aac3abb6567a799ee84ba8a73848562e0ca0e5 |
| retsu__sf5ce-skill2trigger2__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 3f678b11a6de44fef78cf20e16357e295d7c8140aafd77800e42d8570a482c52 |
| retsu__sf5ce-skill2trigger2__old__new__4.tsv | 39 / 1 / 0 | 39 / 1 / 0 | 1ca481ef091728cfcc96ec92ed42cf324c5c6d2e40d2482184048d120b424c91 |
| geki__sf5ce-skill1trigger1__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 86f59e2434d7b6fa190d1d312c6b2bee2c52ecf9838f8c7bda9172322ad5db8e |
| geki__sf5ce-skill1trigger1__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | b090468bdbcd04d837e2ac240ddcdda0409ad00a65500b55265b317dbd26c0bd |
| geki__sf5ce-skill1trigger2__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 07e86923ed1f22da63a1ab18d7d8dc8480486158ac5d7bcabe11792050520402 |
| geki__sf5ce-skill1trigger2__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 0788b3653f06ddee2e22381659e9e1bba535a28cfce736c74f3bc2a01e1d3b15 |
| geki__sf5ce-skill2trigger1__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | b94e2208287af0f49fd16725afb52474eeea9403edc016a23c313e821d2309aa |
| geki__sf5ce-skill2trigger1__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 5d93137a31bff64a740690f9860c8197359c5f9ec969bad0bd66fadbf2a1d273 |
| geki__sf5ce-skill2trigger2__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 44040e210033ec97eee2b831e340fa8c511740e882457cdbeb86274b1dd7fed5 |
| geki__sf5ce-skill2trigger2__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | ed120e7de419028fb3a54ca88fcf4eff749d04571e40e215123e1009d3c77c14 |
| joe__sf5ce-skill1trigger1__new__old__4.tsv | 36 / 4 / 0 | 36 / 4 / 0 | 71557d583324d010c088f248fae258c8ec2eb6074c21565f540dfadc0641f2b9 |
| joe__sf5ce-skill1trigger1__old__new__4.tsv | 38 / 2 / 0 | 38 / 2 / 0 | b60de7413a4068eadc218930e257987fa7e6676afcf2f5c8054813b5ecde60b6 |
| joe__sf5ce-skill1trigger2__new__old__4.tsv | 36 / 4 / 0 | 36 / 4 / 0 | af623f68a64d6b7b0d0bdfbb1042b023ae7110a63b368366212ce958a40a84f2 |
| joe__sf5ce-skill1trigger2__old__new__4.tsv | 38 / 2 / 0 | 38 / 2 / 0 | bbf12c0f692a547fc46d0f48c8411c594ba4e9efaa32bdb326cbadf13024b1f4 |
| joe__sf5ce-skill2trigger1__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 322f6089354879b4b5bd6b17cf2a69a57734a6b5cc902f9520549f5aeaf14d98 |
| joe__sf5ce-skill2trigger1__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 2ac2c5f4eff57106e2793946a91b88600e354cd1039c8b76c657e022772f501b |
| joe__sf5ce-skill2trigger2__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 91bfbdbcfa14f18a858bad41d508a6ff3e85f8015e7caa098e3dbf24ce51dbde |
| joe__sf5ce-skill2trigger2__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | c004ae4df4595dcc778f3f9023f6b4df88ff66d5221fe8f87673019650cb5d11 |
| mike__sf5ce-skill1trigger1__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 9ea83e71626dd022e416b7dd4f8fd42f4585919db64b9ddc6967be2c2edcf5eb |
| mike__sf5ce-skill1trigger1__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | bd1e88e50c247e468a96723fcf37b6dd98af65e9e168cf535bea883c23cfee40 |
| mike__sf5ce-skill1trigger2__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 62ce70e1b68d3c72bd95c258d5ab5a9368c721cbfb4c551a20e7dba505fa3d42 |
| mike__sf5ce-skill1trigger2__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | abef35a7d778e38e6b1329f0945818bf326ce1412d58f73b4d18d02dd3513b13 |
| mike__sf5ce-skill2trigger1__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 2062e1c37515ff0047e3dbca64c43a5f13160e51c7a293e4eb80598e944f449f |
| mike__sf5ce-skill2trigger1__old__new__4.tsv | 39 / 1 / 0 | 39 / 1 / 0 | 353ac079f4a920e8caf73bb738688d6c896cff8396035548618be8519d866a30 |
| mike__sf5ce-skill2trigger2__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 8b0eb87a3e46d665bc8933c7d1d81e5cf061932c324084c9afce33ac10d09e29 |
| mike__sf5ce-skill2trigger2__old__new__4.tsv | 39 / 1 / 0 | 39 / 1 / 0 | bbe027f89cde8da96d9659c1717f6994fec466a25617b710df5f436a02f43b11 |
| lee__sf5ce-skill1trigger1__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | e5363df0b1d3ddc909e2da4d01089e94df00590c65224156d3d0474835a4a1f3 |
| lee__sf5ce-skill1trigger1__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | d830fee27d5d63bde2bb889872957dd07c5cacbba1ba1a228070c75824898174 |
| lee__sf5ce-skill1trigger2__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | fc5ef8e7b8169b9ff93920bbd4c2bb7ce6671b3fb6b014d4ab4f66bad177d945 |
| lee__sf5ce-skill1trigger2__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 4b532b472991ef4cfc28e60e5bda544039696f6b49f4b4a3b0774b14716aa957 |
| lee__sf5ce-skill2trigger1__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 1e88fd486f686d0c443937ae918935d09815c50f2ceed5c28ec460b9f899b5cb |
| lee__sf5ce-skill2trigger1__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | cfaa17febfd7cebe7b37fda6afd8c5fe86dc3e73925ddeb4096df427fd82297a |
| lee__sf5ce-skill2trigger2__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 0ff6bccfccbb16b6ed5ced023b0c0a1209e40805efde92be88f1ded83a8bad71 |
| lee__sf5ce-skill2trigger2__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | e8f7c21d0d40ab746a7bfc2fcc3306765a8c47434c2eef5461104b97b5202c8c |
| gen__sf5ce-skill1trigger1__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | b266b12a67fc9e6a579c3f9d3e8e2841749999833e0f0c611f6976bd655bf603 |
| gen__sf5ce-skill1trigger1__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 5164790a238e935cbd35085daa9908c63254abbdcd76a6c1118c3180a43c6d84 |
| gen__sf5ce-skill1trigger2__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | dd90d066caed7207539cbeac4dc3060f4786406cf4587bfee2fa702249127e4d |
| gen__sf5ce-skill1trigger2__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 4f6b934bad412e55ff77dcfef2bd138764d3661457444452c732e79f82d18bf7 |
| gen__sf5ce-skill2trigger1__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 974411ebbc85b043403bf9a780f29a687afb2eb372bfd0b20187b2878f488590 |
| gen__sf5ce-skill2trigger1__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | bb99474cd85c87bdc81282a6da868e42a56f69fb6e3fe2c1b15bc9749aa2c732 |
| gen__sf5ce-skill2trigger2__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 78bd9ce4f65ea1ab1dd4bcc9272d444fdd1b5cad94bf74bec352d835a98e254d |
| gen__sf5ce-skill2trigger2__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 7a8a9c0c35b2e850fa985b9b0fd1d8e15057de4f0f57f1dfbc3f7ff7168322f0 |
| birdie__sf5-skill1trigger1__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 3a26a21b127694fda7d22109ec0aaf463b80dab93cc2e1057fe6ce9acf09e3a2 |
| birdie__sf5-skill1trigger1__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | db25566a38c448edcee18dbde39c7ab1ddb43b61537d709b69e19e0bcd868634 |
| birdie__sf5ae-skill1trigger1__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 4327160a3a7dbadc69b5d4a6615a47f190cc8057c3f802e7e7e813e65b0d70e9 |
| birdie__sf5ae-skill1trigger1__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | a6130c14775eee9f29266224e4caf958cb90383141e85c2ebdb0cc41b68a41e6 |
| birdie__sf5ae-skill1trigger2__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | ae71a9d22882774b7cab92f48d27a4ba48beeefd5b89ddbd6613bca0f5dd99b2 |
| birdie__sf5ae-skill1trigger2__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 6a425f0f7c2f12a4a8fff98ad5db282b450cb11681fcdfcd926193fd2db640f1 |
| birdie__sf5ce-skill1trigger1__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | a7bb4d6b6b3ddc102730879642112aeec71206287798f223f94e6ffb26373a1f |
| birdie__sf5ce-skill1trigger1__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 5e364434d5f8aa9653b7acb159364d9d950ba6c6d204190200a590eb8b3846c4 |
| birdie__sf5ce-skill1trigger2__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | abaeb87e7e5d0874157d88e69da6430088378261bc34fbb3db39464c4ed68848 |
| birdie__sf5ce-skill1trigger2__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | ff6695eae1f3bf2bf5c63c5616fb7ac5974f9e906af95a781633856c95c5176c |
| birdie__sf5ce-skill2trigger1__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 71013ceb2d7ae115fa3a83019722ddf31303e60fcfcb3dd0b7110226f1693895 |
| birdie__sf5ce-skill2trigger1__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 2d01900ee830989ae26fb15cc3b3f4a32df2f25a89e794c030826f555eca7855 |
| birdie__sf5ce-skill2trigger2__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 219088e443292f5ac02bc9fac4055ca8ec36da9a460773f7c3c35f47c33edd98 |
| birdie__sf5ce-skill2trigger2__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | e71712434415ec6b459d711eb0c52bdd8045b20e2a1eb84288e480edb3834cc5 |
| eagle__sf5ce-skill1trigger1__new__old__4.tsv | 33 / 7 / 0 | 33 / 7 / 0 | 3b21ced1c6a1b5e95d9d14de06126073698ec939dc7a730ad139cd4ec85faa83 |
| eagle__sf5ce-skill1trigger1__old__new__4.tsv | 38 / 2 / 0 | 38 / 2 / 0 | e33e0a5c8252175356cb738c8d713959fea4f10943ab3a8eddda471b0e2f8e3f |
| eagle__sf5ce-skill1trigger2__new__old__4.tsv | 33 / 7 / 0 | 33 / 7 / 0 | d8f806cb855f6c904425b6d989e11d466b34b93cae38ef2fde0f8a922396ecd0 |
| eagle__sf5ce-skill1trigger2__old__new__4.tsv | 38 / 2 / 0 | 38 / 2 / 0 | c36a0c2c78238ef6603d711d0e7ca3e161fe6972efc401053068b83ca1abfacf |
| eagle__sf5ce-skill2trigger1__new__old__4.tsv | 39 / 1 / 0 | 39 / 1 / 0 | 91e9e596464612846f820e0cb4ccff4984d5b0165d9accb4911a10e034ae6d83 |
| eagle__sf5ce-skill2trigger1__old__new__4.tsv | 37 / 3 / 0 | 37 / 3 / 0 | 69abbfd317578946282787cbc1743286430a109933171fffa98685be92bb6624 |
| eagle__sf5ce-skill2trigger2__new__old__4.tsv | 39 / 1 / 0 | 39 / 1 / 0 | 3a761229b947cdd93885e88bd14205cac415ab279dab5d2efe7f513711e49625 |
| eagle__sf5ce-skill2trigger2__old__new__4.tsv | 38 / 2 / 0 | 38 / 2 / 0 | 5002f5eb1bc330384524c9d212e76ac18dddb2939978fb8d6e9864764ccdf19d |
| adon__sf5ce-skill1trigger1__new__old__4.tsv | 39 / 1 / 0 | 39 / 1 / 0 | 459e8b6405d7db939e33161f5c2e0566e61fef2efbbf83620424200eb1368ad8 |
| adon__sf5ce-skill1trigger1__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | ab0d593e0ccd6e591a6a9ebe9e73c899063c9c6d03ab42dbfbbec6213032a82a |
| adon__sf5ce-skill1trigger2__new__old__4.tsv | 39 / 1 / 0 | 39 / 1 / 0 | 897fb63ab769e142ce34490d3b927ec5b928df088fc15a9e4d2bbc3f09bb4093 |
| adon__sf5ce-skill1trigger2__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 65b022a63f3d8b143555a0864150872c851de256e99151ea06da04ff18e800fd |
| adon__sf5ce-skill2trigger1__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 99e76f3adba63f9dc1300ae490b1e2531862931af50d6c50882e629564371913 |
| adon__sf5ce-skill2trigger1__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | e4edec6e5eccbc11cccfcbaa18e90a1c1c647e6d731f9666d38f91865d375794 |
| adon__sf5ce-skill2trigger2__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | b274f50911208fc0e55270fd10b767ae135d804ef46c378180c63ee2ac27aaf3 |
| adon__sf5ce-skill2trigger2__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | c5f93bc72c8096bac01b71d95520093a7ec6656f74f826f16b55424e037819cf |
| sagat__sf5ae-skill1trigger1__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 408aba69d622ab41d415f85c1e8611bba9948d7ece61d1ebb522a0cff2aa0716 |
| sagat__sf5ae-skill1trigger1__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | e4b9ecd98ff1f59ac78dd21b7ce49252dff37707a097fe44bd11c137d84749c0 |
| sagat__sf5ae-skill1trigger2__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 2e12252d32ed200eb0d4056c6fe4887e729a584f69b9c4b10da22da665630e50 |
| sagat__sf5ae-skill1trigger2__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 8d5bc621697e129aaeaf9125353af4b7ac30419e3b35fcced1e416e4e32682c3 |
| sagat__sf5ce-skill1trigger1__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 410d3abc5610b5d0658606262138268f84eec32f2f9825fd07b00a644fac7fcc |
| sagat__sf5ce-skill1trigger1__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | fc27f8aeea44fd26351180896a632d36ec1eeb5e34dd0569b12b30b32d0203b5 |
| sagat__sf5ce-skill1trigger2__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 1d1c52ec988110e4c44ee805adb2e1145f2705f5429de15eea323cf8c9fa33f2 |
| sagat__sf5ce-skill1trigger2__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | d3b26722af198a5227925cade526a8670a8b4a193604d96b75e2f66857c15d61 |
| sagat__sf5ce-skill2trigger1__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 6e1f157f86ac64139b20080953616691fa74a2ac5ea0f97dc6a4ccee300a5d8b |
| sagat__sf5ce-skill2trigger1__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 54ecb29a1af2fcb590957103e41dcb7f38281adb073163120db176d7a90487e5 |
| sagat__sf5ce-skill2trigger2__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 1cff8062ee19c2df9b748b2f49e67ad88d76fa39946aacf6eab3b87f6bbe56b6 |
| sagat__sf5ce-skill2trigger2__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | ea01eb6aab0f2eb6490cd8af37df1322d89fc1dbeeb3733ee2ed84bafd37da7f |
| ryu__sf4-ultra1__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | cb169808fb5128dc1c8369362e1262644800ab06b9dbcdb35b7b2a2a294d191d |
| ryu__sf4-ultra1__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 323017c6af20fb274788a9dbdc431f8c510c8bc9a99a1a61be204ea9750e0176 |
| ryu__ssf4-ultra1__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | a6a3777000ed7980eb04dc2ad8484728a84d11eb443d922b4b549b5a7e82fc81 |
| ryu__ssf4-ultra1__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 9d03f5aa79bca9222aa4776fede207be19ab04c44c7a57039d7898318a618c10 |
| ryu__ssf4-ultra2__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | c925fdb2c87f6dab9744da17a9edf6a578dfbb4f3b2a891d8b4707976a23d2f2 |
| ryu__ssf4-ultra2__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 18cdc78f1aa2273bae065cf3faef16101b4d2b5d7e1892cfdfe91a9d250c4f7e |
| ryu__ae2012-ultra1__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | c8e7d37237b131b60c1b321b7e2e1c589b5a436f877f4117f35858de303a57fd |
| ryu__ae2012-ultra1__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 78a01dfae46b0e069d2eb44fc9cda65074bcc43621aa8278721891b98f25f54d |
| ryu__ae2012-ultra2__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | abd36f70d6998cae68595b823ae2859360c7fb89e5cdfe4c1b8d077a2ab8968f |
| ryu__ae2012-ultra2__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | c2a6adffed4ad0df87027e486028722f243c866fb513ac87557b9fc95a092922 |
| ryu__usf4-ultra1__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 2c9ec1a1fb798639097dba0b05e913474b9930269da761bb7889fe395728b376 |
| ryu__usf4-ultra1__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 3b1c7df8ab6f0018c239d795192260d1ba35f5e89ac47b1db3d8bb29374b772c |
| ryu__usf4-ultra2__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | a42b2e620a2cf362b0555122ec90b039f1577701d1e54f52783b4fbfda007f24 |
| ryu__usf4-ultra2__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 1125ae8a4d12ec6c2f497d6a1fb7b96161bb5e2d54b1528124d7c043f918cae2 |
| ryu__usf4-ultradouble__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 6361c3dfb6020da315da5c2b6b56175be787e692f07c43a599f03e90f1dd2d72 |
| ryu__usf4-ultradouble__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 11511c7b45825a301281b5f2f82a0c9383c2377e9503750ac785244b116b0615 |
| ken__sf4-ultra1__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 2d686c53fc6d62c3fdefd4f3c784979d2a261135ff407149eec9b74e0344e6aa |
| ken__sf4-ultra1__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 7e763a16b83a3f136eba4991fa7c88490bd00944d7d9080ccdd86fcd0ae2ca5b |
| ken__ssf4-ultra1__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | a140eb84bf66e168cfc3957ecc485431e0bbd100ad0f418ec6b591908aa07d3b |
| ken__ssf4-ultra1__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 0db3f4f8485f7c41ae8f4be5abe65f7b60ffc0e00cc1b06221e8efc8948a1239 |
| ken__ssf4-ultra2__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 35cfc446f27f79d84587ae85e68fbecd8ecc4891df151235eaf3cd8a6d8abc11 |
| ken__ssf4-ultra2__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 301992d7234722a1f72e47782403cfdee1dd7b7ab55928324d85da35ff160af4 |
| ken__ae2012-ultra1__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | f1baf3565168d8ca3746b9c910c0cb082f846a297e0036c1e07d2483a7d1f78a |
| ken__ae2012-ultra1__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 12cef878a635eeb611368d8238a49e8dbf6a52d413323e5699f35fb558f6db90 |
| ken__ae2012-ultra2__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 139e9410d7ee6d64abeece5a27d8d17d021ad8d395d4367843fdc84d66d5c67b |
| ken__ae2012-ultra2__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | da8f6a3a57ccdb5d3853388b2e9e62133545cc3d6ab389e301c205d0b6cff0db |
| ken__usf4-ultra1__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | a0cc4d92725ff969ca1f30fd14807fe07fd85adf7bfb01501d9eadd3927e1581 |
| ken__usf4-ultra1__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | e84813e9d056c5ca6271355f915293e8435bec497a7411558f6c359633b1c6ee |
| ken__usf4-ultra2__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | f4ef05077a74519bc824d236c538fb253242562566f9ef033313a5d335b2e166 |
| ken__usf4-ultra2__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | cc7bd99423cd8edf737d0050a525ee0ababb8094ddc1394c53d6f6e0f5afa0a5 |
| ken__usf4-ultradouble__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | daa9073a129f6f0a7c7e6d0b535a8b1a887face3ca382b26e9b913fc071ba0cf |
| ken__usf4-ultradouble__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 25cb507bb79de1d7e0447eec4e66cecb3a12412388c0ffeaa8cb17f9d67c199d |
| retsu__usf4-ultra1__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | add0bff7c9f1fa729056f27320bf96881218ac39e71c0bd06bf4cb4e626fbce6 |
| retsu__usf4-ultra1__old__new__4.tsv | 37 / 3 / 0 | 37 / 3 / 0 | b5e13815d201f9aa8802ddf4dafd8708efe704cf7e97b456c0b50e74f12be75c |
| retsu__usf4-ultra2__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | a620316620dca7dd390caedf8fceb0aacb9852c13df64fa841a88089de1801d9 |
| retsu__usf4-ultra2__old__new__4.tsv | 38 / 2 / 0 | 38 / 2 / 0 | 94347d5ba366a7039a776ff906255c5a02a4b5cfa1b650ad53baf878449b0310 |
| retsu__usf4-ultradouble__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | bf0ecfa1f01b5d0cb4b2adb4fe93b76f13a71b98116e9d9df419914a5b7ad2fe |
| retsu__usf4-ultradouble__old__new__4.tsv | 38 / 2 / 0 | 38 / 2 / 0 | 0160dc7a31cd74ca98b5e56df7fad98e621d082cd12dded093788958574a3ff3 |
| geki__usf4-ultra1__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | b029b0998e7b7b4a0a2e4f83c19522da5e47bbd2faf8e113d3a36a40c0e93896 |
| geki__usf4-ultra1__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 4e38faf24771903e7551c8d9d46eeec1875a5c99847ff944cd00f23e154f78d2 |
| geki__usf4-ultra2__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | eaa33cdb9193f09a9b349a10c0b0bb4ab0337f4c7bbc61a38d9d47144ad619ea |
| geki__usf4-ultra2__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 581262c27eb25c7267d70de83cc3adc4713aed65a13d9616bed73a35f96f2173 |
| geki__usf4-ultradouble__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 7c274459fcf1b0b4cb24c170839b42467059cf26be333cee177110f63b0cdfd9 |
| geki__usf4-ultradouble__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 6b37fdab4c0dcd54fa8186568679bcc537f8e0a084d5a18a91490b4f5c45f0fd |
| joe__usf4-ultra1__new__old__4.tsv | 39 / 1 / 0 | 39 / 1 / 0 | 43edabeaa5b90179f49b1ee5ed78c48b5305c60813f17cb519a710b62ba7ec30 |
| joe__usf4-ultra1__old__new__4.tsv | 39 / 1 / 0 | 39 / 1 / 0 | c7ecac8756880eb9e1571fac0798048c937ff78c6081d7a40f48b4915fb1c828 |
| joe__usf4-ultra2__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 3c3666733e70530352fc3f0f6c2f9ec888f6174a5a3be67ad3e3e2a11a07111c |
| joe__usf4-ultra2__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | a4f22a93f116b0a37204c24abee1eb6c50ae78e04dbbef0ccb22d34738b089e9 |
| joe__usf4-ultradouble__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | db55ec5f8402f0d4179ab2eb1b2a534fa1f82ab5a5acf2383094cd36a5e8f854 |
| joe__usf4-ultradouble__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 8ea5f9711aa3a01773a0e29376e7bf178d1af5b7556807f21a33b385daeda386 |
| mike__usf4-ultra1__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 2a1c843ae95e60c992bc489f484f3510014073f3c62f2f76c3bab8fc31689739 |
| mike__usf4-ultra1__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | f5f4359c6605412ca37c5be2fb55a2ac813b9d4c6c0d86e93fb86db1284a49d9 |
| mike__usf4-ultra2__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 49f8d975af211d4a49bf1708e10758964537ef80ee096ce5f36f18c2d2ca1cd6 |
| mike__usf4-ultra2__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | dc9c87752af1f57ac03a6db1630020b346ae34b6cddbda7a940c9ce2d987fd78 |
| mike__usf4-ultradouble__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 56fda10132c3ea85acea0e7707cde660b443969aecaf96488a9ab2eea8fbe198 |
| mike__usf4-ultradouble__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 1035320779692a58fd074a731ce1d98a100badfef60cc477c56f4ec769b5cee7 |
| lee__usf4-ultra1__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 0b401de4ec9e836524257cd87b496b5cc433444694781d1c528ef3e8818b26bf |
| lee__usf4-ultra1__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | ef85b52e323a39bc4a7930d29e22d9cbc6f4c7ad36002a115884ac9dd6010dd7 |
| lee__usf4-ultra2__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 266b0cbe8fd786c2eb83c49d762b7f1e67d787cdc8899fe3e380614758410d5f |
| lee__usf4-ultra2__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 1c742662e63deee21b6869a8316ad54bc86c993114cbe7f4254878ad021eb645 |
| lee__usf4-ultradouble__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | d346023a815cd2a2405897509c17f2fb6ac0caa5240b8614c1a6dfd3f05938f2 |
| lee__usf4-ultradouble__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | b3ef5299e4d4fdb0fe1b278363d70e14debce388266480a93e3bc6de459af2d4 |
| gen__sf4-ultra1__new__old__4.tsv | 38 / 2 / 0 | 38 / 2 / 0 | ee166e6a56d753a9e589cb487bcf9ea6528301ec3c2612e29d9d1157b5db19cb |
| gen__sf4-ultra1__old__new__4.tsv | 38 / 2 / 0 | 38 / 2 / 0 | 0e8a010fed49c17ff044d97f9cbfd6ad0ee9d08a610b5083bd276d8272f3599d |
| gen__ssf4-ultra1__new__old__4.tsv | 38 / 2 / 0 | 38 / 2 / 0 | 24f0fcb27bd46747cf88af40f293170b4cae564ee26dfab18bd0794c403d8c0a |
| gen__ssf4-ultra1__old__new__4.tsv | 38 / 2 / 0 | 38 / 2 / 0 | 095c6afe338bfb6a184acedc3b45f18b61f82e8472fa7811ffb4406f49521560 |
| gen__ssf4-ultra2__new__old__4.tsv | 39 / 1 / 0 | 39 / 1 / 0 | 7adf2c5b6c2ee45c5cd778441438a8ce2e047409c31cd06b4b1f016079443163 |
| gen__ssf4-ultra2__old__new__4.tsv | 39 / 1 / 0 | 39 / 1 / 0 | 5ca6ffaf383d5aff6cb7239e0eef3517e21363b77e18efd281c2122b485cff57 |
| gen__ae2012-ultra1__new__old__4.tsv | 38 / 2 / 0 | 38 / 2 / 0 | b67dae63bfb12091370804b0ec81c577e8395d4e2eb41203082dcb041de3b308 |
| gen__ae2012-ultra1__old__new__4.tsv | 38 / 2 / 0 | 38 / 2 / 0 | 4ee449bb83602fef55c76c04d275ddb0248175029eb447f29051f435438b4658 |
| gen__ae2012-ultra2__new__old__4.tsv | 39 / 1 / 0 | 39 / 1 / 0 | 4e07fbe835f7ea9b56284a00108bf7ae16231ce8920645fc69c40ed6f92fcfd7 |
| gen__ae2012-ultra2__old__new__4.tsv | 39 / 1 / 0 | 39 / 1 / 0 | 5279d6f726d46edf0d63cd987f8f1539a035249999f333bff0493146685f5465 |
| gen__usf4-ultra1__new__old__4.tsv | 38 / 2 / 0 | 38 / 2 / 0 | 6319823a00bd7b5ee454e2017175ab21de0927423c42ec326eab57847bf3a311 |
| gen__usf4-ultra1__old__new__4.tsv | 39 / 1 / 0 | 39 / 1 / 0 | 787850f8d7be44e335a715a6a1d5ddcfb796cf0f6d42c0e7e1f754dbfd05c9ea |
| gen__usf4-ultra2__new__old__4.tsv | 39 / 1 / 0 | 39 / 1 / 0 | d4a8e1d3983f33ba55cdc52ee36ce0015f57a304f247cfbcdffa7fcbcd898815 |
| gen__usf4-ultra2__old__new__4.tsv | 39 / 1 / 0 | 39 / 1 / 0 | e7e9ae8e18eb167cab1c492ce6d82cf6875d4f8778ee06cdd7d2b016835b2bc0 |
| gen__usf4-ultradouble__new__old__4.tsv | 39 / 1 / 0 | 39 / 1 / 0 | 3023ffb718787a95ecfcd4fe48afabdbadc6dc4119b491b730bb9a27e366e82c |
| gen__usf4-ultradouble__old__new__4.tsv | 39 / 1 / 0 | 39 / 1 / 0 | 8800a2398ac29cf496a9320e22c26b3c24b3fd0698d86b92d7c0b684ab2c92b1 |
| birdie__usf4-ultra1__new__old__4.tsv | 38 / 2 / 0 | 38 / 2 / 0 | 075ad18434ef1c557b942ad162ca53db9dbe05bb21d64c5581d51e3a0d0c4d5d |
| birdie__usf4-ultra1__old__new__4.tsv | 36 / 4 / 0 | 36 / 4 / 0 | 01601a668b2a152b3487a7f726996dedcd7dbfd239345153cec491319cbadd93 |
| birdie__usf4-ultra2__new__old__4.tsv | 39 / 1 / 0 | 39 / 1 / 0 | 98dbfcfeda30efde0f2c09c0d00e86ea816502b5a2db8ddc20ee1a4c811e2660 |
| birdie__usf4-ultra2__old__new__4.tsv | 36 / 4 / 0 | 36 / 4 / 0 | 2a8a772fc45d78c2961c2737603290276e3dd5f010a092076ad17faf37e9d426 |
| birdie__usf4-ultradouble__new__old__4.tsv | 36 / 4 / 0 | 36 / 4 / 0 | 0eb0899ef65ac3e5217429212dd32c779fe9467d8a6d1127fe709c42c071f7dd |
| birdie__usf4-ultradouble__old__new__4.tsv | 37 / 3 / 0 | 37 / 3 / 0 | 2a78539648841f3fb9247fcc841fc2ce12811ce30de7c37d61e438e2dd2bf78c |
| eagle__usf4-ultra1__new__old__4.tsv | 35 / 5 / 0 | 35 / 5 / 0 | 2f9634bf39d3b15044ae6fcf25bfe95e238ab1eac4da4fc98162dad8bd25e693 |
| eagle__usf4-ultra1__old__new__4.tsv | 35 / 5 / 0 | 35 / 5 / 0 | 96fb0861f44311ff9100d2c8af939d42f66437ed4ac0851c43f8d0375f0fd61c |
| eagle__usf4-ultra2__new__old__4.tsv | 35 / 5 / 0 | 35 / 5 / 0 | be6c120b168cbf28e4b153520f9bda39a45c82cf5247edd55259d15f051e8e0d |
| eagle__usf4-ultra2__old__new__4.tsv | 35 / 5 / 0 | 35 / 5 / 0 | 05849545c976a5c6c7ea7557c383499bc200334109c5632445a3ba72115c71f2 |
| eagle__usf4-ultradouble__new__old__4.tsv | 32 / 8 / 0 | 32 / 8 / 0 | ece890e49e3f861194b5d72373d40e7c55a673f4e15a5d7db3596e58201295d3 |
| eagle__usf4-ultradouble__old__new__4.tsv | 35 / 5 / 0 | 35 / 5 / 0 | 1a9311e4f6a1706a67c9f570cf360e76d5c210aecfe758ccedcb426c933fe0c2 |
| adon__ssf4-ultra1__new__old__4.tsv | 38 / 2 / 0 | 38 / 2 / 0 | f1e53401ef9faacc6b5d3e8b14bf7c7d6fd0afe2968bad157caafd49c19ee3c4 |
| adon__ssf4-ultra1__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | e247e47eeefe77f1a79be45fe56dc6f051dc6eb798ec9eacdc3c7e11cd2b7084 |
| adon__ssf4-ultra2__new__old__4.tsv | 39 / 1 / 0 | 39 / 1 / 0 | 3fa2044e8284ab97c3f739c139be43531b357b09c1096007921ae37943a49343 |
| adon__ssf4-ultra2__old__new__4.tsv | 39 / 1 / 0 | 39 / 1 / 0 | 379dbfff71e8ae0fafcc36115951e80f75e00ec130ddafc38fc3ab8d74fcc18b |
| adon__ae2012-ultra1__new__old__4.tsv | 38 / 2 / 0 | 38 / 2 / 0 | 0178b7e09b9f200826f341cd670701155bf1ae29308d661def17efadd49313c7 |
| adon__ae2012-ultra1__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | f61d983972f8cc9705b354d4fde54ae1e72363c9f6df6c774fdfe70b4a308eb2 |
| adon__ae2012-ultra2__new__old__4.tsv | 39 / 1 / 0 | 39 / 1 / 0 | ee9aaf4b50f85369264dcff6325c2c465ecabea920a8d9bcb051928cf2646d89 |
| adon__ae2012-ultra2__old__new__4.tsv | 39 / 1 / 0 | 39 / 1 / 0 | 2c2dd9b475ad322e670503609115e7121f291c158a00bc88f15f046c12ea1ece |
| adon__usf4-ultra1__new__old__4.tsv | 38 / 2 / 0 | 38 / 2 / 0 | e02c8fd65058d3dd920f6f075d2606010a1737131b45b71868e9b5d40a666bc1 |
| adon__usf4-ultra1__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 4ac457f3dc3946471b5904e10ee03d8a9be5371a8e562af31e14c31494cc088f |
| adon__usf4-ultra2__new__old__4.tsv | 38 / 2 / 0 | 38 / 2 / 0 | d8aedf683950d09e8f79429d6a39707569390180dad8379b506ae462910078a6 |
| adon__usf4-ultra2__old__new__4.tsv | 39 / 1 / 0 | 39 / 1 / 0 | 65f056527a0fa0b865a2876efefffb0ae89c4e5319b1d1b5bb317a67928cde08 |
| adon__usf4-ultradouble__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | ed181f883b36f2eea1dd173f96c54e07c0f2874160f3f154798f19cf9d536b60 |
| adon__usf4-ultradouble__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | e82fa5164b8cfeace4608d2cb854e0aee6106cb4b9b998d3445f143f4f0f5715 |
| sagat__sf4-ultra1__new__old__4.tsv | 39 / 1 / 0 | 39 / 1 / 0 | 26e5b9893c375dda0d623f68faa6a12c5c956a91e34f3103f30987c83b742b73 |
| sagat__sf4-ultra1__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 476174ede566b6c3271b4698d1812ac42d59f9be87999d371fe19bb4484d9242 |
| sagat__ssf4-ultra1__new__old__4.tsv | 39 / 1 / 0 | 39 / 1 / 0 | 790f80b0bcf36f6831c451ecb0e53719df62f1026b15cfaddc9fd504d1af8549 |
| sagat__ssf4-ultra1__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 224515032bf99a987af210bccf6be4f60781369e0493bd7d6083b09d3f26ba29 |
| sagat__ssf4-ultra2__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | f45e86bcab611752b9cde3a7b4153ccc5454792b7903dedf33a018b92e762279 |
| sagat__ssf4-ultra2__old__new__4.tsv | 39 / 1 / 0 | 39 / 1 / 0 | bda53dce90a079686a85fea840b3276b3f9d87f4640003292329aece2a14caad |
| sagat__ae2012-ultra1__new__old__4.tsv | 39 / 1 / 0 | 39 / 1 / 0 | 3c8cec5712a11f83ff057bf8f5fc615d9b24ddfc9301e4ad67ba54ca6a341ec1 |
| sagat__ae2012-ultra1__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | b144012aeea4e6cb90c5fcc6b0c9435944aed22ef7a5f456e91a3a02d02f8b7c |
| sagat__ae2012-ultra2__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 721c2c7760178104e9d080b7e32f5703bf3f536317ef29140ace3da917ba692e |
| sagat__ae2012-ultra2__old__new__4.tsv | 39 / 1 / 0 | 39 / 1 / 0 | 4167ade9ea0852ca5d79ce5547c94919c3c2ca38709280cdd24ad71f932e2125 |
| sagat__usf4-ultra1__new__old__4.tsv | 39 / 1 / 0 | 39 / 1 / 0 | fd65378d1285f8794bfa9b501eaffcd4c7583cb057765d0bb10938bf125d2355 |
| sagat__usf4-ultra1__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 38456a5913488f1645278fe0117ad049275a77dc2bf33423e74dbc9c7cc1b727 |
| sagat__usf4-ultra2__new__old__4.tsv | 39 / 1 / 0 | 39 / 1 / 0 | 94c3ee1bc551d0e920fdc7fc8df21eeddd34c3fa50f062d7be7b96e3f2d35fcb |
| sagat__usf4-ultra2__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 4ab3251eb7a2d854637ed643ab8d1bfa10a2b72a9c6c6902fafbbb0233d82973 |
| sagat__usf4-ultradouble__new__old__4.tsv | 37 / 3 / 0 | 37 / 3 / 0 | dcf1207539dfda7bce817b415e3a4d66993f54584dd623c7bb2017f67538a6f2 |
| sagat__usf4-ultradouble__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 6c2d4828900a57dd1892221f66f0ce3408fd1ce0865baacf197090d50e0b63e5 |
| ryu__sf3ng-art1__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | d5f3932cb7c7c9a30da5955ce82773502fc81a425995add65853c4774e07ebdc |
| ryu__sf3ng-art1__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 8ce5cd5cc78dad239b5440363b17e780a95e79a1f8568c3eff0c0bb9c4aefef7 |
| ryu__sf3ng-art2__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 518680aacc639ca61841ccd8d1712d728faa8d91dfdc6f601a1825cc0511dd51 |
| ryu__sf3ng-art2__old__new__4.tsv | 39 / 1 / 0 | 39 / 1 / 0 | f95c4b43433479c1330e34848aa378e4d561713d432532dc8e335b48aa284de0 |
| ryu__sf3ng-art3__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 0cf493df3de46847903a0ebf9da6c61446de8f667ae286ec615a9360bda09f58 |
| ryu__sf3ng-art3__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 8669876ed72eb04c877f58ead9052a8384fac71732f1c4d16f4c9a0f3f6fec46 |
| ryu__sf3twoi-art1__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | b3de03071cb28cf878743461bda0209f6193c4aa6664dd759aa68a8e8ddbeb90 |
| ryu__sf3twoi-art1__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 49bc7e6867b69a08ee418de8a4677c4c2b54d45e497b9a68ce3689afa47ed935 |
| ryu__sf3twoi-art2__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 7fce0912b939155d7105e5fb8339031173512cb54b4c21d75e1c7d0a9e80f267 |
| ryu__sf3twoi-art2__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 8fddf1e9def4387175a3aed5717301e31cfaaf44fb91cad0f21acca4df48b070 |
| ryu__sf3twoi-art3__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | b85ab2bb51858e53c906c3b6f10f16a05f7dd36b7d0615c85ee6caba472757a7 |
| ryu__sf3twoi-art3__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | b6720043ab38035f5180a66963dd931e4f4feb394a5224ed6d92fd49c5846134 |
| ryu__sf3threes-art1__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 83bdafa9ac7c57a3669c99907251d4aa7017fc9e2e70c099b9454a0a6ec50763 |
| ryu__sf3threes-art1__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | be20c6b5d0c80eab61309b19f4d2519a1a6c47b234999f6d44c1e6f0221e4d33 |
| ryu__sf3threes-art2__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | d9e7665dffe8e391f9ed1c4d153f573a9294cd6f6a083aff3e7a7885499a5517 |
| ryu__sf3threes-art2__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 28c4195da45d81f6a32ed9d36bec1fc2fc617917ade760a5702610d055d82fad |
| ryu__sf3threes-art3__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | dfa58cdf639efa0be6cecac5ad44f6a4b2e3ea303db241a15f29418b56586591 |
| ryu__sf3threes-art3__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 43d4a158206d5e1774ae62479bf3f155a7224328d707238f3737522f0c3d23fc |
| ken__sf3ng-art1__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 4ad4700378e5c809bf6c8bac4adb328510e4ce9000182660ec24a1c76374bbbc |
| ken__sf3ng-art1__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 66480561f5b1272abcdb8243f1128377e3661ae6acc2df5893a5be1a8d99daa0 |
| ken__sf3ng-art2__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | dcc6e65dec024b5f45c02c226d051082a73de8d5a91d3caca05409a38486b26e |
| ken__sf3ng-art2__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | eac40e65fb345a9a3f16879a641571dbe9798cfcec976080af1dbda7866c5bf2 |
| ken__sf3ng-art3__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | c1088f8971e837f706d168ecdd9f3d87c84aafbd4b25d4aad9e8627dc151af78 |
| ken__sf3ng-art3__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 97fd45c88e06b7264054371d64c4c51cac7a7a855e8061a05d9c605d8e95b8a1 |
| ken__sf3twoi-art1__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 0c0696e078aa535f397f9cf57ae1208341aec050a761682aaeaf002b81269036 |
| ken__sf3twoi-art1__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 4acb54bbc0556879a05a45a130644e75638bce52793cf7179777fef5dc338c71 |
| ken__sf3twoi-art2__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 5003481c2c98bc3ac390be26944c85590d9296adc711deef1664e21b0d321264 |
| ken__sf3twoi-art2__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 7411156432b9e23453ae391988a6cfacb25d67b5aadadcf3048dfd8c26326e0f |
| ken__sf3twoi-art3__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | ca95507096d6cf9440d70c11126e33829bcc37516258dbe8bec2397f90b8c701 |
| ken__sf3twoi-art3__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | a1942cdeeab0459f2d3bcebd6f6818058b5a95c084246e6c7e41a4d7363d90e8 |
| ken__sf3threes-art1__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 7b7ae4b49cd20af460c98e11f52216c661ff4f26ca98b3ccb076eb501fe7a5ae |
| ken__sf3threes-art1__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 1510730cab986105a32ea9cd419580073d76dce73245e6a65205648e946ed0af |
| ken__sf3threes-art2__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 008b98d436c24f0370734eaf13bde9f97232ab9fd4f923ab7e46d75400143840 |
| ken__sf3threes-art2__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 746b40526bbd8c622c63df74f319797dda6aa2e7c66e1ef904d3a71b2b8fb551 |
| ken__sf3threes-art3__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 8bd46df601a19a690910c7389a7b6b51b2653d4d99f8f6a1e99f5d54f0f800cd |
| ken__sf3threes-art3__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | eb9a4d593df817d7c19051d3117fa616dd206ba955e282fdc3762e5d215b9cf8 |
| retsu__sf3threes-art1__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | d2e412d1734b87ef44bdd4fa5b3a71428ec579c89a209d3b1a0b4498f86c58fc |
| retsu__sf3threes-art1__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 97adc3e56a33caaa43fab4bf0169ee4ec7354f8afea94c622107f812beb9c4d5 |
| retsu__sf3threes-art2__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 1fced46fd9f7fd3337375989a63ceb223e9f05eb7fec19e09f7bdc099eb87dcd |
| retsu__sf3threes-art2__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 70ad50178c5c1be7e5e08aba4087438fee997cf6d4cbd736d083b0ec2f0a4e29 |
| retsu__sf3threes-art3__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | d746d47702ae777ecbf99b4994d56e87986c9806528a6258cfd40bc154e468a0 |
| retsu__sf3threes-art3__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 17a61077531ecaa202821b4209c009f6cf1f7f0bf122213d1a4299f9e081e7c7 |
| geki__sf3threes-art1__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 8076f41bf62618e60af369989c38ebe2878d3362350e85a4267317d94a519651 |
| geki__sf3threes-art1__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | bc83d2febbbcf2264616408deecdc96e72caec557872fae0f469a13180a0b825 |
| geki__sf3threes-art2__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 59c4ed48802655add9d8adc82b5dc36a5d2c6f1a0d0b5adcabff67c99cb51e62 |
| geki__sf3threes-art2__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 0aa9c406f7ba674c39a644be58f48d245e0198db47e3064797df519cd2ea3472 |
| geki__sf3threes-art3__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | dfaa1fe4823c42ffbbffef60f087c13658237b1c7258291fd72ebad763be33d9 |
| geki__sf3threes-art3__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 2fa6d1cadd59931ba9b9b8b52de38ef922785785973e71ab033a0e5e6988d260 |
| joe__sf3threes-art1__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 07734b8a96cc45f0c8ec92834d7b5578c95dca315b78f02914c1e18a0c23b40a |
| joe__sf3threes-art1__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 57b5de6ae315bb044d0f465762af62c4120da7159c07d71c5ef76621fac52b45 |
| joe__sf3threes-art2__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 45754a32be1f55bd4927e93383e2bb4d27c7d462b6dd1a0d4131fd2fa8f29a2b |
| joe__sf3threes-art2__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | a4a9c520208bc47f33514313c641bf0fbceffec2dcf650fed7a456cbc0498423 |
| joe__sf3threes-art3__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 79cfe8248ed7f93c380c854e69ccbf0390d3c7f31d95a7dedb49186a0bdb8c34 |
| joe__sf3threes-art3__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 2362e4a60fe826c103c165119311b5167bd4be4a3e4f82427e5cc90f4c816a82 |
| mike__sf3threes-art1__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 0c6fa6c1e4aa327b6360e7298b039f991366ccbf123583c7a4ce7241c8d8165a |
| mike__sf3threes-art1__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 7cf1fada0405cbf7edec8930c4a5f87f28fa80f79e98cdb7093e097cfdccc25c |
| mike__sf3threes-art2__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | ce0f51c7fa359a662fadfe2f043f00bfd27f3f913f045841e0affb0fb519ff0d |
| mike__sf3threes-art2__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 3e7a841cf2eaf12a561fb94be115e9ef92d7de13ed390a7128cdec0213bac905 |
| mike__sf3threes-art3__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | b609b7035480a669879025c1215bf1c82331f4f13c5b63cb5df7b89e3dd620e5 |
| mike__sf3threes-art3__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | b0fd96a269d585c7c30cb214d6cc7d87c8e04ebc4074bfeac05b9c91a4536a57 |
| lee__sf3threes-art1__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 08e90d3828792fe94fdb6de260424ebd96609c8fb5fbcdb2f7e00187dc77232f |
| lee__sf3threes-art1__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 38abc68fd11366855f3b13d9c35abb8ce85a9d79e2d187f12cd329222a96c64a |
| lee__sf3threes-art2__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 1f936cd939c77dd83f6d80ea30c3d9e38fb9294bcf23b0b0d87339265ea4080c |
| lee__sf3threes-art2__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | d6f57684713801339cc97b6fbc84337e593973f322d7f2a0786ae5eb3e5d3227 |
| lee__sf3threes-art3__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 00e6dd52be6c92bea85bdf7e327ad87f7606bccb306b267e310cd0965bbb9680 |
| lee__sf3threes-art3__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 46db6cb1ee5d87a2e201486542e85b98e6ee2bc9a76d3953e43d9a7ed3809d38 |
| gen__sf3threes-art1__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | fee6ddeb30f9ac6cedfa84a6c8f095ccc7c669a4206c18a2db7bb52f351c0a36 |
| gen__sf3threes-art1__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 40f788d6cb4eda418b0264802d8edcc0e84f73e110769cc96cf7ccb3e71e9928 |
| gen__sf3threes-art2__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 54974efd86f23e9a62f8c645a566ee48fda6de2f5430284f106e7cc7b1f88e9e |
| gen__sf3threes-art2__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | f549c3d241263dd27f0bb9f9573c6a850e3e03378ab225d550f8d1ed512be2a9 |
| gen__sf3threes-art3__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | c80faf97b87dd6e21889010cc46e13122bdb2063e59458c2d4413b009d64d694 |
| gen__sf3threes-art3__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | bb51cc1f9ba52b995e14bcd486be0106c8982c61ea2151dd4d55c7eb4bca9495 |
| birdie__sf3threes-art1__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 0d8b257ecf5210bb2e58fa6469b78d563916d13ade9e404edd7b7c9b81835798 |
| birdie__sf3threes-art1__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | b778815574715789a1ca709029b581d055ae0cf2397bb49c3fe9ffa35b23bdc8 |
| birdie__sf3threes-art2__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 27fe57dacb0a2ffb9b6871386bf2fbaffcd04270342a4d433faf420117839d38 |
| birdie__sf3threes-art2__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 71a36de3c0d647270f4816ec2e39a00f5dc208917d5ff20e0f2c5f123eb3e601 |
| birdie__sf3threes-art3__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 4b2c11442bd32d10af408e6865e7ed2414a9fc2738e1362268da66a5399336e0 |
| birdie__sf3threes-art3__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 130ac89ae3d5c9a86e93c9265d350092ca1d337dd04e0713a90570bcbe3b4114 |
| eagle__sf3threes-art1__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 1f816787e2d2a59989f703a016d607f5b9aff2095da79722b4fbfe5e75f795ab |
| eagle__sf3threes-art1__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 3aaa797e6fbf1863b7246046537bf345923a987bc32f24dd03ad43e96f3524b1 |
| eagle__sf3threes-art2__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | dbee23ffc08f7dc0d0e250397ca193811dbcabe0ba178fc260ef9bc0dee31549 |
| eagle__sf3threes-art2__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | b5f3c59bbc7e831c6ab33f18d532345f3aeaef72fb25d902069d743b58d40501 |
| eagle__sf3threes-art3__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 817f35e5db97895ca62b27a26806bd5dc3f187c37377520f9390336653d10a92 |
| eagle__sf3threes-art3__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 9eee176ae92ae5d6c4b18abf8b5c6ffedf5e2166b7d7ce79500b107ebfea9163 |
| adon__sf3threes-art1__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 7dcb5ee84452bb882f3ceca5dc7e79b570096d39fa071bcc566f60718a04d639 |
| adon__sf3threes-art1__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 3b8b4e70e2f8f2fa40b23c0ed654f93c8d404d355de2ed06e371c465fd469f8c |
| adon__sf3threes-art2__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 759ebd89dd62e1b76a1138e419318f187933ca2f09ac36d8cb3618556ca80480 |
| adon__sf3threes-art2__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | cd8ed71d57534ed8c242ac8d1d94f2efb5cf0e150bb9bc376bf23c917d15d67f |
| adon__sf3threes-art3__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | f759de39a4f360c1296e5f4a83b31c582012979cbf34027cb818ddd877c5ad9b |
| adon__sf3threes-art3__old__new__4.tsv | 39 / 1 / 0 | 39 / 1 / 0 | 83802d1704f3af91499a14977de3264dfe843b521f9b0870ece877f87fa4f53a |
| sagat__sf3threes-art1__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | c29715e036995f975f35f8145ad22d43a048dd709df3480ded6af1b5f2682776 |
| sagat__sf3threes-art1__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 354ae1ba595b033fffb331843989af48fecac6c40996f4252133b024d9fb4953 |
| sagat__sf3threes-art2__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 1e8a49aa235863b96d3ef93576c047cd3740b04007c35f02e1aaa6397d16d300 |
| sagat__sf3threes-art2__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 45a77de7c84c0e636da4aa395de852dbf33b473d7771581dc5e573408af5ba88 |
| sagat__sf3threes-art3__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 1643242bdb38c833bd42f1a2f57c3637b82d5388ab35080783d9d61edbb2fc0e |
| sagat__sf3threes-art3__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 776d6e94fdcae6aeec7a0bd5a8796f165c21c8dbb77e8d281578d64a2cb52eb5 |
| ryu__st__new__old__4.tsv | 38 / 2 / 0 | 38 / 2 / 0 | a625a94079645f57dad42459a94a57f98262a3b941cdf1f372d0f1c7d4b9e20c |
| ryu__st__old__old__4.tsv | 34 / 6 / 0 | 34 / 6 / 0 | 3a956e391e4363ed225a15eae4e648325d9501062a7546870f4afe2af5c22551 |
| ryu__sfz3__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 1192e0aba98cf9146270df68e9ad776ff4439db61967c13fc8b7ab0cd8b96ca0 |
| ryu__sfz3__old__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 07e231a1963fd176709086cebd0fb3d96f5041822cdfe01711cddeee4841a720 |
| retsu__st__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 988a1bb85bb09157dbcbd8f7a756a7bf30be651eed764da550cb36f8d23770cf |
| retsu__st__old__old__4.tsv | 39 / 1 / 0 | 39 / 1 / 0 | 46b487d35f262098380d72ed63b5fa61fc20151a817704bf0e2393bfdd57c964 |
| retsu__sfz3__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 3c6248a3f53ac0a66ce8ebbfc0368e1099b209190636407459ca0bf53ac7d6ff |
| retsu__sfz3__old__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 51d76dcd7a557d7d3e1b877c375b2e7c65fc8da8103832b85906928e28a55822 |
| geki__st__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 2896fc2176fef3fed1f955cf737e288521aae4c991390afdb4d7d7bdb778cc67 |
| geki__st__old__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | fe5628f3c5af5b7188ac8876dc8e5603a0a22e745e442d13ddbb2e840f476312 |
| geki__sfz3__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 50f61c1f874b7f5da2f7dd2a6c40fd820bf5cb76500525c328c3bf5c5eb925be |
| geki__sfz3__old__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | b80a3de3c3d1f2c762822359f3c5f3078076e96f5a268f1e56e46c5a2d633d49 |
| lee__st__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | b8ea6101fee17e9c3e95aba2158ab39aefbcd6997bac6cbec27b86bdb50e50c3 |
| lee__st__old__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 8c094847c9953cb779d0d7b929d3d956b2a13b883d4a832c58ebd3925c82cd75 |
| lee__sfz3__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 9743366a0491211c9be028a273f4c4619d30611df1952d5eac3bb9f5e6e95de3 |
| lee__sfz3__old__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 55ec1dcfcc6e33cb6352215755db39ec57656c24b90048899d49ed0cf269117e |
| gen__st__new__old__4.tsv | 39 / 1 / 0 | 39 / 1 / 0 | de265b76b8ff9f7ba832879d10aec3301084f8d30a177ed50429b96383840e6a |
| gen__st__old__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | eec3d750f19be95afbcc427e13c7aa0aadbdf0aac00ab57dd2de1a7de5145c45 |
| gen__sfz3__new__old__4.tsv | 39 / 1 / 0 | 39 / 1 / 0 | 57de16e5f38986fad839584a6b075c221e8f731a94467089b1add2276351bc3f |
| gen__sfz3__old__old__4.tsv | 37 / 3 / 0 | 37 / 3 / 0 | ddcf2413ed8c13308c8741e6b5d882d4941afddea518ed966c6c73a19a703495 |
| birdie__st__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 24a6697aec8034aa41ba6f03dbb6c6ecdbe7388a45a7005df003c2fbe10af4b8 |
| birdie__st__old__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 1316e6a47f5bc2e15b1f7b635f24b1152a6fe70fb75a16a43712680573a8e0f2 |
| birdie__sfz3__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | feb8ec44344f7f42785783e12baa1ac6d41bef6e215e6dfbcc90f36b0519aacc |
| birdie__sfz3__old__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | c6933e30e78298b96e07136655e96922982776ec0ccdfdb838b8ce13e1ed7e33 |
| eagle__st__new__old__4.tsv | 38 / 2 / 0 | 38 / 2 / 0 | 39a3a7b7fdc0ba091d25e923fb192aa79abbd5950743e075d426068964b04814 |
| eagle__st__old__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 4215b82020012f4866245b91194f208be4b74d80ef7cf007d424140422ed29d5 |
| eagle__sfz3__new__old__4.tsv | 39 / 1 / 0 | 39 / 1 / 0 | 3413d7166021d08c2f381cd6a7eded72c9b8b191af7b6e0f3af87f4302989e99 |
| eagle__sfz3__old__old__4.tsv | 36 / 4 / 0 | 36 / 4 / 0 | 079c4b58cbb1fcd257a3aeeea4c38035d487260040504706ae6df37e9ef3aec9 |

## Provenance and preserved artifacts

| Artifact | Source | Version | Executable SHA-256 | Manifest SHA-256 | Raw fingerprint | Frozen provenance SHA-256 | Owned build SHA-256 | Build log SHA-256 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Final | 1018d722f308930758e12f092df29e0675dfd822 | 1.5.32 | 44c0d3a5bca73ed126dba846f90518e6af18efa0387037bb3033f55e7b4bb5da | ef8fca3d67bc3a8b0b98f623310ab8380a6f5395801dcd48d35a17737913f6dc | 42f37121c0472401be5a5f530d3ae2437feeb6120d17eb8845e62bd3bc8e355c | 46b5b6426b8d4a11a8f149987d3496c2a5ab1064cb8f3d0ea3633556af130f43 | 84f706c68be024e7f6d2bc2c38d4dd426e49705414a663c5e2e77fa224702006 | f2a747ac471e03f4c74a6187a23ba940e9c2e414861db4123d5e88d5eedf4071 |
| Baseline controls | 095562d769850796f5fd74cf80c515f311559d3a | 1.5.31 | 1b1c8568fb975f528c94ffc50ab9fb6e4322ca33dcd014826e18194deda80745 | aae7b149a19a56cc1a0b79e24d2384fbed466c6d34e1c2d71144cccfc810395c | a2d30b441519b5166729551108730a664fda789bdd229792d16a321368db84b3 | a7ce3346d11e6e1417da04830fdc2abe8b9ed2048604cda04927e8c13fd5bc37 | d97bac7c1c4fd2a87218f85cecc2b3e42c45944ed1543b0fc34497426d6da754 | fa60beb83a66816b8818db24ceffd02bf528d72ebee8003ee1ec2d10d47d8d0b |
| Current controls | 1018d722f308930758e12f092df29e0675dfd822 | 1.5.32 | 44c0d3a5bca73ed126dba846f90518e6af18efa0387037bb3033f55e7b4bb5da | 47e9d9453aff5effa72ded22cf61251e7aef6bd92eb443831a65580747c51235 | a2d30b441519b5166729551108730a664fda789bdd229792d16a321368db84b3 | 46b5b6426b8d4a11a8f149987d3496c2a5ab1064cb8f3d0ea3633556af130f43 | 84f706c68be024e7f6d2bc2c38d4dd426e49705414a663c5e2e77fa224702006 | f2a747ac471e03f4c74a6187a23ba940e9c2e414861db4123d5e88d5eedf4071 |

Each raw fingerprint hashes sorted UTF-8 filenames, NUL, complete raw bytes and NUL. Frozen executables, provenance, owned build records, build logs, per-cell execution records and stdout logs were present and rehashed. The owned command and zero exit status were verified; the before/after committed source/tree/version and resulting executable hashes agree. This generator does not alter those artifacts or the measurement manifests.

- Final: raw folder `C:\Users\George\Projects\sf1-port\target\sc20\final-1018d72`; executable `D:\sf1-port-storage\build\original-target\sc20\bin\cpu_measure-sc20-1018d722f308.exe`; provenance `C:\Users\George\Projects\sf1-port\target\sc20\bin\measurement-1018d722f308-current.json`; owned build `D:\sf1-port-storage\build\original-target\sc20\candidate-build.build.json`; build log `D:\sf1-port-storage\build\original-target\sc20\candidate-build.log`.
- Baseline controls: raw folder `C:\Users\George\Projects\sf1-port\target\sc20\baseline-controls`; executable `D:\sf1-port-storage\build\original-target\sc20\bin\cpu_measure-sc20-095562d76985.exe`; provenance `C:\Users\George\Projects\sf1-port\target\sc20\bin\measurement-095562d76985-baseline.json`; owned build `D:\sf1-port-storage\build\original-target\sc20\baseline-build.build.json`; build log `D:\sf1-port-storage\build\original-target\sc20\baseline-build.log`.
- Current controls: raw folder `C:\Users\George\Projects\sf1-port\target\sc20\current-controls-1018d72`; executable `D:\sf1-port-storage\build\original-target\sc20\bin\cpu_measure-sc20-1018d722f308.exe`; provenance `C:\Users\George\Projects\sf1-port\target\sc20\bin\measurement-1018d722f308-current.json`; owned build `D:\sf1-port-storage\build\original-target\sc20\candidate-build.build.json`; build log `D:\sf1-port-storage\build\original-target\sc20\candidate-build.log`.

## Validation and review

Full suites are permitted under the current maintainer instruction. The explicit checks below record which focused and full-suite commands actually ran, their exits and any skips. Permission alone establishes no pass.

Workspace metadata and all 11 local lockfile packages were checked against reported version `1.5.32`. The validation and review outcomes below are explicit supplied claims with their preserved evidence, not deductions from measurement wins.

- **passed**, focused-native-and-library, source `1018d722f308930758e12f092df29e0675dfd822`, observed command exit `0`: 428 passed, 0 failed, 0 ignored and 0 asset self-skips across 8 targets; 493.2 seconds including the optimized rebuild.

```text
rtk proxy cargo test --profile suite -p sf-engine --lib --test stronger_cpu_sf6 --test stronger_cpu --test sf6_native --test sf6_era --test stronger_cpu_sf5 --test dramatic_world_tour_stronger_cpu --test adon_victory_settle --locked -j 1 --no-fail-fast -- --nocapture --test-threads=2
```

Evidence: `C:\Users\George\Projects\sf1-port\target\sc20\focused-delivery.log`, SHA-256 `7661deddf9ff44911f3c972116dcb13bb00b05898be1fcf157764f276aa3eaba`.

Source qualification: All eight targets executed after committing 1018d72; no subsequent production or test source changes.

- **passed**, game-runtime-version, source `1018d722f308930758e12f092df29e0675dfd822`, observed command exit `0`: Street Fighter': The World Tournament v1.5.32 (1018d722); all 11 local package versions match 1.5.32.

```text
rtk proxy target/suite/sf-game.exe --version
```

Evidence: `C:\Users\George\Projects\sf1-port\target\sc20\game-version.log`, SHA-256 `8430e4d65887ea5d0fb803ef52bbaf814c49e6d60707c86e426cf0aa312ce089`.

- **passed**, full-suite-parallel, source `1018d722f308930758e12f092df29e0675dfd822`, observed command exit `0`: 2,389 passed, 0 failed and 3 declared ignored tests; 22 slow notices; 2,440.103 seconds in nextest and 2,912.2 seconds including the rebuild.

```text
rtk proxy cargo suite-parallel
```

Evidence: `C:\Users\George\Projects\sf1-port\target\sc20\suite-parallel.log`, SHA-256 `86c2253cc66588a9488e173555d79c2e701fbb862988834ecd0ea977402e8522`.

- **passed**, full-suite-serial, source `1018d722f308930758e12f092df29e0675dfd822`, observed command exit `0`: 166 passed, 0 failed and 1 declared ignored test; 345.447 seconds in nextest and 349.2 seconds total.

```text
rtk proxy cargo suite-serial
```

Evidence: `C:\Users\George\Projects\sf1-port\target\sc20\suite-serial.log`, SHA-256 `030e8b0c5db8d2a0672bc1fd8354268ff1132290a9af236a37b3271330906d0c`.

- **Standards review**, `095562d769850796f5fd74cf80c515f311559d3a..1018d722f308930758e12f092df29e0675dfd822`; remaining actionable findings: 0. Independent Standards review and final test-only delta recheck found zero actionable findings. Evidence: Independent agent sc20_final_standards reviewed the complete fixed-base delta and final1018d72 fixture correction; static review, no tests run by reviewer.

Preserved review file: `C:\Users\George\Projects\sf1-port\target\sc20\final-standards-review.md`, SHA-256 `6166c2fc36634593d4f260e14f51dc7eadf1fd22c0477860dbc156248a1d6d7d`.

- **Spec review**, `095562d769850796f5fd74cf80c515f311559d3a..1018d722f308930758e12f092df29e0675dfd822`; remaining actionable findings: 0. Independent Spec source review found zero confirmed source defects; readiness depends on the measured gates and maintainer playtest. Evidence: Independent agent sc20_final_spec read ticket212, ADR0026 and approved design, checked committed source and final test-only correction; no default-enable approval.

Preserved review file: `C:\Users\George\Projects\sf1-port\target\sc20\final-spec-review.md`, SHA-256 `252e5aad17ddd5605276f2601981520ab6d07399f4cdcaa5bbc5d4ce865786dd`.

- **Report review**, `095562d769850796f5fd74cf80c515f311559d3a..1018d722f308930758e12f092df29e0675dfd822`; remaining actionable findings: 0. Independent raw-data and expanded report audit found zero actionable findings. All 180 standard cells, 388 paired controls, 44 Dramatic cells, 1080 added means and 720 round/opening totals were independently checked. Evidence: Agent sc20_delivery_audit reviewed standard preview33588e0b0a6253245d9477ce1c2ef995f6298408d54e0b8fbbb5c30f4dfdba4d and Dramatic previewd9e323349b5282b363184c0dc89e3d9c3d41b36b90d823b425fee2f2f63084c8; final publication adds this observed review record with unchanged measured data.

Preserved review file: `C:\Users\George\Projects\sf1-port\target\sc20\delivery-independent-audit.md`, SHA-256 `f7cb6e8de953249295c5d78d663862f3832d63e2caf17cf98a8670df50edf330`.

Human playtest status supplied for this report: No human SF6 cast playtest was performed in this SC20 work. Earlier Ryu SC12 approval does not approve the new cast or whole-cast enablement.

The complete standard grid has96/96 primary win/activity cells and72/72 adjacent-level cells passing. Every all-attempts pessimistic win bar also passes; because the nominal idle and unfinished requirements also pass, there are zero additional pessimistic overall misses. The eleven newly covered fighters account for88 primary and66 adjacent cells; the unchanged Ryu pair supplies8 and6 regression cells.

All 43,200 standard attempts are accounted for:42982 completed matches,218 native branch stops,zero unfinished andzero completed draws. Stops are separate from completed AI losses and remain tracked by the existing untranslated-branch issue270. No native branch fix is claimed by this delivery.

All 388 compatibility files,15,520 attempts perbinary, match byte for byte, including196 pre-existing native branch stops. The raw fingerprint is identical to the exact post-SC19 baseline. This preserves the reported old-era results, including their already tracked SF4/SF5 misses; it does not re-grade those earlier rollouts from this smaller control sample.

Development tests caught a real Easy input collision: a forward motion during a native cancel window could complete a paid double-Forward cancel Rush even when the route metadata spent no meter. The new-cast-only final pad guard reads only own command history/control/resources and leaves the native recognizer intact. Native Easy trials prove no unthreatened paid starts for all eleven new fighters.

The earlier integration build failed because its fixture imported a private live-move-list helper; the test was corrected without widening the production API. Early native fixture failures were diagnosed with retained logs and actual accepted commands, costs and collisions; native engine stops are now hard failures before any bounded proof exit, with no address or seed exclusions.

One exploratory forced-SA1 Birdie trial reached untranslated address 0x1823c7 at seed 4/frame 1286 after six native 48-point SA1 starts had already been observed. The red trace is retained at target/sc20/probe-super-birdie-proof-before-stop.log. Focused cost trials now stop at their first complete requested proof and fail any earlier engine stop; the full measuring runs retain all stopped attempts instead of using that early proof exit.

The first combined focused run had 427 passes and one failure: Gen entered no route in the two short Dramatic samples. The final fixture retains both original seeds within fixed seeds 1-8, raises the per-seed cap to4,800 frames and preserves all route, pad, hash and native-stop assertions. Gen entered routes in seeds 2, 4, 5, 6 and 8. The final committed source then passed all 428 focused tests and both complete suite passes.

The latest maintainer instruction permits suites optionally and supersedes the earlier blanket waiver for future SC work. Both full-suite passes actually ran for SC20; the historical SC18/SC19 waivers remain facts about those deliveries.

A separate 44-cell Dramatic diagnostic covers every new fighter as boss at every level, with Ryu and a different new fighter as independent allies. Its 1,757 completed matches, three stops and one draw are attached separately; no sampled boss/level went without a recorded round loss. This sampling result supplies no human playtest or whole-cast enablement approval.

## Per-fighter and per-pair results

| Fighter | Primary pass / cells | Adjacent pass / cells | Nominal misses | Pessimistic win misses | Stopped / unfinished |
| --- | --- | --- | --- | --- | --- |
| ryu | 8 / 8 | 6 / 6 | 0 | 0 | 1 / 0 |
| ken | 8 / 8 | 6 / 6 | 0 | 0 | 0 / 0 |
| retsu | 8 / 8 | 6 / 6 | 0 | 0 | 1 / 0 |
| geki | 8 / 8 | 6 / 6 | 0 | 0 | 3 / 0 |
| joe | 8 / 8 | 6 / 6 | 0 | 0 | 2 / 0 |
| mike | 8 / 8 | 6 / 6 | 0 | 0 | 6 / 0 |
| lee | 8 / 8 | 6 / 6 | 0 | 0 | 0 / 0 |
| gen | 8 / 8 | 6 / 6 | 0 | 0 | 62 / 0 |
| birdie | 8 / 8 | 6 / 6 | 0 | 0 | 50 / 0 |
| eagle | 8 / 8 | 6 / 6 | 0 | 0 | 88 / 0 |
| adon | 8 / 8 | 6 / 6 | 0 | 0 | 4 / 0 |
| sagat | 8 / 8 | 6 / 6 | 0 | 0 | 1 / 0 |

| Fighter | Style | Primary pass /8 | Adjacent pass /6 | Nominal reasons | Pessimistic win misses | Stopped / unfinished |
| --- | --- | --- | --- | --- | --- | --- |
| ryu | sf6 | 8 | 6 | none | 0 | 1 / 0 |
| ken | sf6 | 8 | 6 | none | 0 | 0 / 0 |
| retsu | sf6 | 8 | 6 | none | 0 | 1 / 0 |
| geki | sf6 | 8 | 6 | none | 0 | 3 / 0 |
| joe | sf6 | 8 | 6 | none | 0 | 2 / 0 |
| mike | sf6 | 8 | 6 | none | 0 | 6 / 0 |
| lee | sf6 | 8 | 6 | none | 0 | 0 / 0 |
| gen | sf6 | 8 | 6 | none | 0 | 62 / 0 |
| birdie | sf6 | 8 | 6 | none | 0 | 50 / 0 |
| eagle | sf6 | 8 | 6 | none | 0 | 88 / 0 |
| adon | sf6 | 8 | 6 | none | 0 | 4 / 0 |
| sagat | sf6 | 8 | 6 | none | 0 | 1 / 0 |

## Every nominal miss and measured reason

Reasons identify failed measured bars (`wins`, `idle`, `unfinished`), not an inferred gameplay root cause. Every failed cell remains in the complete tables below.

None.

## Pessimistic win misses

0 total; 0 are additional to nominally passing cells. These use all attempted seeds.

None.

### Additional misses beyond nominal passes

None.

## Complete final measuring tables

### All 96 new-versus-old cells

| Fighter | Style | Levels | Compared side | Finished W-L-D | Attempted / completed / stopped / unfinished | Idle first/second % | Attacks/min first/second | Nominal bar | Pessimistic wins | Raw file |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| ryu | sf6 | 1 | new P1 | 225-15-0 | 240 / 240 / 0 / 0 | 24.77/32.35 | 60.31/57.57 | pass | pass | ryu__sf6__new__old__1.tsv |
| ryu | sf6 | 1 | new P2 | 227-13-0 | 240 / 240 / 0 / 0 | 25.29/32.07 | 59.76/57.57 | pass | pass | ryu__sf6__old__new__1.tsv |
| ryu | sf6 | 2 | new P1 | 196-44-0 | 240 / 240 / 0 / 0 | 23.39/26.82 | 62.28/60.74 | pass | pass | ryu__sf6__new__old__2.tsv |
| ryu | sf6 | 2 | new P2 | 196-44-0 | 240 / 240 / 0 / 0 | 23.56/27.00 | 62.57/60.50 | pass | pass | ryu__sf6__old__new__2.tsv |
| ryu | sf6 | 3 | new P1 | 205-35-0 | 240 / 240 / 0 / 0 | 20.58/22.59 | 71.58/66.87 | pass | pass | ryu__sf6__new__old__3.tsv |
| ryu | sf6 | 3 | new P2 | 202-38-0 | 240 / 240 / 0 / 0 | 20.55/22.65 | 71.03/67.47 | pass | pass | ryu__sf6__old__new__3.tsv |
| ryu | sf6 | 4 | new P1 | 175-65-0 | 240 / 240 / 0 / 0 | 20.21/21.79 | 72.55/68.44 | pass | pass | ryu__sf6__new__old__4.tsv |
| ryu | sf6 | 4 | new P2 | 179-61-0 | 240 / 240 / 0 / 0 | 20.19/22.14 | 71.75/68.00 | pass | pass | ryu__sf6__old__new__4.tsv |
| ken | sf6 | 1 | new P1 | 220-20-0 | 240 / 240 / 0 / 0 | 29.21/34.85 | 60.15/59.75 | pass | pass | ken__sf6__new__old__1.tsv |
| ken | sf6 | 1 | new P2 | 218-22-0 | 240 / 240 / 0 / 0 | 29.01/34.83 | 59.97/59.62 | pass | pass | ken__sf6__old__new__1.tsv |
| ken | sf6 | 2 | new P1 | 224-16-0 | 240 / 240 / 0 / 0 | 25.67/31.61 | 65.29/58.25 | pass | pass | ken__sf6__new__old__2.tsv |
| ken | sf6 | 2 | new P2 | 226-14-0 | 240 / 240 / 0 / 0 | 25.89/30.91 | 65.93/59.03 | pass | pass | ken__sf6__old__new__2.tsv |
| ken | sf6 | 3 | new P1 | 218-22-0 | 240 / 240 / 0 / 0 | 23.09/27.25 | 72.85/66.36 | pass | pass | ken__sf6__new__old__3.tsv |
| ken | sf6 | 3 | new P2 | 220-20-0 | 240 / 240 / 0 / 0 | 23.37/27.28 | 72.37/65.77 | pass | pass | ken__sf6__old__new__3.tsv |
| ken | sf6 | 4 | new P1 | 205-35-0 | 240 / 240 / 0 / 0 | 22.05/28.06 | 72.32/66.29 | pass | pass | ken__sf6__new__old__4.tsv |
| ken | sf6 | 4 | new P2 | 195-45-0 | 240 / 240 / 0 / 0 | 21.74/28.32 | 72.37/65.91 | pass | pass | ken__sf6__old__new__4.tsv |
| retsu | sf6 | 1 | new P1 | 220-20-0 | 240 / 240 / 0 / 0 | 41.77/46.50 | 54.55/56.18 | pass | pass | retsu__sf6__new__old__1.tsv |
| retsu | sf6 | 1 | new P2 | 227-13-0 | 240 / 240 / 0 / 0 | 41.72/46.39 | 54.24/56.24 | pass | pass | retsu__sf6__old__new__1.tsv |
| retsu | sf6 | 2 | new P1 | 234-6-0 | 240 / 240 / 0 / 0 | 38.15/44.50 | 59.15/55.07 | pass | pass | retsu__sf6__new__old__2.tsv |
| retsu | sf6 | 2 | new P2 | 235-5-0 | 240 / 240 / 0 / 0 | 38.26/44.84 | 58.75/55.04 | pass | pass | retsu__sf6__old__new__2.tsv |
| retsu | sf6 | 3 | new P1 | 235-5-0 | 240 / 240 / 0 / 0 | 34.17/41.55 | 66.35/59.76 | pass | pass | retsu__sf6__new__old__3.tsv |
| retsu | sf6 | 3 | new P2 | 237-3-0 | 240 / 240 / 0 / 0 | 33.57/41.71 | 67.71/60.59 | pass | pass | retsu__sf6__old__new__3.tsv |
| retsu | sf6 | 4 | new P1 | 236-4-0 | 240 / 240 / 0 / 0 | 31.00/42.06 | 69.99/60.00 | pass | pass | retsu__sf6__new__old__4.tsv |
| retsu | sf6 | 4 | new P2 | 235-5-0 | 240 / 240 / 0 / 0 | 31.22/42.53 | 69.22/59.33 | pass | pass | retsu__sf6__old__new__4.tsv |
| geki | sf6 | 1 | new P1 | 229-11-0 | 240 / 240 / 0 / 0 | 35.60/45.23 | 60.19/57.65 | pass | pass | geki__sf6__new__old__1.tsv |
| geki | sf6 | 1 | new P2 | 229-11-0 | 240 / 240 / 0 / 0 | 35.17/45.79 | 60.92/57.33 | pass | pass | geki__sf6__old__new__1.tsv |
| geki | sf6 | 2 | new P1 | 237-3-0 | 240 / 240 / 0 / 0 | 31.46/43.59 | 64.07/58.60 | pass | pass | geki__sf6__new__old__2.tsv |
| geki | sf6 | 2 | new P2 | 238-2-0 | 240 / 240 / 0 / 0 | 31.26/43.88 | 63.78/59.74 | pass | pass | geki__sf6__old__new__2.tsv |
| geki | sf6 | 3 | new P1 | 237-3-0 | 240 / 240 / 0 / 0 | 28.63/39.43 | 67.94/65.42 | pass | pass | geki__sf6__new__old__3.tsv |
| geki | sf6 | 3 | new P2 | 236-4-0 | 240 / 240 / 0 / 0 | 28.46/39.69 | 68.77/65.78 | pass | pass | geki__sf6__old__new__3.tsv |
| geki | sf6 | 4 | new P1 | 233-7-0 | 240 / 240 / 0 / 0 | 28.30/40.31 | 70.06/67.37 | pass | pass | geki__sf6__new__old__4.tsv |
| geki | sf6 | 4 | new P2 | 235-5-0 | 240 / 240 / 0 / 0 | 28.33/39.78 | 70.28/68.01 | pass | pass | geki__sf6__old__new__4.tsv |
| joe | sf6 | 1 | new P1 | 219-21-0 | 240 / 240 / 0 / 0 | 37.64/46.10 | 49.99/40.88 | pass | pass | joe__sf6__new__old__1.tsv |
| joe | sf6 | 1 | new P2 | 214-26-0 | 240 / 240 / 0 / 0 | 37.52/45.87 | 50.27/41.59 | pass | pass | joe__sf6__old__new__1.tsv |
| joe | sf6 | 2 | new P1 | 236-4-0 | 240 / 240 / 0 / 0 | 33.24/44.06 | 52.29/37.02 | pass | pass | joe__sf6__new__old__2.tsv |
| joe | sf6 | 2 | new P2 | 235-5-0 | 240 / 240 / 0 / 0 | 33.17/43.92 | 52.72/37.19 | pass | pass | joe__sf6__old__new__2.tsv |
| joe | sf6 | 3 | new P1 | 235-4-0 | 240 / 239 / 1 / 0 | 28.01/43.70 | 64.92/40.04 | pass | pass | joe__sf6__new__old__3.tsv |
| joe | sf6 | 3 | new P2 | 235-4-0 | 240 / 239 / 1 / 0 | 27.94/44.26 | 64.68/39.59 | pass | pass | joe__sf6__old__new__3.tsv |
| joe | sf6 | 4 | new P1 | 239-1-0 | 240 / 240 / 0 / 0 | 27.02/48.58 | 70.12/39.37 | pass | pass | joe__sf6__new__old__4.tsv |
| joe | sf6 | 4 | new P2 | 238-2-0 | 240 / 240 / 0 / 0 | 26.55/48.92 | 69.83/38.42 | pass | pass | joe__sf6__old__new__4.tsv |
| mike | sf6 | 1 | new P1 | 237-3-0 | 240 / 240 / 0 / 0 | 40.30/46.48 | 48.23/39.43 | pass | pass | mike__sf6__new__old__1.tsv |
| mike | sf6 | 1 | new P2 | 223-17-0 | 240 / 240 / 0 / 0 | 39.91/46.55 | 48.21/41.48 | pass | pass | mike__sf6__old__new__1.tsv |
| mike | sf6 | 2 | new P1 | 237-3-0 | 240 / 240 / 0 / 0 | 34.47/42.71 | 56.25/43.46 | pass | pass | mike__sf6__new__old__2.tsv |
| mike | sf6 | 2 | new P2 | 224-14-0 | 240 / 238 / 2 / 0 | 34.49/42.76 | 53.98/45.47 | pass | pass | mike__sf6__old__new__2.tsv |
| mike | sf6 | 3 | new P1 | 232-8-0 | 240 / 240 / 0 / 0 | 32.20/38.94 | 64.72/48.25 | pass | pass | mike__sf6__new__old__3.tsv |
| mike | sf6 | 3 | new P2 | 233-7-0 | 240 / 240 / 0 / 0 | 32.48/39.41 | 62.33/49.87 | pass | pass | mike__sf6__old__new__3.tsv |
| mike | sf6 | 4 | new P1 | 239-1-0 | 240 / 240 / 0 / 0 | 30.53/38.76 | 69.54/46.38 | pass | pass | mike__sf6__new__old__4.tsv |
| mike | sf6 | 4 | new P2 | 240-0-0 | 240 / 240 / 0 / 0 | 30.82/39.28 | 68.24/50.83 | pass | pass | mike__sf6__old__new__4.tsv |
| lee | sf6 | 1 | new P1 | 193-47-0 | 240 / 240 / 0 / 0 | 44.69/55.16 | 48.90/40.07 | pass | pass | lee__sf6__new__old__1.tsv |
| lee | sf6 | 1 | new P2 | 193-47-0 | 240 / 240 / 0 / 0 | 44.32/55.05 | 49.11/39.97 | pass | pass | lee__sf6__old__new__1.tsv |
| lee | sf6 | 2 | new P1 | 205-35-0 | 240 / 240 / 0 / 0 | 40.09/50.50 | 52.69/43.11 | pass | pass | lee__sf6__new__old__2.tsv |
| lee | sf6 | 2 | new P2 | 211-29-0 | 240 / 240 / 0 / 0 | 39.70/50.90 | 52.98/43.27 | pass | pass | lee__sf6__old__new__2.tsv |
| lee | sf6 | 3 | new P1 | 189-51-0 | 240 / 240 / 0 / 0 | 39.93/45.47 | 55.28/48.66 | pass | pass | lee__sf6__new__old__3.tsv |
| lee | sf6 | 3 | new P2 | 182-58-0 | 240 / 240 / 0 / 0 | 39.81/45.59 | 55.63/49.41 | pass | pass | lee__sf6__old__new__3.tsv |
| lee | sf6 | 4 | new P1 | 233-7-0 | 240 / 240 / 0 / 0 | 38.94/46.98 | 56.55/44.58 | pass | pass | lee__sf6__new__old__4.tsv |
| lee | sf6 | 4 | new P2 | 225-15-0 | 240 / 240 / 0 / 0 | 38.76/47.25 | 55.99/44.78 | pass | pass | lee__sf6__old__new__4.tsv |
| gen | sf6 | 1 | new P1 | 216-18-0 | 240 / 234 / 6 / 0 | 45.75/51.96 | 53.46/53.52 | pass | pass | gen__sf6__new__old__1.tsv |
| gen | sf6 | 1 | new P2 | 205-32-0 | 240 / 237 / 3 / 0 | 46.07/52.83 | 52.89/53.31 | pass | pass | gen__sf6__old__new__1.tsv |
| gen | sf6 | 2 | new P1 | 231-6-0 | 240 / 237 / 3 / 0 | 45.57/48.75 | 55.76/57.78 | pass | pass | gen__sf6__new__old__2.tsv |
| gen | sf6 | 2 | new P2 | 232-4-0 | 240 / 236 / 4 / 0 | 45.31/48.46 | 55.85/58.09 | pass | pass | gen__sf6__old__new__2.tsv |
| gen | sf6 | 3 | new P1 | 228-11-0 | 240 / 239 / 1 / 0 | 39.98/45.32 | 66.20/66.67 | pass | pass | gen__sf6__new__old__3.tsv |
| gen | sf6 | 3 | new P2 | 234-5-0 | 240 / 239 / 1 / 0 | 40.93/44.98 | 64.99/65.45 | pass | pass | gen__sf6__old__new__3.tsv |
| gen | sf6 | 4 | new P1 | 236-3-0 | 240 / 239 / 1 / 0 | 38.05/44.31 | 69.97/69.54 | pass | pass | gen__sf6__new__old__4.tsv |
| gen | sf6 | 4 | new P2 | 232-3-0 | 240 / 235 / 5 / 0 | 37.92/44.92 | 70.06/69.94 | pass | pass | gen__sf6__old__new__4.tsv |
| birdie | sf6 | 1 | new P1 | 207-33-0 | 240 / 240 / 0 / 0 | 31.21/43.73 | 62.15/50.47 | pass | pass | birdie__sf6__new__old__1.tsv |
| birdie | sf6 | 1 | new P2 | 195-45-0 | 240 / 240 / 0 / 0 | 31.14/43.55 | 61.42/49.80 | pass | pass | birdie__sf6__old__new__1.tsv |
| birdie | sf6 | 2 | new P1 | 222-16-0 | 240 / 238 / 2 / 0 | 27.28/40.99 | 65.25/48.91 | pass | pass | birdie__sf6__new__old__2.tsv |
| birdie | sf6 | 2 | new P2 | 212-27-0 | 240 / 239 / 1 / 0 | 27.61/40.95 | 65.10/49.43 | pass | pass | birdie__sf6__old__new__2.tsv |
| birdie | sf6 | 3 | new P1 | 210-27-0 | 240 / 237 / 3 / 0 | 24.82/36.26 | 65.64/51.18 | pass | pass | birdie__sf6__new__old__3.tsv |
| birdie | sf6 | 3 | new P2 | 205-30-0 | 240 / 235 / 5 / 0 | 24.41/36.63 | 66.40/51.27 | pass | pass | birdie__sf6__old__new__3.tsv |
| birdie | sf6 | 4 | new P1 | 213-27-0 | 240 / 240 / 0 / 0 | 23.18/33.72 | 68.14/54.35 | pass | pass | birdie__sf6__new__old__4.tsv |
| birdie | sf6 | 4 | new P2 | 212-26-0 | 240 / 238 / 2 / 0 | 23.52/34.35 | 68.54/54.34 | pass | pass | birdie__sf6__old__new__4.tsv |
| eagle | sf6 | 1 | new P1 | 204-31-0 | 240 / 235 / 5 / 0 | 45.95/51.76 | 51.23/47.58 | pass | pass | eagle__sf6__new__old__1.tsv |
| eagle | sf6 | 1 | new P2 | 206-32-0 | 240 / 238 / 2 / 0 | 46.15/51.93 | 50.81/47.56 | pass | pass | eagle__sf6__old__new__1.tsv |
| eagle | sf6 | 2 | new P1 | 227-8-0 | 240 / 235 / 5 / 0 | 41.93/49.41 | 54.79/49.78 | pass | pass | eagle__sf6__new__old__2.tsv |
| eagle | sf6 | 2 | new P2 | 228-7-0 | 240 / 235 / 5 / 0 | 41.59/48.76 | 55.02/49.96 | pass | pass | eagle__sf6__old__new__2.tsv |
| eagle | sf6 | 3 | new P1 | 216-17-0 | 240 / 233 / 7 / 0 | 39.41/46.51 | 64.48/56.06 | pass | pass | eagle__sf6__new__old__3.tsv |
| eagle | sf6 | 3 | new P2 | 210-22-0 | 240 / 232 / 8 / 0 | 39.06/46.53 | 65.24/55.89 | pass | pass | eagle__sf6__old__new__3.tsv |
| eagle | sf6 | 4 | new P1 | 223-16-0 | 240 / 239 / 1 / 0 | 39.31/45.19 | 66.92/54.97 | pass | pass | eagle__sf6__new__old__4.tsv |
| eagle | sf6 | 4 | new P2 | 205-27-0 | 240 / 232 / 8 / 0 | 38.59/45.42 | 67.52/54.76 | pass | pass | eagle__sf6__old__new__4.tsv |
| adon | sf6 | 1 | new P1 | 212-28-0 | 240 / 240 / 0 / 0 | 42.32/48.84 | 49.77/41.21 | pass | pass | adon__sf6__new__old__1.tsv |
| adon | sf6 | 1 | new P2 | 216-24-0 | 240 / 240 / 0 / 0 | 42.37/48.82 | 49.58/41.34 | pass | pass | adon__sf6__old__new__1.tsv |
| adon | sf6 | 2 | new P1 | 227-13-0 | 240 / 240 / 0 / 0 | 38.82/47.02 | 55.07/44.46 | pass | pass | adon__sf6__new__old__2.tsv |
| adon | sf6 | 2 | new P2 | 227-13-0 | 240 / 240 / 0 / 0 | 39.31/47.31 | 54.61/44.20 | pass | pass | adon__sf6__old__new__2.tsv |
| adon | sf6 | 3 | new P1 | 232-8-0 | 240 / 240 / 0 / 0 | 40.47/45.58 | 55.21/41.70 | pass | pass | adon__sf6__new__old__3.tsv |
| adon | sf6 | 3 | new P2 | 232-8-0 | 240 / 240 / 0 / 0 | 40.15/45.17 | 55.80/44.01 | pass | pass | adon__sf6__old__new__3.tsv |
| adon | sf6 | 4 | new P1 | 227-13-0 | 240 / 240 / 0 / 0 | 37.25/47.69 | 55.47/40.42 | pass | pass | adon__sf6__new__old__4.tsv |
| adon | sf6 | 4 | new P2 | 231-9-0 | 240 / 240 / 0 / 0 | 37.31/48.00 | 56.18/41.01 | pass | pass | adon__sf6__old__new__4.tsv |
| sagat | sf6 | 1 | new P1 | 222-17-0 | 240 / 239 / 1 / 0 | 39.10/48.48 | 50.98/46.03 | pass | pass | sagat__sf6__new__old__1.tsv |
| sagat | sf6 | 1 | new P2 | 221-19-0 | 240 / 240 / 0 / 0 | 38.98/48.69 | 50.76/45.79 | pass | pass | sagat__sf6__old__new__1.tsv |
| sagat | sf6 | 2 | new P1 | 233-7-0 | 240 / 240 / 0 / 0 | 35.95/44.74 | 55.06/43.58 | pass | pass | sagat__sf6__new__old__2.tsv |
| sagat | sf6 | 2 | new P2 | 233-7-0 | 240 / 240 / 0 / 0 | 35.83/44.54 | 54.33/44.04 | pass | pass | sagat__sf6__old__new__2.tsv |
| sagat | sf6 | 3 | new P1 | 238-2-0 | 240 / 240 / 0 / 0 | 32.38/41.39 | 61.53/49.81 | pass | pass | sagat__sf6__new__old__3.tsv |
| sagat | sf6 | 3 | new P2 | 237-3-0 | 240 / 240 / 0 / 0 | 32.50/41.92 | 61.15/49.25 | pass | pass | sagat__sf6__old__new__3.tsv |
| sagat | sf6 | 4 | new P1 | 238-2-0 | 240 / 240 / 0 / 0 | 31.41/43.01 | 63.01/49.33 | pass | pass | sagat__sf6__new__old__4.tsv |
| sagat | sf6 | 4 | new P2 | 237-3-0 | 240 / 240 / 0 / 0 | 31.65/42.32 | 61.77/49.36 | pass | pass | sagat__sf6__old__new__4.tsv |


### All 72 adjacent-level cells

| Fighter | Style | Levels | Compared side | Finished W-L-D | Attempted / completed / stopped / unfinished | Idle first/second % | Attacks/min first/second | Nominal bar | Pessimistic wins | Raw file |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| ryu | sf6 | 2/1 | higher P1 | 198-42-0 | 240 / 240 / 0 / 0 | 22.14/23.43 | 54.52/51.26 | pass | pass | ryu__sf6__new__new__2-1.tsv |
| ryu | sf6 | 1/2 | higher P2 | 190-49-0 | 240 / 239 / 1 / 0 | 22.34/23.63 | 54.72/50.92 | pass | pass | ryu__sf6__new__new__1-2.tsv |
| ryu | sf6 | 3/2 | higher P1 | 219-21-0 | 240 / 240 / 0 / 0 | 19.75/22.19 | 63.98/52.89 | pass | pass | ryu__sf6__new__new__3-2.tsv |
| ryu | sf6 | 2/3 | higher P2 | 208-32-0 | 240 / 240 / 0 / 0 | 19.27/22.50 | 63.13/52.01 | pass | pass | ryu__sf6__new__new__2-3.tsv |
| ryu | sf6 | 4/3 | higher P1 | 192-48-0 | 240 / 240 / 0 / 0 | 18.95/19.19 | 65.55/61.45 | pass | pass | ryu__sf6__new__new__4-3.tsv |
| ryu | sf6 | 3/4 | higher P2 | 188-52-0 | 240 / 240 / 0 / 0 | 18.78/19.33 | 65.23/62.22 | pass | pass | ryu__sf6__new__new__3-4.tsv |
| ken | sf6 | 2/1 | higher P1 | 218-22-0 | 240 / 240 / 0 / 0 | 26.03/28.43 | 55.92/47.94 | pass | pass | ken__sf6__new__new__2-1.tsv |
| ken | sf6 | 1/2 | higher P2 | 224-16-0 | 240 / 240 / 0 / 0 | 26.04/28.04 | 55.81/48.55 | pass | pass | ken__sf6__new__new__1-2.tsv |
| ken | sf6 | 3/2 | higher P1 | 205-35-0 | 240 / 240 / 0 / 0 | 23.58/24.80 | 60.65/53.73 | pass | pass | ken__sf6__new__new__3-2.tsv |
| ken | sf6 | 2/3 | higher P2 | 211-29-0 | 240 / 240 / 0 / 0 | 23.38/24.86 | 61.28/53.49 | pass | pass | ken__sf6__new__new__2-3.tsv |
| ken | sf6 | 4/3 | higher P1 | 198-42-0 | 240 / 240 / 0 / 0 | 21.75/22.04 | 62.75/59.54 | pass | pass | ken__sf6__new__new__4-3.tsv |
| ken | sf6 | 3/4 | higher P2 | 208-32-0 | 240 / 240 / 0 / 0 | 22.11/21.80 | 61.50/58.63 | pass | pass | ken__sf6__new__new__3-4.tsv |
| retsu | sf6 | 2/1 | higher P1 | 183-57-0 | 240 / 240 / 0 / 0 | 40.67/42.74 | 51.31/45.31 | pass | pass | retsu__sf6__new__new__2-1.tsv |
| retsu | sf6 | 1/2 | higher P2 | 165-74-0 | 240 / 239 / 1 / 0 | 40.70/42.39 | 51.41/46.03 | pass | pass | retsu__sf6__new__new__1-2.tsv |
| retsu | sf6 | 3/2 | higher P1 | 181-59-0 | 240 / 240 / 0 / 0 | 32.97/38.04 | 61.47/54.65 | pass | pass | retsu__sf6__new__new__3-2.tsv |
| retsu | sf6 | 2/3 | higher P2 | 161-79-0 | 240 / 240 / 0 / 0 | 33.19/37.51 | 61.19/55.01 | pass | pass | retsu__sf6__new__new__2-3.tsv |
| retsu | sf6 | 4/3 | higher P1 | 191-49-0 | 240 / 240 / 0 / 0 | 31.17/31.33 | 64.29/63.48 | pass | pass | retsu__sf6__new__new__4-3.tsv |
| retsu | sf6 | 3/4 | higher P2 | 184-56-0 | 240 / 240 / 0 / 0 | 31.02/31.52 | 64.34/63.45 | pass | pass | retsu__sf6__new__new__3-4.tsv |
| geki | sf6 | 2/1 | higher P1 | 168-71-0 | 240 / 239 / 1 / 0 | 32.26/35.84 | 58.71/54.08 | pass | pass | geki__sf6__new__new__2-1.tsv |
| geki | sf6 | 1/2 | higher P2 | 158-82-0 | 240 / 240 / 0 / 0 | 32.08/36.09 | 58.12/54.05 | pass | pass | geki__sf6__new__new__1-2.tsv |
| geki | sf6 | 3/2 | higher P1 | 179-60-0 | 240 / 239 / 1 / 0 | 28.95/32.15 | 60.57/55.70 | pass | pass | geki__sf6__new__new__3-2.tsv |
| geki | sf6 | 2/3 | higher P2 | 178-62-0 | 240 / 240 / 0 / 0 | 29.10/32.24 | 61.14/56.14 | pass | pass | geki__sf6__new__new__2-3.tsv |
| geki | sf6 | 4/3 | higher P1 | 180-60-0 | 240 / 240 / 0 / 0 | 27.60/28.25 | 64.16/62.13 | pass | pass | geki__sf6__new__new__4-3.tsv |
| geki | sf6 | 3/4 | higher P2 | 189-50-0 | 240 / 239 / 1 / 0 | 28.19/28.81 | 63.70/62.57 | pass | pass | geki__sf6__new__new__3-4.tsv |
| joe | sf6 | 2/1 | higher P1 | 217-23-0 | 240 / 240 / 0 / 0 | 34.21/37.82 | 47.91/39.03 | pass | pass | joe__sf6__new__new__2-1.tsv |
| joe | sf6 | 1/2 | higher P2 | 203-37-0 | 240 / 240 / 0 / 0 | 34.68/37.74 | 48.07/39.51 | pass | pass | joe__sf6__new__new__1-2.tsv |
| joe | sf6 | 3/2 | higher P1 | 234-6-0 | 240 / 240 / 0 / 0 | 30.55/33.28 | 57.12/46.57 | pass | pass | joe__sf6__new__new__3-2.tsv |
| joe | sf6 | 2/3 | higher P2 | 230-10-0 | 240 / 240 / 0 / 0 | 31.02/33.24 | 56.98/46.60 | pass | pass | joe__sf6__new__new__2-3.tsv |
| joe | sf6 | 4/3 | higher P1 | 229-11-0 | 240 / 240 / 0 / 0 | 30.85/30.49 | 62.75/60.52 | pass | pass | joe__sf6__new__new__4-3.tsv |
| joe | sf6 | 3/4 | higher P2 | 224-16-0 | 240 / 240 / 0 / 0 | 30.98/30.56 | 63.11/60.51 | pass | pass | joe__sf6__new__new__3-4.tsv |
| mike | sf6 | 2/1 | higher P1 | 204-34-0 | 240 / 238 / 2 / 0 | 34.97/38.22 | 50.04/43.10 | pass | pass | mike__sf6__new__new__2-1.tsv |
| mike | sf6 | 1/2 | higher P2 | 185-54-0 | 240 / 239 / 1 / 0 | 35.14/39.18 | 49.40/43.06 | pass | pass | mike__sf6__new__new__1-2.tsv |
| mike | sf6 | 3/2 | higher P1 | 173-67-0 | 240 / 240 / 0 / 0 | 32.23/32.60 | 58.83/52.18 | pass | pass | mike__sf6__new__new__3-2.tsv |
| mike | sf6 | 2/3 | higher P2 | 153-87-0 | 240 / 240 / 0 / 0 | 32.37/33.00 | 57.24/51.80 | pass | pass | mike__sf6__new__new__2-3.tsv |
| mike | sf6 | 4/3 | higher P1 | 178-62-0 | 240 / 240 / 0 / 0 | 29.70/31.10 | 63.23/57.40 | pass | pass | mike__sf6__new__new__4-3.tsv |
| mike | sf6 | 3/4 | higher P2 | 150-89-0 | 240 / 239 / 1 / 0 | 30.42/30.97 | 60.63/59.27 | pass | pass | mike__sf6__new__new__3-4.tsv |
| lee | sf6 | 2/1 | higher P1 | 163-77-0 | 240 / 240 / 0 / 0 | 40.60/41.86 | 49.91/48.67 | pass | pass | lee__sf6__new__new__2-1.tsv |
| lee | sf6 | 1/2 | higher P2 | 151-89-0 | 240 / 240 / 0 / 0 | 40.79/42.55 | 49.43/48.62 | pass | pass | lee__sf6__new__new__1-2.tsv |
| lee | sf6 | 3/2 | higher P1 | 146-94-0 | 240 / 240 / 0 / 0 | 41.72/38.50 | 49.79/50.09 | pass | pass | lee__sf6__new__new__3-2.tsv |
| lee | sf6 | 2/3 | higher P2 | 144-96-0 | 240 / 240 / 0 / 0 | 41.70/38.22 | 49.83/50.88 | pass | pass | lee__sf6__new__new__2-3.tsv |
| lee | sf6 | 4/3 | higher P1 | 171-69-0 | 240 / 240 / 0 / 0 | 38.47/38.93 | 52.96/50.99 | pass | pass | lee__sf6__new__new__4-3.tsv |
| lee | sf6 | 3/4 | higher P2 | 163-77-0 | 240 / 240 / 0 / 0 | 38.77/38.30 | 52.22/51.22 | pass | pass | lee__sf6__new__new__3-4.tsv |
| gen | sf6 | 2/1 | higher P1 | 192-43-0 | 240 / 235 / 5 / 0 | 41.52/43.85 | 57.86/53.10 | pass | pass | gen__sf6__new__new__2-1.tsv |
| gen | sf6 | 1/2 | higher P2 | 185-48-0 | 240 / 233 / 7 / 0 | 41.17/44.23 | 58.32/52.64 | pass | pass | gen__sf6__new__new__1-2.tsv |
| gen | sf6 | 3/2 | higher P1 | 131-100-0 | 240 / 231 / 9 / 0 | 39.45/40.49 | 62.56/58.71 | pass | pass | gen__sf6__new__new__3-2.tsv |
| gen | sf6 | 2/3 | higher P2 | 123-108-0 | 240 / 231 / 9 / 0 | 38.28/40.33 | 63.02/58.88 | pass | pass | gen__sf6__new__new__2-3.tsv |
| gen | sf6 | 4/3 | higher P1 | 154-83-0 | 240 / 237 / 3 / 0 | 37.13/37.15 | 64.39/64.45 | pass | pass | gen__sf6__new__new__4-3.tsv |
| gen | sf6 | 3/4 | higher P2 | 143-94-0 | 240 / 237 / 3 / 0 | 37.96/37.62 | 64.40/63.83 | pass | pass | gen__sf6__new__new__3-4.tsv |
| birdie | sf6 | 2/1 | higher P1 | 204-31-0 | 240 / 235 / 5 / 0 | 27.61/32.09 | 63.84/56.94 | pass | pass | birdie__sf6__new__new__2-1.tsv |
| birdie | sf6 | 1/2 | higher P2 | 191-41-0 | 240 / 232 / 8 / 0 | 28.05/31.75 | 63.99/57.51 | pass | pass | birdie__sf6__new__new__1-2.tsv |
| birdie | sf6 | 3/2 | higher P1 | 162-72-0 | 240 / 234 / 6 / 0 | 24.30/28.21 | 63.97/60.39 | pass | pass | birdie__sf6__new__new__3-2.tsv |
| birdie | sf6 | 2/3 | higher P2 | 163-71-0 | 240 / 234 / 6 / 0 | 24.40/27.99 | 63.52/60.13 | pass | pass | birdie__sf6__new__new__2-3.tsv |
| birdie | sf6 | 4/3 | higher P1 | 187-48-0 | 240 / 235 / 5 / 0 | 22.89/26.39 | 62.66/61.22 | pass | pass | birdie__sf6__new__new__4-3.tsv |
| birdie | sf6 | 3/4 | higher P2 | 193-41-0 | 240 / 234 / 6 / 0 | 22.59/26.58 | 64.23/60.83 | pass | pass | birdie__sf6__new__new__3-4.tsv |
| eagle | sf6 | 2/1 | higher P1 | 211-25-0 | 240 / 236 / 4 / 0 | 43.50/43.98 | 55.73/49.08 | pass | pass | eagle__sf6__new__new__2-1.tsv |
| eagle | sf6 | 1/2 | higher P2 | 200-36-0 | 240 / 236 / 4 / 0 | 42.95/43.89 | 54.92/48.48 | pass | pass | eagle__sf6__new__new__1-2.tsv |
| eagle | sf6 | 3/2 | higher P1 | 122-110-0 | 240 / 232 / 8 / 0 | 41.92/38.57 | 58.48/53.51 | pass | pass | eagle__sf6__new__new__3-2.tsv |
| eagle | sf6 | 2/3 | higher P2 | 138-94-0 | 240 / 232 / 8 / 0 | 41.58/39.14 | 58.30/53.06 | pass | pass | eagle__sf6__new__new__2-3.tsv |
| eagle | sf6 | 4/3 | higher P1 | 158-72-0 | 240 / 230 / 10 / 0 | 39.09/38.81 | 62.91/61.61 | pass | pass | eagle__sf6__new__new__4-3.tsv |
| eagle | sf6 | 3/4 | higher P2 | 150-82-0 | 240 / 232 / 8 / 0 | 39.11/39.06 | 62.97/62.06 | pass | pass | eagle__sf6__new__new__3-4.tsv |
| adon | sf6 | 2/1 | higher P1 | 187-53-0 | 240 / 240 / 0 / 0 | 39.20/41.14 | 52.41/45.18 | pass | pass | adon__sf6__new__new__2-1.tsv |
| adon | sf6 | 1/2 | higher P2 | 168-72-0 | 240 / 240 / 0 / 0 | 39.32/41.36 | 52.50/45.66 | pass | pass | adon__sf6__new__new__1-2.tsv |
| adon | sf6 | 3/2 | higher P1 | 152-87-0 | 240 / 239 / 1 / 0 | 41.39/37.43 | 52.57/50.93 | pass | pass | adon__sf6__new__new__3-2.tsv |
| adon | sf6 | 2/3 | higher P2 | 126-114-0 | 240 / 240 / 0 / 0 | 40.92/37.67 | 52.94/51.78 | pass | pass | adon__sf6__new__new__2-3.tsv |
| adon | sf6 | 4/3 | higher P1 | 177-63-0 | 240 / 240 / 0 / 0 | 41.99/39.95 | 49.89/49.71 | pass | pass | adon__sf6__new__new__4-3.tsv |
| adon | sf6 | 3/4 | higher P2 | 176-63-0 | 240 / 239 / 1 / 0 | 41.59/40.05 | 50.42/50.21 | pass | pass | adon__sf6__new__new__3-4.tsv |
| sagat | sf6 | 2/1 | higher P1 | 220-20-0 | 240 / 240 / 0 / 0 | 37.16/39.10 | 52.51/44.84 | pass | pass | sagat__sf6__new__new__2-1.tsv |
| sagat | sf6 | 1/2 | higher P2 | 216-24-0 | 240 / 240 / 0 / 0 | 37.51/38.82 | 51.19/44.39 | pass | pass | sagat__sf6__new__new__1-2.tsv |
| sagat | sf6 | 3/2 | higher P1 | 199-41-0 | 240 / 240 / 0 / 0 | 32.24/35.56 | 56.22/49.55 | pass | pass | sagat__sf6__new__new__3-2.tsv |
| sagat | sf6 | 2/3 | higher P2 | 177-63-0 | 240 / 240 / 0 / 0 | 32.04/35.42 | 56.37/50.42 | pass | pass | sagat__sf6__new__new__2-3.tsv |
| sagat | sf6 | 4/3 | higher P1 | 200-40-0 | 240 / 240 / 0 / 0 | 30.57/32.73 | 60.23/55.83 | pass | pass | sagat__sf6__new__new__4-3.tsv |
| sagat | sf6 | 3/4 | higher P2 | 196-44-0 | 240 / 240 / 0 / 0 | 30.80/33.10 | 60.38/55.36 | pass | pass | sagat__sf6__new__new__3-4.tsv |


### All 12 old-versus-old controls

| Fighter | Style | Levels | Compared side | Finished W-L-D | Attempted / completed / stopped / unfinished | Idle first/second % | Attacks/min first/second | Nominal bar | Pessimistic wins | Raw file |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| ryu | sf6 | 4 | old vs old | 97-143-0 | 240 / 240 / 0 / 0 | 22.25/20.64 | 76.17/77.03 | - | - | ryu__sf6__old__old__4.tsv |
| ken | sf6 | 4 | old vs old | 104-136-0 | 240 / 240 / 0 / 0 | 24.37/23.36 | 77.09/77.03 | - | - | ken__sf6__old__old__4.tsv |
| retsu | sf6 | 4 | old vs old | 114-126-0 | 240 / 240 / 0 / 0 | 38.54/38.42 | 74.47/74.36 | - | - | retsu__sf6__old__old__4.tsv |
| geki | sf6 | 4 | old vs old | 129-111-0 | 240 / 240 / 0 / 0 | 38.16/38.00 | 70.52/71.16 | - | - | geki__sf6__old__old__4.tsv |
| joe | sf6 | 4 | old vs old | 119-121-0 | 240 / 240 / 0 / 0 | 40.03/40.24 | 53.07/53.44 | - | - | joe__sf6__old__old__4.tsv |
| mike | sf6 | 4 | old vs old | 128-112-0 | 240 / 240 / 0 / 0 | 39.36/38.85 | 56.66/56.38 | - | - | mike__sf6__old__old__4.tsv |
| lee | sf6 | 4 | old vs old | 124-116-0 | 240 / 240 / 0 / 0 | 46.98/47.12 | 52.89/52.04 | - | - | lee__sf6__old__old__4.tsv |
| gen | sf6 | 4 | old vs old | 140-98-0 | 240 / 238 / 2 / 0 | 44.92/45.09 | 68.58/68.06 | - | - | gen__sf6__old__old__4.tsv |
| birdie | sf6 | 4 | old vs old | 132-107-0 | 240 / 239 / 1 / 0 | 35.02/34.98 | 58.55/58.25 | - | - | birdie__sf6__old__old__4.tsv |
| eagle | sf6 | 4 | old vs old | 110-125-0 | 240 / 235 / 5 / 0 | 43.75/42.98 | 62.14/62.74 | - | - | eagle__sf6__old__old__4.tsv |
| adon | sf6 | 4 | old vs old | 113-125-0 | 240 / 238 / 2 / 0 | 44.22/43.64 | 51.97/52.20 | - | - | adon__sf6__old__old__4.tsv |
| sagat | sf6 | 4 | old vs old | 123-117-0 | 240 / 240 / 0 / 0 | 41.99/42.01 | 61.27/60.96 | - | - | sagat__sf6__old__old__4.tsv |


## Complete descriptive opening metrics

All 180 validated final raw files are included below. First/second uses the same compared side as the gate tables: new/old in primary cells, higher/lower in adjacent cells, and P1/P2 in old/old controls. These descriptive values add no acceptance bars.

Recorded rounds and opening counts are summed as integers. HPO is hits per connected opening; DPO is damage per opening in health-bar points (the native ratio divides sub-HP damage by openings times 20). Follow rate is the fraction of openings with a later hit from a different attack, from 0 to 1; it is not a percentage.

The TSV stores each per-match ratio rounded to two decimals, not the underlying hit, damage or followed-opening counter totals. The displayed values are opening-count-weighted means of those reported rounded ratios, separately for each fighter: exact Decimal hundredths times that fighter's opening count, divided by its total openings. They are not exact aggregate ratios reconstructed from the underlying counters. Display uses four Decimal places; a zero-opening side displays 0.0000 and every reported zero-opening ratio is required to be 0.00. Live-frame counts are not weights for these opening metrics.

All recorded rows contribute, including unfinished progress if present; stopped attempts have no descriptive metrics. The native observer has limits: frame attribution of a health drop versus guard/hit reaction has not been validated move by move, so a hit may be filed as chip or conversely; the same attack repeated without an intervening different frame counts as one attack start; round winners/counts are observed from health bars rather than the native round-result event.

| Fighter | Style | Levels | Compared side | Recorded rounds first/second | Recorded openings first/second | Opening-weighted reported HPO first/second | Opening-weighted reported DPO first/second | Opening-weighted reported follow fraction first/second | Raw file |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| ryu | sf6 | 4 | old vs old | 256/329 | 3103/3265 | 1.1363/1.1518 | 8.0276/8.0206 | 0.0934/0.0987 | ryu__sf6__old__old__4.tsv |
| ryu | sf6 | 1 | new P1 | 459/67 | 3565/1829 | 1.0181/1.0211 | 7.0278/6.9871 | 0.0170/0.0113 | ryu__sf6__new__old__1.tsv |
| ryu | sf6 | 1 | new P2 | 463/82 | 3666/2059 | 1.0207/1.0193 | 7.0559/6.9177 | 0.0198/0.0058 | ryu__sf6__old__new__1.tsv |
| ryu | sf6 | 2 | new P1 | 420/147 | 3612/2250 | 1.0736/1.1283 | 7.2737/7.9032 | 0.0396/0.0813 | ryu__sf6__new__old__2.tsv |
| ryu | sf6 | 2 | new P2 | 420/155 | 3720/2353 | 1.0775/1.1240 | 7.2244/7.7482 | 0.0434/0.0763 | ryu__sf6__old__new__2.tsv |
| ryu | sf6 | 3 | new P1 | 423/136 | 3674/2179 | 1.0578/1.0999 | 7.2916/8.0345 | 0.0365/0.0745 | ryu__sf6__new__old__3.tsv |
| ryu | sf6 | 3 | new P2 | 430/148 | 3789/2334 | 1.0551/1.0919 | 7.3073/7.9485 | 0.0289/0.0699 | ryu__sf6__old__new__3.tsv |
| ryu | sf6 | 4 | new P1 | 387/197 | 3642/2417 | 1.0581/1.1714 | 7.2439/8.2833 | 0.0360/0.1181 | ryu__sf6__new__old__4.tsv |
| ryu | sf6 | 4 | new P2 | 393/188 | 3693/2473 | 1.0578/1.1505 | 7.2716/8.1915 | 0.0375/0.1037 | ryu__sf6__old__new__4.tsv |
| ryu | sf6 | 2/1 | higher P1 | 421/166 | 3573/2726 | 1.0753/1.0113 | 7.1091/7.0436 | 0.0439/0.0106 | ryu__sf6__new__new__2-1.tsv |
| ryu | sf6 | 1/2 | higher P2 | 419/172 | 3645/2761 | 1.0763/1.0110 | 7.0592/7.0556 | 0.0412/0.0103 | ryu__sf6__new__new__1-2.tsv |
| ryu | sf6 | 3/2 | higher P1 | 453/102 | 3524/2280 | 1.0595/1.0948 | 7.1674/6.9535 | 0.0408/0.0507 | ryu__sf6__new__new__3-2.tsv |
| ryu | sf6 | 2/3 | higher P2 | 436/121 | 3458/2280 | 1.0665/1.0983 | 7.2060/7.0011 | 0.0396/0.0542 | ryu__sf6__new__new__2-3.tsv |
| ryu | sf6 | 4/3 | higher P1 | 415/156 | 3581/2576 | 1.0609/1.0922 | 6.9871/7.0618 | 0.0382/0.0548 | ryu__sf6__new__new__4-3.tsv |
| ryu | sf6 | 3/4 | higher P2 | 404/184 | 3654/2757 | 1.0589/1.0927 | 7.0055/6.8559 | 0.0396/0.0534 | ryu__sf6__new__new__3-4.tsv |
| ken | sf6 | 4 | old vs old | 271/321 | 3332/3432 | 1.1108/1.1073 | 8.0001/8.0292 | 0.0987/0.0942 | ken__sf6__old__old__4.tsv |
| ken | sf6 | 1 | new P1 | 451/93 | 3583/2164 | 1.0342/1.0234 | 7.2550/7.0092 | 0.0232/0.0051 | ken__sf6__new__old__1.tsv |
| ken | sf6 | 1 | new P2 | 449/102 | 3606/2284 | 1.0323/1.0243 | 7.2657/7.0189 | 0.0212/0.0093 | ken__sf6__old__new__1.tsv |
| ken | sf6 | 2 | new P1 | 460/77 | 3466/1877 | 1.0640/1.0611 | 7.7463/7.8277 | 0.0325/0.0511 | ken__sf6__new__old__2.tsv |
| ken | sf6 | 2 | new P2 | 461/69 | 3420/1829 | 1.0662/1.0599 | 7.7773/7.8149 | 0.0351/0.0516 | ken__sf6__old__new__2.tsv |
| ken | sf6 | 3 | new P1 | 448/93 | 3479/2042 | 1.0636/1.0721 | 7.6973/7.9322 | 0.0326/0.0614 | ken__sf6__new__old__3.tsv |
| ken | sf6 | 3 | new P2 | 450/96 | 3510/2010 | 1.0649/1.0628 | 7.6764/8.0023 | 0.0317/0.0562 | ken__sf6__old__new__3.tsv |
| ken | sf6 | 4 | new P1 | 431/141 | 3606/2409 | 1.0619/1.0759 | 7.5526/7.6719 | 0.0293/0.0659 | ken__sf6__new__old__4.tsv |
| ken | sf6 | 4 | new P2 | 418/151 | 3591/2452 | 1.0592/1.0761 | 7.5331/7.6849 | 0.0283/0.0627 | ken__sf6__old__new__4.tsv |
| ken | sf6 | 2/1 | higher P1 | 451/98 | 3156/2059 | 1.0559/1.0431 | 7.9118/7.5588 | 0.0257/0.0296 | ken__sf6__new__new__2-1.tsv |
| ken | sf6 | 1/2 | higher P2 | 458/98 | 3205/2091 | 1.0527/1.0334 | 7.8616/7.6107 | 0.0278/0.0234 | ken__sf6__new__new__1-2.tsv |
| ken | sf6 | 3/2 | higher P1 | 431/141 | 3273/2223 | 1.0380/1.0892 | 7.8445/8.0262 | 0.0174/0.0439 | ken__sf6__new__new__3-2.tsv |
| ken | sf6 | 2/3 | higher P2 | 442/119 | 3215/2135 | 1.0477/1.0873 | 7.8361/7.9930 | 0.0215/0.0428 | ken__sf6__new__new__2-3.tsv |
| ken | sf6 | 4/3 | higher P1 | 421/138 | 3185/2335 | 1.0511/1.0774 | 7.8475/7.9288 | 0.0253/0.0415 | ken__sf6__new__new__4-3.tsv |
| ken | sf6 | 3/4 | higher P2 | 434/122 | 3158/2218 | 1.0420/1.0744 | 7.8431/7.9671 | 0.0196/0.0368 | ken__sf6__new__new__3-4.tsv |
| retsu | sf6 | 4 | old vs old | 285/302 | 2439/2464 | 1.2463/1.2590 | 9.4789/9.5312 | 0.2192/0.2269 | retsu__sf6__old__old__4.tsv |
| retsu | sf6 | 1 | new P1 | 456/101 | 3061/1680 | 1.0528/1.0592 | 8.3401/8.2592 | 0.0377/0.0414 | retsu__sf6__new__old__1.tsv |
| retsu | sf6 | 1 | new P2 | 461/75 | 2965/1528 | 1.0539/1.0540 | 8.3749/8.4184 | 0.0373/0.0468 | retsu__sf6__old__new__1.tsv |
| retsu | sf6 | 2 | new P1 | 472/39 | 2526/1051 | 1.1014/1.1176 | 9.7287/9.0135 | 0.0529/0.1004 | retsu__sf6__new__old__2.tsv |
| retsu | sf6 | 2 | new P2 | 473/46 | 2578/1158 | 1.0871/1.1280 | 9.5986/9.0642 | 0.0442/0.1108 | retsu__sf6__old__new__2.tsv |
| retsu | sf6 | 3 | new P1 | 473/47 | 2526/1168 | 1.0601/1.1418 | 9.9641/8.9984 | 0.0334/0.1195 | retsu__sf6__new__old__3.tsv |
| retsu | sf6 | 3 | new P2 | 475/44 | 2564/1209 | 1.0756/1.1403 | 9.9334/8.9436 | 0.0429/0.1155 | retsu__sf6__old__new__3.tsv |
| retsu | sf6 | 4 | new P1 | 473/32 | 2503/1241 | 1.0723/1.1585 | 9.6140/8.1930 | 0.0421/0.1280 | retsu__sf6__new__old__4.tsv |
| retsu | sf6 | 4 | new P2 | 473/33 | 2475/1253 | 1.0688/1.1363 | 9.7713/8.0838 | 0.0401/0.1121 | retsu__sf6__old__new__4.tsv |
| retsu | sf6 | 2/1 | higher P1 | 403/191 | 2581/2122 | 1.0750/1.0272 | 9.5741/9.1350 | 0.0397/0.0201 | retsu__sf6__new__new__2-1.tsv |
| retsu | sf6 | 1/2 | higher P2 | 374/205 | 2469/2163 | 1.0785/1.0396 | 9.5803/9.1334 | 0.0458/0.0272 | retsu__sf6__new__new__1-2.tsv |
| retsu | sf6 | 3/2 | higher P1 | 398/195 | 2395/1922 | 1.0409/1.0689 | 10.3443/10.0938 | 0.0220/0.0366 | retsu__sf6__new__new__3-2.tsv |
| retsu | sf6 | 2/3 | higher P2 | 371/233 | 2392/2066 | 1.0411/1.0556 | 10.2835/10.1647 | 0.0228/0.0331 | retsu__sf6__new__new__2-3.tsv |
| retsu | sf6 | 4/3 | higher P1 | 409/171 | 2416/1784 | 1.0466/1.0704 | 10.3690/10.3320 | 0.0303/0.0367 | retsu__sf6__new__new__4-3.tsv |
| retsu | sf6 | 3/4 | higher P2 | 398/178 | 2382/1772 | 1.0537/1.0589 | 10.2679/10.4351 | 0.0300/0.0392 | retsu__sf6__new__new__3-4.tsv |
| geki | sf6 | 4 | old vs old | 309/287 | 4240/4141 | 1.0173/1.0244 | 5.5403/5.5817 | 0.0003/0.0003 | geki__sf6__old__old__4.tsv |
| geki | sf6 | 1 | new P1 | 465/80 | 4197/2541 | 1.0396/1.0103 | 5.6637/5.6465 | 0.0263/0.0004 | geki__sf6__new__old__1.tsv |
| geki | sf6 | 1 | new P2 | 465/67 | 4163/2528 | 1.0371/1.0055 | 5.6815/5.6189 | 0.0265/0.0008 | geki__sf6__old__new__1.tsv |
| geki | sf6 | 2 | new P1 | 476/31 | 3656/1954 | 1.0705/1.0122 | 6.2869/6.0660 | 0.0334/0.0010 | geki__sf6__new__old__2.tsv |
| geki | sf6 | 2 | new P2 | 477/31 | 3674/1965 | 1.0626/1.0122 | 6.2792/5.9909 | 0.0282/0.0016 | geki__sf6__old__new__2.tsv |
| geki | sf6 | 3 | new P1 | 477/38 | 3605/2187 | 1.0813/1.0129 | 6.4728/6.0082 | 0.0382/0.0005 | geki__sf6__new__old__3.tsv |
| geki | sf6 | 3 | new P2 | 475/31 | 3538/2068 | 1.0853/1.0188 | 6.4834/6.0354 | 0.0338/0.0000 | geki__sf6__old__new__3.tsv |
| geki | sf6 | 4 | new P1 | 469/47 | 3564/2385 | 1.0761/1.0259 | 6.5000/5.9763 | 0.0238/0.0000 | geki__sf6__new__old__4.tsv |
| geki | sf6 | 4 | new P2 | 473/43 | 3536/2368 | 1.0878/1.0202 | 6.5660/5.8821 | 0.0289/0.0000 | geki__sf6__old__new__4.tsv |
| geki | sf6 | 2/1 | higher P1 | 371/209 | 3751/3401 | 1.0891/1.0611 | 6.2893/5.9481 | 0.0471/0.0429 | geki__sf6__new__new__2-1.tsv |
| geki | sf6 | 1/2 | higher P2 | 358/217 | 3738/3454 | 1.0779/1.0672 | 6.1670/5.9679 | 0.0438/0.0464 | geki__sf6__new__new__1-2.tsv |
| geki | sf6 | 3/2 | higher P1 | 396/183 | 3888/3004 | 1.0674/1.0832 | 6.2914/6.4712 | 0.0261/0.0461 | geki__sf6__new__new__3-2.tsv |
| geki | sf6 | 2/3 | higher P2 | 393/189 | 3871/2963 | 1.0915/1.1054 | 6.4374/6.5511 | 0.0341/0.0570 | geki__sf6__new__new__2-3.tsv |
| geki | sf6 | 4/3 | higher P1 | 394/186 | 3880/2939 | 1.0909/1.1260 | 6.4536/6.7795 | 0.0370/0.0553 | geki__sf6__new__new__4-3.tsv |
| geki | sf6 | 3/4 | higher P2 | 411/170 | 3921/2938 | 1.0899/1.1086 | 6.4995/6.7581 | 0.0310/0.0485 | geki__sf6__new__new__3-4.tsv |
| joe | sf6 | 4 | old vs old | 296/297 | 3489/3437 | 1.1551/1.1546 | 6.4445/6.5667 | 0.0023/0.0020 | joe__sf6__old__old__4.tsv |
| joe | sf6 | 1 | new P1 | 449/105 | 3352/1868 | 1.0493/1.0321 | 7.4653/8.4513 | 0.0332/0.0016 | joe__sf6__new__old__1.tsv |
| joe | sf6 | 1 | new P2 | 443/114 | 3282/1906 | 1.0480/1.0392 | 7.5582/8.4020 | 0.0324/0.0021 | joe__sf6__old__new__1.tsv |
| joe | sf6 | 2 | new P1 | 476/26 | 2982/1043 | 1.0319/1.0413 | 7.9403/8.4730 | 0.0100/0.0057 | joe__sf6__new__old__2.tsv |
| joe | sf6 | 2 | new P2 | 474/46 | 3052/1222 | 1.0396/1.0572 | 8.0231/8.3642 | 0.0131/0.0016 | joe__sf6__old__new__2.tsv |
| joe | sf6 | 3 | new P1 | 473/39 | 2938/1284 | 1.0401/1.0351 | 8.0221/7.8689 | 0.0180/0.0031 | joe__sf6__new__old__3.tsv |
| joe | sf6 | 3 | new P2 | 475/39 | 2939/1402 | 1.0331/1.0534 | 7.9908/7.7781 | 0.0152/0.0007 | joe__sf6__old__new__3.tsv |
| joe | sf6 | 4 | new P1 | 479/27 | 2893/1439 | 1.0218/1.0423 | 7.6959/7.2082 | 0.0093/0.0056 | joe__sf6__new__old__4.tsv |
| joe | sf6 | 4 | new P2 | 477/29 | 2803/1476 | 1.0288/1.0621 | 7.7782/6.9593 | 0.0070/0.0087 | joe__sf6__old__new__4.tsv |
| joe | sf6 | 2/1 | higher P1 | 445/99 | 3001/1798 | 1.0414/1.0414 | 8.1642/8.3946 | 0.0156/0.0315 | joe__sf6__new__new__2-1.tsv |
| joe | sf6 | 1/2 | higher P2 | 432/140 | 3037/1953 | 1.0331/1.0451 | 8.1911/8.4444 | 0.0163/0.0378 | joe__sf6__new__new__1-2.tsv |
| joe | sf6 | 3/2 | higher P1 | 471/58 | 2809/1364 | 1.0213/1.0452 | 8.7831/8.6783 | 0.0102/0.0211 | joe__sf6__new__new__3-2.tsv |
| joe | sf6 | 2/3 | higher P2 | 468/55 | 2766/1336 | 1.0166/1.0463 | 8.7972/8.8337 | 0.0065/0.0201 | joe__sf6__new__new__2-3.tsv |
| joe | sf6 | 4/3 | higher P1 | 466/78 | 2884/1550 | 1.0148/1.0219 | 8.6899/8.8081 | 0.0058/0.0084 | joe__sf6__new__new__4-3.tsv |
| joe | sf6 | 3/4 | higher P2 | 460/78 | 2862/1557 | 1.0209/1.0257 | 8.6543/8.8311 | 0.0059/0.0103 | joe__sf6__new__new__3-4.tsv |
| mike | sf6 | 4 | old vs old | 308/283 | 2938/2774 | 1.0594/1.0426 | 8.2694/8.4451 | 0.0031/0.0025 | mike__sf6__old__old__4.tsv |
| mike | sf6 | 1 | new P1 | 476/34 | 2832/1204 | 1.0141/1.0116 | 8.9544/9.0873 | 0.0095/0.0008 | mike__sf6__new__old__1.tsv |
| mike | sf6 | 1 | new P2 | 458/72 | 2862/1552 | 1.0135/1.0160 | 8.9268/8.9485 | 0.0097/0.0026 | mike__sf6__old__new__1.tsv |
| mike | sf6 | 2 | new P1 | 477/30 | 2697/1071 | 1.0176/1.0093 | 9.3124/9.8390 | 0.0052/0.0000 | mike__sf6__new__old__2.tsv |
| mike | sf6 | 2 | new P2 | 456/74 | 2787/1477 | 1.0261/1.0046 | 9.2227/9.5469 | 0.0046/0.0006 | mike__sf6__old__new__2.tsv |
| mike | sf6 | 3 | new P1 | 469/36 | 2640/1093 | 1.0196/1.0064 | 9.4303/9.6386 | 0.0064/0.0000 | mike__sf6__new__old__3.tsv |
| mike | sf6 | 3 | new P2 | 471/52 | 2742/1345 | 1.0157/1.0162 | 9.3735/9.4744 | 0.0029/0.0000 | mike__sf6__old__new__3.tsv |
| mike | sf6 | 4 | new P1 | 478/5 | 2550/822 | 1.0153/1.0072 | 9.5623/8.3216 | 0.0070/0.0012 | mike__sf6__new__old__4.tsv |
| mike | sf6 | 4 | new P2 | 480/18 | 2650/1053 | 1.0165/1.0085 | 9.4217/8.6487 | 0.0049/0.0000 | mike__sf6__old__new__4.tsv |
| mike | sf6 | 2/1 | higher P1 | 429/120 | 2731/1909 | 1.0263/1.0052 | 9.3430/9.0282 | 0.0073/0.0042 | mike__sf6__new__new__2-1.tsv |
| mike | sf6 | 1/2 | higher P2 | 406/167 | 2774/2191 | 1.0236/1.0096 | 9.2605/8.8330 | 0.0058/0.0073 | mike__sf6__new__new__1-2.tsv |
| mike | sf6 | 3/2 | higher P1 | 381/208 | 2900/2397 | 1.0258/1.0312 | 9.1524/9.1162 | 0.0010/0.0054 | mike__sf6__new__new__3-2.tsv |
| mike | sf6 | 2/3 | higher P2 | 348/234 | 2788/2423 | 1.0294/1.0275 | 9.2050/9.1343 | 0.0014/0.0042 | mike__sf6__new__new__2-3.tsv |
| mike | sf6 | 4/3 | higher P1 | 384/201 | 3042/2481 | 1.0336/1.0351 | 8.9129/8.7348 | 0.0020/0.0024 | mike__sf6__new__new__4-3.tsv |
| mike | sf6 | 3/4 | higher P2 | 334/237 | 2898/2643 | 1.0315/1.0348 | 8.8071/8.7194 | 0.0014/0.0026 | mike__sf6__new__new__3-4.tsv |
| lee | sf6 | 4 | old vs old | 305/293 | 2828/2766 | 1.0172/1.0199 | 8.2428/8.2761 | 0.0007/0.0004 | lee__sf6__old__old__4.tsv |
| lee | sf6 | 1 | new P1 | 420/158 | 3108/2017 | 1.0292/1.0089 | 8.0311/8.8256 | 0.0263/0.0020 | lee__sf6__new__old__1.tsv |
| lee | sf6 | 1 | new P2 | 416/157 | 2968/2030 | 1.0382/1.0108 | 8.3670/8.8366 | 0.0345/0.0024 | lee__sf6__old__new__1.tsv |
| lee | sf6 | 2 | new P1 | 433/132 | 3151/1966 | 1.0477/1.0137 | 8.0125/8.6261 | 0.0135/0.0010 | lee__sf6__new__old__2.tsv |
| lee | sf6 | 2 | new P2 | 444/123 | 3093/1969 | 1.0386/1.0106 | 8.1171/8.4785 | 0.0136/0.0010 | lee__sf6__old__new__2.tsv |
| lee | sf6 | 3 | new P1 | 400/165 | 3124/2252 | 1.0552/1.0066 | 7.7893/8.2617 | 0.0153/0.0009 | lee__sf6__new__old__3.tsv |
| lee | sf6 | 3 | new P2 | 396/199 | 3189/2484 | 1.0708/1.0081 | 7.8575/8.1750 | 0.0106/0.0008 | lee__sf6__old__new__3.tsv |
| lee | sf6 | 4 | new P1 | 472/75 | 3282/1731 | 1.0448/1.0029 | 7.6705/8.0484 | 0.0096/0.0006 | lee__sf6__new__old__4.tsv |
| lee | sf6 | 4 | new P2 | 458/87 | 3208/1843 | 1.0373/1.0059 | 7.6628/7.9042 | 0.0065/0.0000 | lee__sf6__old__new__4.tsv |
| lee | sf6 | 2/1 | higher P1 | 364/227 | 2987/2536 | 1.0658/1.0421 | 8.1993/8.2856 | 0.0295/0.0409 | lee__sf6__new__new__2-1.tsv |
| lee | sf6 | 1/2 | higher P2 | 346/258 | 2931/2741 | 1.0634/1.0385 | 8.2408/8.1073 | 0.0283/0.0377 | lee__sf6__new__new__1-2.tsv |
| lee | sf6 | 3/2 | higher P1 | 344/252 | 3053/2643 | 1.0542/1.0665 | 7.8947/8.4270 | 0.0218/0.0493 | lee__sf6__new__new__3-2.tsv |
| lee | sf6 | 2/3 | higher P2 | 341/262 | 3112/2703 | 1.0528/1.0733 | 7.9280/8.4050 | 0.0250/0.0471 | lee__sf6__new__new__2-3.tsv |
| lee | sf6 | 4/3 | higher P1 | 376/198 | 3236/2627 | 1.0400/1.0579 | 7.8455/7.8104 | 0.0095/0.0303 | lee__sf6__new__new__4-3.tsv |
| lee | sf6 | 3/4 | higher P2 | 359/214 | 3148/2727 | 1.0505/1.0569 | 7.8926/7.8997 | 0.0196/0.0214 | lee__sf6__new__new__3-4.tsv |
| gen | sf6 | 4 | old vs old | 338/263 | 3213/2860 | 1.0863/1.0818 | 6.7888/6.8891 | 0.0663/0.0664 | gen__sf6__old__old__4.tsv |
| gen | sf6 | 1 | new P1 | 443/108 | 3557/2176 | 1.0181/1.0051 | 6.3762/6.6326 | 0.0134/0.0014 | gen__sf6__new__old__1.tsv |
| gen | sf6 | 1 | new P2 | 432/126 | 3541/2281 | 1.0177/1.0044 | 6.3725/6.5908 | 0.0151/0.0004 | gen__sf6__old__new__1.tsv |
| gen | sf6 | 2 | new P1 | 468/37 | 3211/1442 | 1.0272/1.0339 | 6.6682/7.1604 | 0.0118/0.0277 | gen__sf6__new__old__2.tsv |
| gen | sf6 | 2 | new P2 | 466/45 | 3255/1565 | 1.0286/1.0312 | 6.6524/7.0923 | 0.0150/0.0280 | gen__sf6__old__new__2.tsv |
| gen | sf6 | 3 | new P1 | 460/56 | 3190/1494 | 1.0279/1.0352 | 6.7736/6.8958 | 0.0128/0.0299 | gen__sf6__new__old__3.tsv |
| gen | sf6 | 3 | new P2 | 470/44 | 3171/1480 | 1.0287/1.0389 | 6.8134/7.0119 | 0.0183/0.0349 | gen__sf6__old__new__3.tsv |
| gen | sf6 | 4 | new P1 | 475/28 | 3159/1392 | 1.0262/1.0574 | 6.7965/6.9094 | 0.0142/0.0480 | gen__sf6__new__old__4.tsv |
| gen | sf6 | 4 | new P2 | 467/37 | 3073/1412 | 1.0235/1.0466 | 6.9810/6.7715 | 0.0143/0.0387 | gen__sf6__old__new__4.tsv |
| gen | sf6 | 2/1 | higher P1 | 413/153 | 3433/2725 | 1.0293/1.0149 | 6.6070/6.3456 | 0.0082/0.0142 | gen__sf6__new__new__2-1.tsv |
| gen | sf6 | 1/2 | higher P2 | 397/160 | 3303/2715 | 1.0392/1.0089 | 6.7031/6.3232 | 0.0103/0.0085 | gen__sf6__new__new__1-2.tsv |
| gen | sf6 | 3/2 | higher P1 | 309/257 | 3197/2973 | 1.0380/1.0343 | 6.7048/6.4721 | 0.0209/0.0080 | gen__sf6__new__new__3-2.tsv |
| gen | sf6 | 2/3 | higher P2 | 301/278 | 3250/3175 | 1.0419/1.0307 | 6.6469/6.5297 | 0.0141/0.0118 | gen__sf6__new__new__2-3.tsv |
| gen | sf6 | 4/3 | higher P1 | 353/226 | 3424/3118 | 1.0492/1.0389 | 6.6645/6.4783 | 0.0249/0.0181 | gen__sf6__new__new__4-3.tsv |
| gen | sf6 | 3/4 | higher P2 | 335/249 | 3297/3037 | 1.0406/1.0437 | 6.7185/6.7258 | 0.0236/0.0187 | gen__sf6__new__new__3-4.tsv |
| birdie | sf6 | 4 | old vs old | 314/274 | 2822/2723 | 1.0554/1.0498 | 8.5850/8.4557 | 0.0028/0.0015 | birdie__sf6__old__old__4.tsv |
| birdie | sf6 | 1 | new P1 | 434/123 | 3446/2290 | 1.0606/1.0462 | 7.5406/7.8108 | 0.0502/0.0022 | birdie__sf6__new__old__1.tsv |
| birdie | sf6 | 1 | new P2 | 413/144 | 3489/2332 | 1.0500/1.0492 | 7.3779/7.7305 | 0.0417/0.0025 | birdie__sf6__old__new__1.tsv |
| birdie | sf6 | 2 | new P1 | 452/85 | 3250/1949 | 1.0444/1.0425 | 7.9232/7.7828 | 0.0239/0.0010 | birdie__sf6__new__old__2.tsv |
| birdie | sf6 | 2 | new P2 | 442/120 | 3342/2121 | 1.0437/1.0409 | 7.8681/7.9478 | 0.0203/0.0018 | birdie__sf6__old__new__2.tsv |
| birdie | sf6 | 3 | new P1 | 436/111 | 3353/2329 | 1.0372/1.0402 | 7.6927/7.6446 | 0.0170/0.0029 | birdie__sf6__new__old__3.tsv |
| birdie | sf6 | 3 | new P2 | 423/120 | 3227/2128 | 1.0429/1.0405 | 7.7684/7.6143 | 0.0225/0.0014 | birdie__sf6__old__new__3.tsv |
| birdie | sf6 | 4 | new P1 | 438/115 | 3273/2311 | 1.0286/1.0388 | 7.9247/7.6074 | 0.0116/0.0009 | birdie__sf6__new__old__4.tsv |
| birdie | sf6 | 4 | new P2 | 440/110 | 3163/2261 | 1.0379/1.0397 | 8.0618/7.6712 | 0.0191/0.0009 | birdie__sf6__old__new__4.tsv |
| birdie | sf6 | 2/1 | higher P1 | 429/127 | 3358/2351 | 1.0477/1.0743 | 7.7504/7.6305 | 0.0233/0.0635 | birdie__sf6__new__new__2-1.tsv |
| birdie | sf6 | 1/2 | higher P2 | 410/150 | 3330/2510 | 1.0582/1.0622 | 7.7869/7.5368 | 0.0311/0.0549 | birdie__sf6__new__new__1-2.tsv |
| birdie | sf6 | 3/2 | higher P1 | 365/201 | 3368/2568 | 1.0463/1.0739 | 7.5461/7.9455 | 0.0265/0.0451 | birdie__sf6__new__new__3-2.tsv |
| birdie | sf6 | 2/3 | higher P2 | 364/203 | 3359/2614 | 1.0436/1.0694 | 7.6444/7.9347 | 0.0237/0.0443 | birdie__sf6__new__new__2-3.tsv |
| birdie | sf6 | 4/3 | higher P1 | 406/153 | 3368/2606 | 1.0429/1.0480 | 7.7348/7.4546 | 0.0218/0.0271 | birdie__sf6__new__new__4-3.tsv |
| birdie | sf6 | 3/4 | higher P2 | 413/142 | 3419/2521 | 1.0371/1.0459 | 7.6975/7.4905 | 0.0199/0.0304 | birdie__sf6__new__new__3-4.tsv |
| eagle | sf6 | 4 | old vs old | 282/316 | 2911/2982 | 1.1088/1.1004 | 7.7941/7.8540 | 0.0082/0.0081 | eagle__sf6__old__old__4.tsv |
| eagle | sf6 | 1 | new P1 | 427/119 | 3442/1907 | 1.0312/1.0647 | 7.0862/8.2800 | 0.0190/0.0026 | eagle__sf6__new__old__1.tsv |
| eagle | sf6 | 1 | new P2 | 433/122 | 3428/1947 | 1.0332/1.0708 | 7.1814/8.2496 | 0.0190/0.0026 | eagle__sf6__old__new__1.tsv |
| eagle | sf6 | 2 | new P1 | 457/60 | 3074/1435 | 1.0635/1.0617 | 7.7984/8.2728 | 0.0150/0.0062 | eagle__sf6__new__old__2.tsv |
| eagle | sf6 | 2 | new P2 | 459/61 | 3107/1573 | 1.0667/1.0602 | 7.7236/8.2303 | 0.0135/0.0038 | eagle__sf6__old__new__2.tsv |
| eagle | sf6 | 3 | new P1 | 446/100 | 3676/1866 | 1.0633/1.0629 | 6.6998/8.0256 | 0.0144/0.0032 | eagle__sf6__new__old__3.tsv |
| eagle | sf6 | 3 | new P2 | 431/94 | 3560/1767 | 1.0672/1.0739 | 6.7167/8.0235 | 0.0188/0.0061 | eagle__sf6__old__new__3.tsv |
| eagle | sf6 | 4 | new P1 | 456/84 | 4025/1803 | 1.0429/1.0883 | 6.0944/8.1048 | 0.0160/0.0121 | eagle__sf6__new__old__4.tsv |
| eagle | sf6 | 4 | new P2 | 425/99 | 3888/1773 | 1.0510/1.0866 | 6.1134/7.9115 | 0.0118/0.0113 | eagle__sf6__old__new__4.tsv |
| eagle | sf6 | 2/1 | higher P1 | 434/121 | 3290/2395 | 1.0724/1.0326 | 7.5898/7.2323 | 0.0237/0.0147 | eagle__sf6__new__new__2-1.tsv |
| eagle | sf6 | 1/2 | higher P2 | 420/147 | 3319/2623 | 1.0770/1.0347 | 7.5948/7.3666 | 0.0279/0.0194 | eagle__sf6__new__new__1-2.tsv |
| eagle | sf6 | 3/2 | higher P1 | 302/273 | 3426/2879 | 1.0762/1.0744 | 6.9317/7.7168 | 0.0318/0.0228 | eagle__sf6__new__new__3-2.tsv |
| eagle | sf6 | 2/3 | higher P2 | 330/254 | 3582/2962 | 1.0680/1.0649 | 6.8833/7.6530 | 0.0291/0.0283 | eagle__sf6__new__new__2-3.tsv |
| eagle | sf6 | 4/3 | higher P1 | 352/214 | 3648/3054 | 1.0598/1.0691 | 6.7029/6.9190 | 0.0309/0.0386 | eagle__sf6__new__new__4-3.tsv |
| eagle | sf6 | 3/4 | higher P2 | 349/225 | 3705/3130 | 1.0614/1.0740 | 6.6868/6.9843 | 0.0361/0.0451 | eagle__sf6__new__new__3-4.tsv |
| adon | sf6 | 4 | old vs old | 290/311 | 3500/3447 | 1.0347/1.0400 | 6.4366/6.5874 | 0.0000/0.0003 | adon__sf6__old__old__4.tsv |
| adon | sf6 | 1 | new P1 | 444/116 | 3109/1877 | 1.0231/1.0011 | 7.7495/8.2434 | 0.0221/0.0011 | adon__sf6__new__old__1.tsv |
| adon | sf6 | 1 | new P2 | 445/107 | 3069/1883 | 1.0189/1.0000 | 7.7447/8.3098 | 0.0176/0.0000 | adon__sf6__old__new__1.tsv |
| adon | sf6 | 2 | new P1 | 466/72 | 3032/1563 | 1.0285/1.0026 | 7.8292/8.3150 | 0.0196/0.0000 | adon__sf6__new__old__2.tsv |
| adon | sf6 | 2 | new P2 | 463/64 | 2949/1408 | 1.0287/1.0049 | 7.8688/8.2735 | 0.0208/0.0007 | adon__sf6__old__new__2.tsv |
| adon | sf6 | 3 | new P1 | 470/55 | 3044/1556 | 1.0340/1.0000 | 7.6777/7.5415 | 0.0185/0.0000 | adon__sf6__new__old__3.tsv |
| adon | sf6 | 3 | new P2 | 468/64 | 3069/1675 | 1.0382/1.0012 | 7.6923/7.5261 | 0.0225/0.0012 | adon__sf6__old__new__3.tsv |
| adon | sf6 | 4 | new P1 | 465/64 | 3122/2422 | 1.0319/1.0037 | 7.5312/5.9914 | 0.0184/0.0000 | adon__sf6__new__old__4.tsv |
| adon | sf6 | 4 | new P2 | 468/78 | 3172/2561 | 1.0326/1.0039 | 7.5375/6.0741 | 0.0170/0.0015 | adon__sf6__old__new__4.tsv |
| adon | sf6 | 2/1 | higher P1 | 409/170 | 3010/2303 | 1.0362/1.0130 | 8.0602/8.2637 | 0.0236/0.0126 | adon__sf6__new__new__2-1.tsv |
| adon | sf6 | 1/2 | higher P2 | 389/207 | 3025/2461 | 1.0297/1.0177 | 8.0900/8.2667 | 0.0231/0.0177 | adon__sf6__new__new__1-2.tsv |
| adon | sf6 | 3/2 | higher P1 | 353/232 | 3057/2496 | 1.0222/1.0219 | 7.6906/8.5493 | 0.0136/0.0159 | adon__sf6__new__new__3-2.tsv |
| adon | sf6 | 2/3 | higher P2 | 318/290 | 3093/2717 | 1.0249/1.0215 | 7.7110/8.3661 | 0.0162/0.0182 | adon__sf6__new__new__2-3.tsv |
| adon | sf6 | 4/3 | higher P1 | 386/188 | 3145/2521 | 1.0359/1.0290 | 7.6745/7.9521 | 0.0145/0.0139 | adon__sf6__new__new__4-3.tsv |
| adon | sf6 | 3/4 | higher P2 | 382/210 | 3236/2665 | 1.0372/1.0363 | 7.6712/7.9042 | 0.0145/0.0180 | adon__sf6__new__new__3-4.tsv |
| sagat | sf6 | 4 | old vs old | 295/292 | 4054/4022 | 1.1677/1.1725 | 6.0374/5.9856 | 0.0149/0.0119 | sagat__sf6__old__old__4.tsv |
| sagat | sf6 | 1 | new P1 | 454/72 | 3395/2124 | 1.0152/1.0389 | 7.5292/6.8834 | 0.0138/0.0005 | sagat__sf6__new__old__1.tsv |
| sagat | sf6 | 1 | new P2 | 455/93 | 3465/2341 | 1.0136/1.0443 | 7.5755/6.9591 | 0.0118/0.0009 | sagat__sf6__old__new__1.tsv |
| sagat | sf6 | 2 | new P1 | 470/32 | 3079/1800 | 1.0650/1.0652 | 8.0814/6.8631 | 0.0244/0.0006 | sagat__sf6__new__old__2.tsv |
| sagat | sf6 | 2 | new P2 | 473/40 | 3170/1925 | 1.0691/1.0656 | 8.0367/6.8786 | 0.0222/0.0025 | sagat__sf6__old__new__2.tsv |
| sagat | sf6 | 3 | new P1 | 480/16 | 3111/1931 | 1.0467/1.0726 | 8.0026/6.2397 | 0.0213/0.0021 | sagat__sf6__new__old__3.tsv |
| sagat | sf6 | 3 | new P2 | 477/28 | 3115/1989 | 1.0561/1.0724 | 8.0696/6.2632 | 0.0197/0.0035 | sagat__sf6__old__new__3.tsv |
| sagat | sf6 | 4 | new P1 | 477/30 | 3079/2217 | 1.0578/1.0901 | 8.0844/5.9688 | 0.0202/0.0054 | sagat__sf6__new__old__4.tsv |
| sagat | sf6 | 4 | new P2 | 478/28 | 3084/2176 | 1.0486/1.0940 | 8.1349/5.9860 | 0.0219/0.0045 | sagat__sf6__old__new__4.tsv |
| sagat | sf6 | 2/1 | higher P1 | 452/101 | 3324/2412 | 1.0706/1.0290 | 8.0207/7.5375 | 0.0338/0.0239 | sagat__sf6__new__new__2-1.tsv |
| sagat | sf6 | 1/2 | higher P2 | 450/101 | 3316/2412 | 1.0719/1.0268 | 7.9351/7.6148 | 0.0325/0.0206 | sagat__sf6__new__new__1-2.tsv |
| sagat | sf6 | 3/2 | higher P1 | 430/159 | 3592/2842 | 1.0567/1.0904 | 7.9387/7.8451 | 0.0249/0.0434 | sagat__sf6__new__new__3-2.tsv |
| sagat | sf6 | 2/3 | higher P2 | 387/196 | 3421/2908 | 1.0530/1.1096 | 8.0010/7.8573 | 0.0220/0.0477 | sagat__sf6__new__new__2-3.tsv |
| sagat | sf6 | 4/3 | higher P1 | 427/164 | 3574/2940 | 1.0645/1.0757 | 8.0512/7.8362 | 0.0301/0.0327 | sagat__sf6__new__new__4-3.tsv |
| sagat | sf6 | 3/4 | higher P2 | 423/161 | 3550/2870 | 1.0594/1.0871 | 8.0273/7.8688 | 0.0275/0.0412 | sagat__sf6__new__new__3-4.tsv |


## Every engine stop and every unfinished match

An engine stop and an unfinished row are different outcomes. A stop may list multiple addresses, so address occurrences can exceed stopped-match counts. Untranslated-branch stops have the existing tracker [#270](https://github.com/johnstreetfighter/sf1-port/issues/270); an address occurrence does not establish its cause. Unfinished rows do not prove a native hang without a separate bounded diagnosis.

### Final

- `ryu__sf6__new__new__1-2.tsv`: `# seed 177 failed: untranslated branches at frame 4018: [103eb8]`
- `retsu__sf6__new__new__1-2.tsv`: `# seed 193 failed: untranslated branches at frame 3765: [1229db]`
- `geki__sf6__new__new__2-1.tsv`: `# seed 86 failed: untranslated branches at frame 6628: [1328c9]`
- `geki__sf6__new__new__3-2.tsv`: `# seed 102 failed: untranslated branches at frame 5064: [1328c9]`
- `geki__sf6__new__new__3-4.tsv`: `# seed 100 failed: untranslated branches at frame 3088: [1328c9]`
- `joe__sf6__new__old__3.tsv`: `# seed 227 failed: untranslated branches at frame 3644: [142577]`
- `joe__sf6__old__new__3.tsv`: `# seed 68 failed: untranslated branches at frame 4515: [142577]`
- `mike__sf6__old__new__2.tsv`: `# seed 24 failed: untranslated branches at frame 5685: [152bd0]`
- `mike__sf6__old__new__2.tsv`: `# seed 112 failed: untranslated branches at frame 1711: [152bd0]`
- `mike__sf6__new__new__2-1.tsv`: `# seed 89 failed: untranslated branches at frame 1639: [152bd0]`
- `mike__sf6__new__new__2-1.tsv`: `# seed 129 failed: untranslated branches at frame 2296: [152bd0]`
- `mike__sf6__new__new__1-2.tsv`: `# seed 233 failed: untranslated branches at frame 1394: [152bd0]`
- `mike__sf6__new__new__3-4.tsv`: `# seed 132 failed: untranslated branches at frame 5595: [152bd0]`
- `gen__sf6__old__old__4.tsv`: `# seed 98 failed: untranslated branches at frame 2059: [17302c]`
- `gen__sf6__old__old__4.tsv`: `# seed 143 failed: untranslated branches at frame 4368: [1725cd]`
- `gen__sf6__new__old__1.tsv`: `# seed 49 failed: untranslated branches at frame 5326: [17302c]`
- `gen__sf6__new__old__1.tsv`: `# seed 82 failed: untranslated branches at frame 4683: [1725cd]`
- `gen__sf6__new__old__1.tsv`: `# seed 104 failed: untranslated branches at frame 7108: [17302c]`
- `gen__sf6__new__old__1.tsv`: `# seed 116 failed: untranslated branches at frame 1766: [17302c]`
- `gen__sf6__new__old__1.tsv`: `# seed 208 failed: untranslated branches at frame 4604: [17302c]`
- `gen__sf6__new__old__1.tsv`: `# seed 224 failed: untranslated branches at frame 3661: [17302c]`
- `gen__sf6__old__new__1.tsv`: `# seed 85 failed: untranslated branches at frame 3235: [17302c]`
- `gen__sf6__old__new__1.tsv`: `# seed 130 failed: untranslated branches at frame 2090: [17302c]`
- `gen__sf6__old__new__1.tsv`: `# seed 174 failed: untranslated branches at frame 5691: [17302c]`
- `gen__sf6__new__old__2.tsv`: `# seed 26 failed: untranslated branches at frame 4764: [17302c]`
- `gen__sf6__new__old__2.tsv`: `# seed 81 failed: untranslated branches at frame 2519: [17302c]`
- `gen__sf6__new__old__2.tsv`: `# seed 188 failed: untranslated branches at frame 3181: [17302c]`
- `gen__sf6__old__new__2.tsv`: `# seed 7 failed: untranslated branches at frame 4275: [17302c]`
- `gen__sf6__old__new__2.tsv`: `# seed 106 failed: untranslated branches at frame 1515: [17302c]`
- `gen__sf6__old__new__2.tsv`: `# seed 172 failed: untranslated branches at frame 1928: [17302c]`
- `gen__sf6__old__new__2.tsv`: `# seed 212 failed: untranslated branches at frame 1545: [17302c]`
- `gen__sf6__new__old__3.tsv`: `# seed 152 failed: untranslated branches at frame 5433: [174361]`
- `gen__sf6__old__new__3.tsv`: `# seed 119 failed: untranslated branches at frame 2638: [17302c]`
- `gen__sf6__new__old__4.tsv`: `# seed 10 failed: untranslated branches at frame 3466: [17302c]`
- `gen__sf6__old__new__4.tsv`: `# seed 51 failed: untranslated branches at frame 1838: [17302c]`
- `gen__sf6__old__new__4.tsv`: `# seed 130 failed: untranslated branches at frame 3663: [17302c]`
- `gen__sf6__old__new__4.tsv`: `# seed 145 failed: untranslated branches at frame 4179: [17302c]`
- `gen__sf6__old__new__4.tsv`: `# seed 174 failed: untranslated branches at frame 2048: [17302c]`
- `gen__sf6__old__new__4.tsv`: `# seed 209 failed: untranslated branches at frame 4393: [1725cd]`
- `gen__sf6__new__new__2-1.tsv`: `# seed 25 failed: untranslated branches at frame 4799: [17302c]`
- `gen__sf6__new__new__2-1.tsv`: `# seed 119 failed: untranslated branches at frame 4008: [17302c]`
- `gen__sf6__new__new__2-1.tsv`: `# seed 175 failed: untranslated branches at frame 3831: [17302c]`
- `gen__sf6__new__new__2-1.tsv`: `# seed 194 failed: untranslated branches at frame 1825: [1725cd]`
- `gen__sf6__new__new__2-1.tsv`: `# seed 226 failed: untranslated branches at frame 5639: [17246f]`
- `gen__sf6__new__new__1-2.tsv`: `# seed 1 failed: untranslated branches at frame 1667: [1730e1]`
- `gen__sf6__new__new__1-2.tsv`: `# seed 21 failed: untranslated branches at frame 4184: [172467]`
- `gen__sf6__new__new__1-2.tsv`: `# seed 153 failed: untranslated branches at frame 3998: [17302c]`
- `gen__sf6__new__new__1-2.tsv`: `# seed 159 failed: untranslated branches at frame 2143: [17302c]`
- `gen__sf6__new__new__1-2.tsv`: `# seed 186 failed: untranslated branches at frame 5069: [172467]`
- `gen__sf6__new__new__1-2.tsv`: `# seed 196 failed: untranslated branches at frame 1496: [1725cd]`
- `gen__sf6__new__new__1-2.tsv`: `# seed 226 failed: untranslated branches at frame 5493: [172467]`
- `gen__sf6__new__new__3-2.tsv`: `# seed 4 failed: untranslated branches at frame 4106: [17302c]`
- `gen__sf6__new__new__3-2.tsv`: `# seed 16 failed: untranslated branches at frame 6033: [172467]`
- `gen__sf6__new__new__3-2.tsv`: `# seed 34 failed: untranslated branches at frame 4786: [174361]`
- `gen__sf6__new__new__3-2.tsv`: `# seed 51 failed: untranslated branches at frame 3973: [17302c]`
- `gen__sf6__new__new__3-2.tsv`: `# seed 95 failed: untranslated branches at frame 2021: [1725cd]`
- `gen__sf6__new__new__3-2.tsv`: `# seed 97 failed: untranslated branches at frame 3927: [17302c]`
- `gen__sf6__new__new__3-2.tsv`: `# seed 144 failed: untranslated branches at frame 3554: [17302c]`
- `gen__sf6__new__new__3-2.tsv`: `# seed 182 failed: untranslated branches at frame 5810: [172467]`
- `gen__sf6__new__new__3-2.tsv`: `# seed 200 failed: untranslated branches at frame 2807: [17302c]`
- `gen__sf6__new__new__2-3.tsv`: `# seed 3 failed: untranslated branches at frame 3958: [17302c]`
- `gen__sf6__new__new__2-3.tsv`: `# seed 64 failed: untranslated branches at frame 4645: [17302c]`
- `gen__sf6__new__new__2-3.tsv`: `# seed 66 failed: untranslated branches at frame 5256: [172467]`
- `gen__sf6__new__new__2-3.tsv`: `# seed 123 failed: untranslated branches at frame 6118: [174361]`
- `gen__sf6__new__new__2-3.tsv`: `# seed 140 failed: untranslated branches at frame 2187: [17302c]`
- `gen__sf6__new__new__2-3.tsv`: `# seed 158 failed: untranslated branches at frame 5884: [17302c]`
- `gen__sf6__new__new__2-3.tsv`: `# seed 184 failed: untranslated branches at frame 2410: [17302c]`
- `gen__sf6__new__new__2-3.tsv`: `# seed 186 failed: untranslated branches at frame 1929: [17302c]`
- `gen__sf6__new__new__2-3.tsv`: `# seed 203 failed: untranslated branches at frame 3946: [17302c]`
- `gen__sf6__new__new__4-3.tsv`: `# seed 127 failed: untranslated branches at frame 3801: [1725cd]`
- `gen__sf6__new__new__4-3.tsv`: `# seed 160 failed: untranslated branches at frame 3524: [1730e1]`
- `gen__sf6__new__new__4-3.tsv`: `# seed 178 failed: untranslated branches at frame 3934: [172467]`
- `gen__sf6__new__new__3-4.tsv`: `# seed 12 failed: untranslated branches at frame 5523: [172467]`
- `gen__sf6__new__new__3-4.tsv`: `# seed 117 failed: untranslated branches at frame 1669: [1725cd]`
- `gen__sf6__new__new__3-4.tsv`: `# seed 121 failed: untranslated branches at frame 4046: [174361]`
- `birdie__sf6__old__old__4.tsv`: `# seed 78 failed: untranslated branches at frame 5297: [1823c7]`
- `birdie__sf6__new__old__2.tsv`: `# seed 157 failed: untranslated branches at frame 4047: [1823c7]`
- `birdie__sf6__new__old__2.tsv`: `# seed 179 failed: untranslated branches at frame 4253: [1823cf]`
- `birdie__sf6__old__new__2.tsv`: `# seed 17 failed: untranslated branches at frame 5357: [1823c7]`
- `birdie__sf6__new__old__3.tsv`: `# seed 158 failed: untranslated branches at frame 4868: [1823c7]`
- `birdie__sf6__new__old__3.tsv`: `# seed 196 failed: untranslated branches at frame 4666: [1823c7]`
- `birdie__sf6__new__old__3.tsv`: `# seed 214 failed: untranslated branches at frame 3629: [1823c7]`
- `birdie__sf6__old__new__3.tsv`: `# seed 4 failed: untranslated branches at frame 3808: [1823c7]`
- `birdie__sf6__old__new__3.tsv`: `# seed 162 failed: untranslated branches at frame 4563: [1823c7]`
- `birdie__sf6__old__new__3.tsv`: `# seed 174 failed: untranslated branches at frame 3454: [1823c7]`
- `birdie__sf6__old__new__3.tsv`: `# seed 186 failed: untranslated branches at frame 4980: [1823c7]`
- `birdie__sf6__old__new__3.tsv`: `# seed 189 failed: untranslated branches at frame 3641: [1823c7]`
- `birdie__sf6__old__new__4.tsv`: `# seed 215 failed: untranslated branches at frame 3520: [1823c7]`
- `birdie__sf6__old__new__4.tsv`: `# seed 234 failed: untranslated branches at frame 4744: [1823c7]`
- `birdie__sf6__new__new__2-1.tsv`: `# seed 31 failed: untranslated branches at frame 4506: [1823c7]`
- `birdie__sf6__new__new__2-1.tsv`: `# seed 49 failed: untranslated branches at frame 5542: [1823c7]`
- `birdie__sf6__new__new__2-1.tsv`: `# seed 81 failed: untranslated branches at frame 5363: [1823c7]`
- `birdie__sf6__new__new__2-1.tsv`: `# seed 120 failed: untranslated branches at frame 5725: [1823c7]`
- `birdie__sf6__new__new__2-1.tsv`: `# seed 147 failed: untranslated branches at frame 5637: [1823c7]`
- `birdie__sf6__new__new__1-2.tsv`: `# seed 12 failed: untranslated branches at frame 5208: [1823c7]`
- `birdie__sf6__new__new__1-2.tsv`: `# seed 37 failed: untranslated branches at frame 4646: [1823c7]`
- `birdie__sf6__new__new__1-2.tsv`: `# seed 41 failed: untranslated branches at frame 5569: [1823c7]`
- `birdie__sf6__new__new__1-2.tsv`: `# seed 128 failed: untranslated branches at frame 5309: [1823c7]`
- `birdie__sf6__new__new__1-2.tsv`: `# seed 135 failed: untranslated branches at frame 3862: [1823c7]`
- `birdie__sf6__new__new__1-2.tsv`: `# seed 163 failed: untranslated branches at frame 3797: [1823c7]`
- `birdie__sf6__new__new__1-2.tsv`: `# seed 181 failed: untranslated branches at frame 4216: [1823c7]`
- `birdie__sf6__new__new__1-2.tsv`: `# seed 188 failed: untranslated branches at frame 4781: [1823c7]`
- `birdie__sf6__new__new__3-2.tsv`: `# seed 17 failed: untranslated branches at frame 4756: [1823c7]`
- `birdie__sf6__new__new__3-2.tsv`: `# seed 84 failed: untranslated branches at frame 3965: [1823c7]`
- `birdie__sf6__new__new__3-2.tsv`: `# seed 104 failed: untranslated branches at frame 3844: [1823c7]`
- `birdie__sf6__new__new__3-2.tsv`: `# seed 122 failed: untranslated branches at frame 5028: [1823c7]`
- `birdie__sf6__new__new__3-2.tsv`: `# seed 193 failed: untranslated branches at frame 4647: [1823c7]`
- `birdie__sf6__new__new__3-2.tsv`: `# seed 197 failed: untranslated branches at frame 4740: [1823c7]`
- `birdie__sf6__new__new__2-3.tsv`: `# seed 78 failed: untranslated branches at frame 4324: [1823c7]`
- `birdie__sf6__new__new__2-3.tsv`: `# seed 90 failed: untranslated branches at frame 5063: [1823c7]`
- `birdie__sf6__new__new__2-3.tsv`: `# seed 167 failed: untranslated branches at frame 4798: [1823c7]`
- `birdie__sf6__new__new__2-3.tsv`: `# seed 168 failed: untranslated branches at frame 4014: [1823c7]`
- `birdie__sf6__new__new__2-3.tsv`: `# seed 176 failed: untranslated branches at frame 5422: [1823c7]`
- `birdie__sf6__new__new__2-3.tsv`: `# seed 239 failed: untranslated branches at frame 4571: [1823c7]`
- `birdie__sf6__new__new__4-3.tsv`: `# seed 1 failed: untranslated branches at frame 4771: [1823c7]`
- `birdie__sf6__new__new__4-3.tsv`: `# seed 24 failed: untranslated branches at frame 3977: [1823c7]`
- `birdie__sf6__new__new__4-3.tsv`: `# seed 70 failed: untranslated branches at frame 3584: [1823c7]`
- `birdie__sf6__new__new__4-3.tsv`: `# seed 83 failed: untranslated branches at frame 3064: [1823c7]`
- `birdie__sf6__new__new__4-3.tsv`: `# seed 141 failed: untranslated branches at frame 3799: [1823c7]`
- `birdie__sf6__new__new__3-4.tsv`: `# seed 27 failed: untranslated branches at frame 3347: [1823c7]`
- `birdie__sf6__new__new__3-4.tsv`: `# seed 32 failed: untranslated branches at frame 3385: [1823c7]`
- `birdie__sf6__new__new__3-4.tsv`: `# seed 39 failed: untranslated branches at frame 4752: [1823c7]`
- `birdie__sf6__new__new__3-4.tsv`: `# seed 109 failed: untranslated branches at frame 3898: [1823c7]`
- `birdie__sf6__new__new__3-4.tsv`: `# seed 207 failed: untranslated branches at frame 4901: [1823c7]`
- `birdie__sf6__new__new__3-4.tsv`: `# seed 215 failed: untranslated branches at frame 3148: [1823c7]`
- `eagle__sf6__old__old__4.tsv`: `# seed 44 failed: untranslated branches at frame 5434: [192b55]`
- `eagle__sf6__old__old__4.tsv`: `# seed 61 failed: untranslated branches at frame 1469: [193220]`
- `eagle__sf6__old__old__4.tsv`: `# seed 146 failed: untranslated branches at frame 5625: [192b55]`
- `eagle__sf6__old__old__4.tsv`: `# seed 186 failed: untranslated branches at frame 4057: [192b55]`
- `eagle__sf6__old__old__4.tsv`: `# seed 194 failed: untranslated branches at frame 4341: [1942f3]`
- `eagle__sf6__new__old__1.tsv`: `# seed 79 failed: untranslated branches at frame 5395: [193220]`
- `eagle__sf6__new__old__1.tsv`: `# seed 110 failed: untranslated branches at frame 1798: [192b5d]`
- `eagle__sf6__new__old__1.tsv`: `# seed 187 failed: untranslated branches at frame 6354: [193220]`
- `eagle__sf6__new__old__1.tsv`: `# seed 206 failed: untranslated branches at frame 1677: [193220]`
- `eagle__sf6__new__old__1.tsv`: `# seed 224 failed: untranslated branches at frame 1399: [193220]`
- `eagle__sf6__old__new__1.tsv`: `# seed 16 failed: untranslated branches at frame 4471: [193220]`
- `eagle__sf6__old__new__1.tsv`: `# seed 197 failed: untranslated branches at frame 1479: [193220]`
- `eagle__sf6__new__old__2.tsv`: `# seed 11 failed: untranslated branches at frame 2124: [193220]`
- `eagle__sf6__new__old__2.tsv`: `# seed 87 failed: untranslated branches at frame 6374: [192b55]`
- `eagle__sf6__new__old__2.tsv`: `# seed 156 failed: untranslated branches at frame 2501: [193220]`
- `eagle__sf6__new__old__2.tsv`: `# seed 162 failed: untranslated branches at frame 4169: [193220]`
- `eagle__sf6__new__old__2.tsv`: `# seed 197 failed: untranslated branches at frame 5396: [192b55]`
- `eagle__sf6__old__new__2.tsv`: `# seed 44 failed: untranslated branches at frame 1815: [193220]`
- `eagle__sf6__old__new__2.tsv`: `# seed 54 failed: untranslated branches at frame 3324: [1942f3]`
- `eagle__sf6__old__new__2.tsv`: `# seed 97 failed: untranslated branches at frame 1533: [193220]`
- `eagle__sf6__old__new__2.tsv`: `# seed 128 failed: untranslated branches at frame 2074: [193220]`
- `eagle__sf6__old__new__2.tsv`: `# seed 213 failed: untranslated branches at frame 3319: [1942f3]`
- `eagle__sf6__new__old__3.tsv`: `# seed 39 failed: untranslated branches at frame 3782: [193220]`
- `eagle__sf6__new__old__3.tsv`: `# seed 55 failed: untranslated branches at frame 3864: [193220]`
- `eagle__sf6__new__old__3.tsv`: `# seed 120 failed: untranslated branches at frame 3384: [192b55]`
- `eagle__sf6__new__old__3.tsv`: `# seed 131 failed: untranslated branches at frame 2622: [193220]`
- `eagle__sf6__new__old__3.tsv`: `# seed 149 failed: untranslated branches at frame 3837: [193220]`
- `eagle__sf6__new__old__3.tsv`: `# seed 190 failed: untranslated branches at frame 3769: [192b55]`
- `eagle__sf6__new__old__3.tsv`: `# seed 216 failed: untranslated branches at frame 4140: [192b55]`
- `eagle__sf6__old__new__3.tsv`: `# seed 38 failed: untranslated branches at frame 3573: [193220]`
- `eagle__sf6__old__new__3.tsv`: `# seed 90 failed: untranslated branches at frame 3633: [192b55]`
- `eagle__sf6__old__new__3.tsv`: `# seed 109 failed: untranslated branches at frame 2318: [193220]`
- `eagle__sf6__old__new__3.tsv`: `# seed 135 failed: untranslated branches at frame 1900: [193220]`
- `eagle__sf6__old__new__3.tsv`: `# seed 136 failed: untranslated branches at frame 3393: [193220]`
- `eagle__sf6__old__new__3.tsv`: `# seed 176 failed: untranslated branches at frame 4539: [192b55]`
- `eagle__sf6__old__new__3.tsv`: `# seed 186 failed: untranslated branches at frame 2334: [1942f3]`
- `eagle__sf6__old__new__3.tsv`: `# seed 236 failed: untranslated branches at frame 3766: [192b55]`
- `eagle__sf6__new__old__4.tsv`: `# seed 126 failed: untranslated branches at frame 3260: [192b55]`
- `eagle__sf6__old__new__4.tsv`: `# seed 37 failed: untranslated branches at frame 3823: [1942f3]`
- `eagle__sf6__old__new__4.tsv`: `# seed 45 failed: untranslated branches at frame 3810: [193220]`
- `eagle__sf6__old__new__4.tsv`: `# seed 48 failed: untranslated branches at frame 3614: [192b55]`
- `eagle__sf6__old__new__4.tsv`: `# seed 143 failed: untranslated branches at frame 3234: [193220]`
- `eagle__sf6__old__new__4.tsv`: `# seed 190 failed: untranslated branches at frame 1425: [1942f3]`
- `eagle__sf6__old__new__4.tsv`: `# seed 208 failed: untranslated branches at frame 4944: [192b55]`
- `eagle__sf6__old__new__4.tsv`: `# seed 209 failed: untranslated branches at frame 2121: [1942f3]`
- `eagle__sf6__old__new__4.tsv`: `# seed 237 failed: untranslated branches at frame 1842: [193220]`
- `eagle__sf6__new__new__2-1.tsv`: `# seed 38 failed: untranslated branches at frame 3940: [192b55]`
- `eagle__sf6__new__new__2-1.tsv`: `# seed 57 failed: untranslated branches at frame 2855: [193220]`
- `eagle__sf6__new__new__2-1.tsv`: `# seed 65 failed: untranslated branches at frame 5850: [192b55]`
- `eagle__sf6__new__new__2-1.tsv`: `# seed 209 failed: untranslated branches at frame 5569: [193220]`
- `eagle__sf6__new__new__1-2.tsv`: `# seed 16 failed: untranslated branches at frame 3097: [193220]`
- `eagle__sf6__new__new__1-2.tsv`: `# seed 22 failed: untranslated branches at frame 1845: [192b65]`
- `eagle__sf6__new__new__1-2.tsv`: `# seed 128 failed: untranslated branches at frame 3541: [193220]`
- `eagle__sf6__new__new__1-2.tsv`: `# seed 150 failed: untranslated branches at frame 2079: [193220]`
- `eagle__sf6__new__new__3-2.tsv`: `# seed 17 failed: untranslated branches at frame 3457: [192b55]`
- `eagle__sf6__new__new__3-2.tsv`: `# seed 22 failed: untranslated branches at frame 1450: [193220]`
- `eagle__sf6__new__new__3-2.tsv`: `# seed 35 failed: untranslated branches at frame 4141: [192b55]`
- `eagle__sf6__new__new__3-2.tsv`: `# seed 57 failed: untranslated branches at frame 5434: [192b55]`
- `eagle__sf6__new__new__3-2.tsv`: `# seed 91 failed: untranslated branches at frame 3723: [1942f3]`
- `eagle__sf6__new__new__3-2.tsv`: `# seed 159 failed: untranslated branches at frame 5056: [193220]`
- `eagle__sf6__new__new__3-2.tsv`: `# seed 169 failed: untranslated branches at frame 3871: [193220]`
- `eagle__sf6__new__new__3-2.tsv`: `# seed 201 failed: untranslated branches at frame 5076: [192b55]`
- `eagle__sf6__new__new__2-3.tsv`: `# seed 5 failed: untranslated branches at frame 1915: [193220]`
- `eagle__sf6__new__new__2-3.tsv`: `# seed 40 failed: untranslated branches at frame 5666: [192b55]`
- `eagle__sf6__new__new__2-3.tsv`: `# seed 83 failed: untranslated branches at frame 5475: [192b55]`
- `eagle__sf6__new__new__2-3.tsv`: `# seed 152 failed: untranslated branches at frame 5316: [193220]`
- `eagle__sf6__new__new__2-3.tsv`: `# seed 167 failed: untranslated branches at frame 6234: [1942f3]`
- `eagle__sf6__new__new__2-3.tsv`: `# seed 175 failed: untranslated branches at frame 6065: [193220]`
- `eagle__sf6__new__new__2-3.tsv`: `# seed 217 failed: untranslated branches at frame 3421: [192b65]`
- `eagle__sf6__new__new__2-3.tsv`: `# seed 231 failed: untranslated branches at frame 5551: [192b55]`
- `eagle__sf6__new__new__4-3.tsv`: `# seed 21 failed: untranslated branches at frame 6090: [1942f3]`
- `eagle__sf6__new__new__4-3.tsv`: `# seed 22 failed: untranslated branches at frame 4630: [192b55]`
- `eagle__sf6__new__new__4-3.tsv`: `# seed 23 failed: untranslated branches at frame 4375: [192b55]`
- `eagle__sf6__new__new__4-3.tsv`: `# seed 82 failed: untranslated branches at frame 3495: [192b55]`
- `eagle__sf6__new__new__4-3.tsv`: `# seed 115 failed: untranslated branches at frame 3849: [192b55]`
- `eagle__sf6__new__new__4-3.tsv`: `# seed 141 failed: untranslated branches at frame 4149: [1942f3]`
- `eagle__sf6__new__new__4-3.tsv`: `# seed 147 failed: untranslated branches at frame 1800: [1942f3]`
- `eagle__sf6__new__new__4-3.tsv`: `# seed 210 failed: untranslated branches at frame 6069: [192b65]`
- `eagle__sf6__new__new__4-3.tsv`: `# seed 233 failed: untranslated branches at frame 4997: [1942f3]`
- `eagle__sf6__new__new__4-3.tsv`: `# seed 238 failed: untranslated branches at frame 5926: [192b55]`
- `eagle__sf6__new__new__3-4.tsv`: `# seed 21 failed: untranslated branches at frame 6844: [1942f3]`
- `eagle__sf6__new__new__3-4.tsv`: `# seed 70 failed: untranslated branches at frame 5363: [192b55]`
- `eagle__sf6__new__new__3-4.tsv`: `# seed 82 failed: untranslated branches at frame 3509: [192b55]`
- `eagle__sf6__new__new__3-4.tsv`: `# seed 100 failed: untranslated branches at frame 1608: [193220]`
- `eagle__sf6__new__new__3-4.tsv`: `# seed 154 failed: untranslated branches at frame 4812: [1942f3]`
- `eagle__sf6__new__new__3-4.tsv`: `# seed 184 failed: untranslated branches at frame 4095: [192b55]`
- `eagle__sf6__new__new__3-4.tsv`: `# seed 189 failed: untranslated branches at frame 4448: [192b55]`
- `eagle__sf6__new__new__3-4.tsv`: `# seed 195 failed: untranslated branches at frame 5374: [1942f3]`
- `adon__sf6__old__old__4.tsv`: `# seed 84 failed: untranslated branches at frame 5021: [1a25c7]`
- `adon__sf6__old__old__4.tsv`: `# seed 119 failed: untranslated branches at frame 4605: [1a25c7]`
- `adon__sf6__new__new__3-2.tsv`: `# seed 124 failed: untranslated branches at frame 3987: [1a25cf]`
- `adon__sf6__new__new__3-4.tsv`: `# seed 127 failed: untranslated branches at frame 4904: [1a25cf]`
- `sagat__sf6__new__old__1.tsv`: `# seed 86 failed: untranslated branches at frame 1714: [1b8365]`

No unfinished rows.

### Baseline controls

- `ken__sf5ce-skill1trigger1__old__new__4.tsv`: `# seed 40 failed: untranslated branches at frame 1975: [112455]`
- `ken__sf5ce-skill1trigger2__old__new__4.tsv`: `# seed 40 failed: untranslated branches at frame 1975: [112455]`
- `retsu__sf5ce-skill1trigger1__new__old__4.tsv`: `# seed 4 failed: untranslated branches at frame 6350: [124057]`
- `retsu__sf5ce-skill1trigger1__new__old__4.tsv`: `# seed 15 failed: untranslated branches at frame 5006: [124057]`
- `retsu__sf5ce-skill1trigger1__new__old__4.tsv`: `# seed 17 failed: untranslated branches at frame 6883: [124057]`
- `retsu__sf5ce-skill1trigger1__new__old__4.tsv`: `# seed 34 failed: untranslated branches at frame 1668: [124057]`
- `retsu__sf5ce-skill1trigger1__new__old__4.tsv`: `# seed 39 failed: untranslated branches at frame 4375: [124057]`
- `retsu__sf5ce-skill1trigger1__old__new__4.tsv`: `# seed 6 failed: untranslated branches at frame 1415: [122d35]`
- `retsu__sf5ce-skill1trigger1__old__new__4.tsv`: `# seed 7 failed: untranslated branches at frame 5086: [124057]`
- `retsu__sf5ce-skill1trigger1__old__new__4.tsv`: `# seed 12 failed: untranslated branches at frame 5014: [124057]`
- `retsu__sf5ce-skill1trigger1__old__new__4.tsv`: `# seed 28 failed: untranslated branches at frame 3769: [124057]`
- `retsu__sf5ce-skill1trigger1__old__new__4.tsv`: `# seed 33 failed: untranslated branches at frame 4520: [124057]`
- `retsu__sf5ce-skill1trigger1__old__new__4.tsv`: `# seed 36 failed: untranslated branches at frame 3761: [124057]`
- `retsu__sf5ce-skill1trigger2__new__old__4.tsv`: `# seed 4 failed: untranslated branches at frame 6350: [124057]`
- `retsu__sf5ce-skill1trigger2__new__old__4.tsv`: `# seed 15 failed: untranslated branches at frame 5006: [124057]`
- `retsu__sf5ce-skill1trigger2__new__old__4.tsv`: `# seed 17 failed: untranslated branches at frame 6883: [124057]`
- `retsu__sf5ce-skill1trigger2__new__old__4.tsv`: `# seed 34 failed: untranslated branches at frame 1668: [124057]`
- `retsu__sf5ce-skill1trigger2__new__old__4.tsv`: `# seed 39 failed: untranslated branches at frame 4375: [124057]`
- `retsu__sf5ce-skill1trigger2__old__new__4.tsv`: `# seed 6 failed: untranslated branches at frame 1415: [122d35]`
- `retsu__sf5ce-skill1trigger2__old__new__4.tsv`: `# seed 7 failed: untranslated branches at frame 5086: [124057]`
- `retsu__sf5ce-skill1trigger2__old__new__4.tsv`: `# seed 12 failed: untranslated branches at frame 5014: [124057]`
- `retsu__sf5ce-skill1trigger2__old__new__4.tsv`: `# seed 28 failed: untranslated branches at frame 3769: [124057]`
- `retsu__sf5ce-skill1trigger2__old__new__4.tsv`: `# seed 33 failed: untranslated branches at frame 4520: [124057]`
- `retsu__sf5ce-skill1trigger2__old__new__4.tsv`: `# seed 36 failed: untranslated branches at frame 3761: [124057]`
- `retsu__sf5ce-skill2trigger1__old__new__4.tsv`: `# seed 6 failed: untranslated branches at frame 1415: [122d35]`
- `retsu__sf5ce-skill2trigger2__old__new__4.tsv`: `# seed 6 failed: untranslated branches at frame 1415: [122d35]`
- `joe__sf5ce-skill1trigger1__new__old__4.tsv`: `# seed 22 failed: untranslated branches at frame 7995: [143889]`
- `joe__sf5ce-skill1trigger1__new__old__4.tsv`: `# seed 23 failed: untranslated branches at frame 3673: [143889]`
- `joe__sf5ce-skill1trigger1__new__old__4.tsv`: `# seed 26 failed: untranslated branches at frame 4674: [143889]`
- `joe__sf5ce-skill1trigger1__new__old__4.tsv`: `# seed 34 failed: untranslated branches at frame 2927: [143889]`
- `joe__sf5ce-skill1trigger1__old__new__4.tsv`: `# seed 6 failed: untranslated branches at frame 5013: [143889]`
- `joe__sf5ce-skill1trigger1__old__new__4.tsv`: `# seed 25 failed: untranslated branches at frame 6790: [143889]`
- `joe__sf5ce-skill1trigger2__new__old__4.tsv`: `# seed 22 failed: untranslated branches at frame 7995: [143889]`
- `joe__sf5ce-skill1trigger2__new__old__4.tsv`: `# seed 23 failed: untranslated branches at frame 3673: [143889]`
- `joe__sf5ce-skill1trigger2__new__old__4.tsv`: `# seed 26 failed: untranslated branches at frame 4674: [143889]`
- `joe__sf5ce-skill1trigger2__new__old__4.tsv`: `# seed 34 failed: untranslated branches at frame 2927: [143889]`
- `joe__sf5ce-skill1trigger2__old__new__4.tsv`: `# seed 6 failed: untranslated branches at frame 5013: [143889]`
- `joe__sf5ce-skill1trigger2__old__new__4.tsv`: `# seed 25 failed: untranslated branches at frame 6790: [143889]`
- `mike__sf5ce-skill2trigger1__old__new__4.tsv`: `# seed 9 failed: untranslated branches at frame 2955: [1539f3]`
- `mike__sf5ce-skill2trigger2__old__new__4.tsv`: `# seed 9 failed: untranslated branches at frame 2955: [1539f3]`
- `eagle__sf5ce-skill1trigger1__new__old__4.tsv`: `# seed 6 failed: untranslated branches at frame 4744: [193ef3]`
- `eagle__sf5ce-skill1trigger1__new__old__4.tsv`: `# seed 7 failed: untranslated branches at frame 4161: [193ef3]`
- `eagle__sf5ce-skill1trigger1__new__old__4.tsv`: `# seed 8 failed: untranslated branches at frame 6166: [193220]`
- `eagle__sf5ce-skill1trigger1__new__old__4.tsv`: `# seed 17 failed: untranslated branches at frame 3711: [193ef3]`
- `eagle__sf5ce-skill1trigger1__new__old__4.tsv`: `# seed 25 failed: untranslated branches at frame 4155: [193ef3]`
- `eagle__sf5ce-skill1trigger1__new__old__4.tsv`: `# seed 26 failed: untranslated branches at frame 1991: [193ef3]`
- `eagle__sf5ce-skill1trigger1__new__old__4.tsv`: `# seed 39 failed: untranslated branches at frame 4425: [193ef3]`
- `eagle__sf5ce-skill1trigger1__old__new__4.tsv`: `# seed 6 failed: untranslated branches at frame 1680: [193ef3]`
- `eagle__sf5ce-skill1trigger1__old__new__4.tsv`: `# seed 26 failed: untranslated branches at frame 4569: [193ef3]`
- `eagle__sf5ce-skill1trigger2__new__old__4.tsv`: `# seed 6 failed: untranslated branches at frame 4744: [193ef3]`
- `eagle__sf5ce-skill1trigger2__new__old__4.tsv`: `# seed 7 failed: untranslated branches at frame 4161: [193ef3]`
- `eagle__sf5ce-skill1trigger2__new__old__4.tsv`: `# seed 8 failed: untranslated branches at frame 6166: [193220]`
- `eagle__sf5ce-skill1trigger2__new__old__4.tsv`: `# seed 17 failed: untranslated branches at frame 3711: [193ef3]`
- `eagle__sf5ce-skill1trigger2__new__old__4.tsv`: `# seed 25 failed: untranslated branches at frame 4155: [193ef3]`
- `eagle__sf5ce-skill1trigger2__new__old__4.tsv`: `# seed 26 failed: untranslated branches at frame 1991: [193ef3]`
- `eagle__sf5ce-skill1trigger2__new__old__4.tsv`: `# seed 39 failed: untranslated branches at frame 4425: [193ef3]`
- `eagle__sf5ce-skill1trigger2__old__new__4.tsv`: `# seed 6 failed: untranslated branches at frame 1680: [193ef3]`
- `eagle__sf5ce-skill1trigger2__old__new__4.tsv`: `# seed 26 failed: untranslated branches at frame 4569: [193ef3]`
- `eagle__sf5ce-skill2trigger1__new__old__4.tsv`: `# seed 7 failed: untranslated branches at frame 4226: [193efb]`
- `eagle__sf5ce-skill2trigger1__old__new__4.tsv`: `# seed 6 failed: untranslated branches at frame 5769: [193efb]`
- `eagle__sf5ce-skill2trigger1__old__new__4.tsv`: `# seed 9 failed: untranslated branches at frame 8427: [193220]`
- `eagle__sf5ce-skill2trigger1__old__new__4.tsv`: `# seed 36 failed: untranslated branches at frame 4837: [193efb]`
- `eagle__sf5ce-skill2trigger2__new__old__4.tsv`: `# seed 7 failed: untranslated branches at frame 4226: [193efb]`
- `eagle__sf5ce-skill2trigger2__old__new__4.tsv`: `# seed 6 failed: untranslated branches at frame 5769: [193efb]`
- `eagle__sf5ce-skill2trigger2__old__new__4.tsv`: `# seed 9 failed: untranslated branches at frame 8427: [193220]`
- `adon__sf5ce-skill1trigger1__new__old__4.tsv`: `# seed 32 failed: untranslated branches at frame 4232: [1a37df]`
- `adon__sf5ce-skill1trigger2__new__old__4.tsv`: `# seed 32 failed: untranslated branches at frame 4232: [1a37df]`
- `retsu__usf4-ultra1__old__new__4.tsv`: `# seed 7 failed: untranslated branches at frame 1745: [122d35]`
- `retsu__usf4-ultra1__old__new__4.tsv`: `# seed 28 failed: untranslated branches at frame 1583: [122d35]`
- `retsu__usf4-ultra1__old__new__4.tsv`: `# seed 35 failed: untranslated branches at frame 3726: [122d45]`
- `retsu__usf4-ultra2__old__new__4.tsv`: `# seed 7 failed: untranslated branches at frame 1745: [122d35]`
- `retsu__usf4-ultra2__old__new__4.tsv`: `# seed 28 failed: untranslated branches at frame 1583: [122d35]`
- `retsu__usf4-ultradouble__old__new__4.tsv`: `# seed 7 failed: untranslated branches at frame 1745: [122d35]`
- `retsu__usf4-ultradouble__old__new__4.tsv`: `# seed 28 failed: untranslated branches at frame 1583: [122d35]`
- `joe__usf4-ultra1__new__old__4.tsv`: `# seed 17 failed: untranslated branches at frame 3890: [143708]`
- `joe__usf4-ultra1__old__new__4.tsv`: `# seed 18 failed: untranslated branches at frame 2291: [143708]`
- `gen__sf4-ultra1__new__old__4.tsv`: `# seed 4 failed: untranslated branches at frame 5081: [1736c8]`
- `gen__sf4-ultra1__new__old__4.tsv`: `# seed 35 failed: untranslated branches at frame 2906: [1736c8]`
- `gen__sf4-ultra1__old__new__4.tsv`: `# seed 18 failed: untranslated branches at frame 3593: [1736c8]`
- `gen__sf4-ultra1__old__new__4.tsv`: `# seed 27 failed: untranslated branches at frame 2352: [172f15]`
- `gen__ssf4-ultra1__new__old__4.tsv`: `# seed 4 failed: untranslated branches at frame 5081: [1736c8]`
- `gen__ssf4-ultra1__new__old__4.tsv`: `# seed 35 failed: untranslated branches at frame 2906: [1736c8]`
- `gen__ssf4-ultra1__old__new__4.tsv`: `# seed 18 failed: untranslated branches at frame 3593: [1736c8]`
- `gen__ssf4-ultra1__old__new__4.tsv`: `# seed 27 failed: untranslated branches at frame 2352: [172f15]`
- `gen__ssf4-ultra2__new__old__4.tsv`: `# seed 35 failed: untranslated branches at frame 2906: [1736c8]`
- `gen__ssf4-ultra2__old__new__4.tsv`: `# seed 27 failed: untranslated branches at frame 2226: [1736c8]`
- `gen__ae2012-ultra1__new__old__4.tsv`: `# seed 4 failed: untranslated branches at frame 5081: [1736c8]`
- `gen__ae2012-ultra1__new__old__4.tsv`: `# seed 35 failed: untranslated branches at frame 2906: [1736c8]`
- `gen__ae2012-ultra1__old__new__4.tsv`: `# seed 18 failed: untranslated branches at frame 3593: [1736c8]`
- `gen__ae2012-ultra1__old__new__4.tsv`: `# seed 27 failed: untranslated branches at frame 2352: [172f15]`
- `gen__ae2012-ultra2__new__old__4.tsv`: `# seed 35 failed: untranslated branches at frame 2906: [1736c8]`
- `gen__ae2012-ultra2__old__new__4.tsv`: `# seed 27 failed: untranslated branches at frame 2226: [1736c8]`
- `gen__usf4-ultra1__new__old__4.tsv`: `# seed 4 failed: untranslated branches at frame 5081: [1736c8]`
- `gen__usf4-ultra1__new__old__4.tsv`: `# seed 35 failed: untranslated branches at frame 2906: [1736c8]`
- `gen__usf4-ultra1__old__new__4.tsv`: `# seed 18 failed: untranslated branches at frame 3593: [1736c8]`
- `gen__usf4-ultra2__new__old__4.tsv`: `# seed 35 failed: untranslated branches at frame 2906: [1736c8]`
- `gen__usf4-ultra2__old__new__4.tsv`: `# seed 27 failed: untranslated branches at frame 2226: [1736c8]`
- `gen__usf4-ultradouble__new__old__4.tsv`: `# seed 35 failed: untranslated branches at frame 2906: [1736c8]`
- `gen__usf4-ultradouble__old__new__4.tsv`: `# seed 40 failed: untranslated branches at frame 3547: [1736c8]`
- `birdie__usf4-ultra1__new__old__4.tsv`: `# seed 29 failed: untranslated branches at frame 3084: [1823c7]`
- `birdie__usf4-ultra1__new__old__4.tsv`: `# seed 34 failed: untranslated branches at frame 4798: [18366b]`
- `birdie__usf4-ultra1__old__new__4.tsv`: `# seed 2 failed: untranslated branches at frame 1556: [18366b]`
- `birdie__usf4-ultra1__old__new__4.tsv`: `# seed 4 failed: untranslated branches at frame 1706: [18366b]`
- `birdie__usf4-ultra1__old__new__4.tsv`: `# seed 8 failed: untranslated branches at frame 1573: [18366b]`
- `birdie__usf4-ultra1__old__new__4.tsv`: `# seed 39 failed: untranslated branches at frame 2883: [18366b]`
- `birdie__usf4-ultra2__new__old__4.tsv`: `# seed 20 failed: untranslated branches at frame 1935: [18366b]`
- `birdie__usf4-ultra2__old__new__4.tsv`: `# seed 2 failed: untranslated branches at frame 1556: [18366b]`
- `birdie__usf4-ultra2__old__new__4.tsv`: `# seed 5 failed: untranslated branches at frame 4323: [18366b]`
- `birdie__usf4-ultra2__old__new__4.tsv`: `# seed 8 failed: untranslated branches at frame 1573: [18366b]`
- `birdie__usf4-ultra2__old__new__4.tsv`: `# seed 39 failed: untranslated branches at frame 2883: [18366b]`
- `birdie__usf4-ultradouble__new__old__4.tsv`: `# seed 19 failed: untranslated branches at frame 3352: [1823c7]`
- `birdie__usf4-ultradouble__new__old__4.tsv`: `# seed 20 failed: untranslated branches at frame 3505: [18366b]`
- `birdie__usf4-ultradouble__new__old__4.tsv`: `# seed 29 failed: untranslated branches at frame 3084: [1823c7]`
- `birdie__usf4-ultradouble__new__old__4.tsv`: `# seed 34 failed: untranslated branches at frame 4798: [18366b]`
- `birdie__usf4-ultradouble__old__new__4.tsv`: `# seed 2 failed: untranslated branches at frame 1556: [18366b]`
- `birdie__usf4-ultradouble__old__new__4.tsv`: `# seed 8 failed: untranslated branches at frame 1573: [18366b]`
- `birdie__usf4-ultradouble__old__new__4.tsv`: `# seed 39 failed: untranslated branches at frame 2883: [18366b]`
- `eagle__usf4-ultra1__new__old__4.tsv`: `# seed 3 failed: untranslated branches at frame 2138: [193d72]`
- `eagle__usf4-ultra1__new__old__4.tsv`: `# seed 22 failed: untranslated branches at frame 3999: [193d72]`
- `eagle__usf4-ultra1__new__old__4.tsv`: `# seed 30 failed: untranslated branches at frame 1475: [193d72]`
- `eagle__usf4-ultra1__new__old__4.tsv`: `# seed 35 failed: untranslated branches at frame 2249: [193d72]`
- `eagle__usf4-ultra1__new__old__4.tsv`: `# seed 37 failed: untranslated branches at frame 1791: [193d72]`
- `eagle__usf4-ultra1__old__new__4.tsv`: `# seed 11 failed: untranslated branches at frame 3872: [193d72]`
- `eagle__usf4-ultra1__old__new__4.tsv`: `# seed 16 failed: untranslated branches at frame 3715: [193d72]`
- `eagle__usf4-ultra1__old__new__4.tsv`: `# seed 22 failed: untranslated branches at frame 3643: [193d72]`
- `eagle__usf4-ultra1__old__new__4.tsv`: `# seed 27 failed: untranslated branches at frame 1508: [193d72]`
- `eagle__usf4-ultra1__old__new__4.tsv`: `# seed 36 failed: untranslated branches at frame 1872: [193d72]`
- `eagle__usf4-ultra2__new__old__4.tsv`: `# seed 4 failed: untranslated branches at frame 2069: [193d72]`
- `eagle__usf4-ultra2__new__old__4.tsv`: `# seed 15 failed: untranslated branches at frame 3841: [1930e7]`
- `eagle__usf4-ultra2__new__old__4.tsv`: `# seed 24 failed: untranslated branches at frame 3473: [193d72]`
- `eagle__usf4-ultra2__new__old__4.tsv`: `# seed 27 failed: untranslated branches at frame 6604: [1930e7]`
- `eagle__usf4-ultra2__new__old__4.tsv`: `# seed 37 failed: untranslated branches at frame 6173: [193d72]`
- `eagle__usf4-ultra2__old__new__4.tsv`: `# seed 11 failed: untranslated branches at frame 6344: [193d72]`
- `eagle__usf4-ultra2__old__new__4.tsv`: `# seed 13 failed: untranslated branches at frame 2030: [193d72]`
- `eagle__usf4-ultra2__old__new__4.tsv`: `# seed 15 failed: untranslated branches at frame 2106: [193d72]`
- `eagle__usf4-ultra2__old__new__4.tsv`: `# seed 27 failed: untranslated branches at frame 1508: [193d72]`
- `eagle__usf4-ultra2__old__new__4.tsv`: `# seed 34 failed: untranslated branches at frame 3359: [193d72]`
- `eagle__usf4-ultradouble__new__old__4.tsv`: `# seed 1 failed: untranslated branches at frame 3858: [1930e7]`
- `eagle__usf4-ultradouble__new__old__4.tsv`: `# seed 3 failed: untranslated branches at frame 2138: [193d72]`
- `eagle__usf4-ultradouble__new__old__4.tsv`: `# seed 14 failed: untranslated branches at frame 2358: [192b55]`
- `eagle__usf4-ultradouble__new__old__4.tsv`: `# seed 27 failed: untranslated branches at frame 2542: [192b65]`
- `eagle__usf4-ultradouble__new__old__4.tsv`: `# seed 30 failed: untranslated branches at frame 1475: [193d72]`
- `eagle__usf4-ultradouble__new__old__4.tsv`: `# seed 35 failed: untranslated branches at frame 2249: [193d72]`
- `eagle__usf4-ultradouble__new__old__4.tsv`: `# seed 37 failed: untranslated branches at frame 1791: [193d72]`
- `eagle__usf4-ultradouble__new__old__4.tsv`: `# seed 40 failed: untranslated branches at frame 3652: [193d72]`
- `eagle__usf4-ultradouble__old__new__4.tsv`: `# seed 2 failed: untranslated branches at frame 4204: [193d72]`
- `eagle__usf4-ultradouble__old__new__4.tsv`: `# seed 11 failed: untranslated branches at frame 1884: [192b55]`
- `eagle__usf4-ultradouble__old__new__4.tsv`: `# seed 13 failed: untranslated branches at frame 2030: [193d72]`
- `eagle__usf4-ultradouble__old__new__4.tsv`: `# seed 27 failed: untranslated branches at frame 1508: [193d72]`
- `eagle__usf4-ultradouble__old__new__4.tsv`: `# seed 36 failed: untranslated branches at frame 1872: [193d72]`
- `adon__ssf4-ultra1__new__old__4.tsv`: `# seed 4 failed: untranslated branches at frame 2175: [1a2d6e]`
- `adon__ssf4-ultra1__new__old__4.tsv`: `# seed 18 failed: untranslated branches at frame 6096: [1a25c7]`
- `adon__ssf4-ultra2__new__old__4.tsv`: `# seed 40 failed: untranslated branches at frame 4363: [1a2d6e]`
- `adon__ssf4-ultra2__old__new__4.tsv`: `# seed 8 failed: untranslated branches at frame 2155: [1a2eef]`
- `adon__ae2012-ultra1__new__old__4.tsv`: `# seed 4 failed: untranslated branches at frame 2175: [1a2d6e]`
- `adon__ae2012-ultra1__new__old__4.tsv`: `# seed 18 failed: untranslated branches at frame 6096: [1a25c7]`
- `adon__ae2012-ultra2__new__old__4.tsv`: `# seed 40 failed: untranslated branches at frame 4363: [1a2d6e]`
- `adon__ae2012-ultra2__old__new__4.tsv`: `# seed 8 failed: untranslated branches at frame 2155: [1a2eef]`
- `adon__usf4-ultra1__new__old__4.tsv`: `# seed 4 failed: untranslated branches at frame 2175: [1a2d6e]`
- `adon__usf4-ultra1__new__old__4.tsv`: `# seed 18 failed: untranslated branches at frame 6096: [1a25c7]`
- `adon__usf4-ultra2__new__old__4.tsv`: `# seed 37 failed: untranslated branches at frame 5709: [1a2d6e]`
- `adon__usf4-ultra2__new__old__4.tsv`: `# seed 40 failed: untranslated branches at frame 4363: [1a2d6e]`
- `adon__usf4-ultra2__old__new__4.tsv`: `# seed 8 failed: untranslated branches at frame 2155: [1a2eef]`
- `sagat__sf4-ultra1__new__old__4.tsv`: `# seed 10 failed: untranslated branches at frame 2019: [1b74e1]`
- `sagat__ssf4-ultra1__new__old__4.tsv`: `# seed 10 failed: untranslated branches at frame 2019: [1b74e1]`
- `sagat__ssf4-ultra2__old__new__4.tsv`: `# seed 1 failed: untranslated branches at frame 5061: [1b78c2]`
- `sagat__ae2012-ultra1__new__old__4.tsv`: `# seed 10 failed: untranslated branches at frame 2019: [1b74e1]`
- `sagat__ae2012-ultra2__old__new__4.tsv`: `# seed 1 failed: untranslated branches at frame 5061: [1b78c2]`
- `sagat__usf4-ultra1__new__old__4.tsv`: `# seed 10 failed: untranslated branches at frame 2019: [1b74e1]`
- `sagat__usf4-ultra2__new__old__4.tsv`: `# seed 31 failed: untranslated branches at frame 2940: [1b78c2]`
- `sagat__usf4-ultradouble__new__old__4.tsv`: `# seed 20 failed: untranslated branches at frame 4862: [1b78c2]`
- `sagat__usf4-ultradouble__new__old__4.tsv`: `# seed 31 failed: untranslated branches at frame 2940: [1b78c2]`
- `sagat__usf4-ultradouble__new__old__4.tsv`: `# seed 39 failed: untranslated branches at frame 4802: [1b78c2]`
- `ryu__sf3ng-art2__old__new__4.tsv`: `# seed 26 failed: untranslated branches at frame 4268: [10192f]`
- `adon__sf3threes-art3__old__new__4.tsv`: `# seed 19 failed: untranslated branches at frame 3911: [1a30c8]`
- `ryu__st__new__old__4.tsv`: `# seed 20 failed: untranslated branches at frame 3070: [10491c]`
- `ryu__st__new__old__4.tsv`: `# seed 28 failed: untranslated branches at frame 4888: [10491c]`
- `ryu__st__old__old__4.tsv`: `# seed 5 failed: untranslated branches at frame 3025: [10491c]`
- `ryu__st__old__old__4.tsv`: `# seed 7 failed: untranslated branches at frame 2988: [10491c]`
- `ryu__st__old__old__4.tsv`: `# seed 12 failed: untranslated branches at frame 4586: [10491c]`
- `ryu__st__old__old__4.tsv`: `# seed 28 failed: untranslated branches at frame 3144: [10491c]`
- `ryu__st__old__old__4.tsv`: `# seed 33 failed: untranslated branches at frame 2940: [10491c]`
- `ryu__st__old__old__4.tsv`: `# seed 34 failed: untranslated branches at frame 3108: [10491c]`
- `retsu__st__old__old__4.tsv`: `# seed 18 failed: untranslated branches at frame 2919: [122d35]`
- `gen__st__new__old__4.tsv`: `# seed 28 failed: untranslated branches at frame 1793: [17302c]`
- `gen__sfz3__new__old__4.tsv`: `# seed 3 failed: untranslated branches at frame 4939: [172f15]`
- `gen__sfz3__old__old__4.tsv`: `# seed 1 failed: untranslated branches at frame 7511: [172f15]`
- `gen__sfz3__old__old__4.tsv`: `# seed 8 failed: untranslated branches at frame 4697: [172f15]`
- `gen__sfz3__old__old__4.tsv`: `# seed 31 failed: untranslated branches at frame 6101: [172f15]`
- `eagle__st__new__old__4.tsv`: `# seed 17 failed: untranslated branches at frame 4164: [193220]`
- `eagle__st__new__old__4.tsv`: `# seed 26 failed: untranslated branches at frame 3481: [193220]`
- `eagle__sfz3__new__old__4.tsv`: `# seed 35 failed: untranslated branches at frame 1898: [192b65]`
- `eagle__sfz3__old__old__4.tsv`: `# seed 2 failed: untranslated branches at frame 3767: [192b55]`
- `eagle__sfz3__old__old__4.tsv`: `# seed 29 failed: untranslated branches at frame 4549: [192b65]`
- `eagle__sfz3__old__old__4.tsv`: `# seed 30 failed: untranslated branches at frame 1595: [192b55]`
- `eagle__sfz3__old__old__4.tsv`: `# seed 37 failed: untranslated branches at frame 4496: [192b55]`

No unfinished rows.

### Current controls

- `ken__sf5ce-skill1trigger1__old__new__4.tsv`: `# seed 40 failed: untranslated branches at frame 1975: [112455]`
- `ken__sf5ce-skill1trigger2__old__new__4.tsv`: `# seed 40 failed: untranslated branches at frame 1975: [112455]`
- `retsu__sf5ce-skill1trigger1__new__old__4.tsv`: `# seed 4 failed: untranslated branches at frame 6350: [124057]`
- `retsu__sf5ce-skill1trigger1__new__old__4.tsv`: `# seed 15 failed: untranslated branches at frame 5006: [124057]`
- `retsu__sf5ce-skill1trigger1__new__old__4.tsv`: `# seed 17 failed: untranslated branches at frame 6883: [124057]`
- `retsu__sf5ce-skill1trigger1__new__old__4.tsv`: `# seed 34 failed: untranslated branches at frame 1668: [124057]`
- `retsu__sf5ce-skill1trigger1__new__old__4.tsv`: `# seed 39 failed: untranslated branches at frame 4375: [124057]`
- `retsu__sf5ce-skill1trigger1__old__new__4.tsv`: `# seed 6 failed: untranslated branches at frame 1415: [122d35]`
- `retsu__sf5ce-skill1trigger1__old__new__4.tsv`: `# seed 7 failed: untranslated branches at frame 5086: [124057]`
- `retsu__sf5ce-skill1trigger1__old__new__4.tsv`: `# seed 12 failed: untranslated branches at frame 5014: [124057]`
- `retsu__sf5ce-skill1trigger1__old__new__4.tsv`: `# seed 28 failed: untranslated branches at frame 3769: [124057]`
- `retsu__sf5ce-skill1trigger1__old__new__4.tsv`: `# seed 33 failed: untranslated branches at frame 4520: [124057]`
- `retsu__sf5ce-skill1trigger1__old__new__4.tsv`: `# seed 36 failed: untranslated branches at frame 3761: [124057]`
- `retsu__sf5ce-skill1trigger2__new__old__4.tsv`: `# seed 4 failed: untranslated branches at frame 6350: [124057]`
- `retsu__sf5ce-skill1trigger2__new__old__4.tsv`: `# seed 15 failed: untranslated branches at frame 5006: [124057]`
- `retsu__sf5ce-skill1trigger2__new__old__4.tsv`: `# seed 17 failed: untranslated branches at frame 6883: [124057]`
- `retsu__sf5ce-skill1trigger2__new__old__4.tsv`: `# seed 34 failed: untranslated branches at frame 1668: [124057]`
- `retsu__sf5ce-skill1trigger2__new__old__4.tsv`: `# seed 39 failed: untranslated branches at frame 4375: [124057]`
- `retsu__sf5ce-skill1trigger2__old__new__4.tsv`: `# seed 6 failed: untranslated branches at frame 1415: [122d35]`
- `retsu__sf5ce-skill1trigger2__old__new__4.tsv`: `# seed 7 failed: untranslated branches at frame 5086: [124057]`
- `retsu__sf5ce-skill1trigger2__old__new__4.tsv`: `# seed 12 failed: untranslated branches at frame 5014: [124057]`
- `retsu__sf5ce-skill1trigger2__old__new__4.tsv`: `# seed 28 failed: untranslated branches at frame 3769: [124057]`
- `retsu__sf5ce-skill1trigger2__old__new__4.tsv`: `# seed 33 failed: untranslated branches at frame 4520: [124057]`
- `retsu__sf5ce-skill1trigger2__old__new__4.tsv`: `# seed 36 failed: untranslated branches at frame 3761: [124057]`
- `retsu__sf5ce-skill2trigger1__old__new__4.tsv`: `# seed 6 failed: untranslated branches at frame 1415: [122d35]`
- `retsu__sf5ce-skill2trigger2__old__new__4.tsv`: `# seed 6 failed: untranslated branches at frame 1415: [122d35]`
- `joe__sf5ce-skill1trigger1__new__old__4.tsv`: `# seed 22 failed: untranslated branches at frame 7995: [143889]`
- `joe__sf5ce-skill1trigger1__new__old__4.tsv`: `# seed 23 failed: untranslated branches at frame 3673: [143889]`
- `joe__sf5ce-skill1trigger1__new__old__4.tsv`: `# seed 26 failed: untranslated branches at frame 4674: [143889]`
- `joe__sf5ce-skill1trigger1__new__old__4.tsv`: `# seed 34 failed: untranslated branches at frame 2927: [143889]`
- `joe__sf5ce-skill1trigger1__old__new__4.tsv`: `# seed 6 failed: untranslated branches at frame 5013: [143889]`
- `joe__sf5ce-skill1trigger1__old__new__4.tsv`: `# seed 25 failed: untranslated branches at frame 6790: [143889]`
- `joe__sf5ce-skill1trigger2__new__old__4.tsv`: `# seed 22 failed: untranslated branches at frame 7995: [143889]`
- `joe__sf5ce-skill1trigger2__new__old__4.tsv`: `# seed 23 failed: untranslated branches at frame 3673: [143889]`
- `joe__sf5ce-skill1trigger2__new__old__4.tsv`: `# seed 26 failed: untranslated branches at frame 4674: [143889]`
- `joe__sf5ce-skill1trigger2__new__old__4.tsv`: `# seed 34 failed: untranslated branches at frame 2927: [143889]`
- `joe__sf5ce-skill1trigger2__old__new__4.tsv`: `# seed 6 failed: untranslated branches at frame 5013: [143889]`
- `joe__sf5ce-skill1trigger2__old__new__4.tsv`: `# seed 25 failed: untranslated branches at frame 6790: [143889]`
- `mike__sf5ce-skill2trigger1__old__new__4.tsv`: `# seed 9 failed: untranslated branches at frame 2955: [1539f3]`
- `mike__sf5ce-skill2trigger2__old__new__4.tsv`: `# seed 9 failed: untranslated branches at frame 2955: [1539f3]`
- `eagle__sf5ce-skill1trigger1__new__old__4.tsv`: `# seed 6 failed: untranslated branches at frame 4744: [193ef3]`
- `eagle__sf5ce-skill1trigger1__new__old__4.tsv`: `# seed 7 failed: untranslated branches at frame 4161: [193ef3]`
- `eagle__sf5ce-skill1trigger1__new__old__4.tsv`: `# seed 8 failed: untranslated branches at frame 6166: [193220]`
- `eagle__sf5ce-skill1trigger1__new__old__4.tsv`: `# seed 17 failed: untranslated branches at frame 3711: [193ef3]`
- `eagle__sf5ce-skill1trigger1__new__old__4.tsv`: `# seed 25 failed: untranslated branches at frame 4155: [193ef3]`
- `eagle__sf5ce-skill1trigger1__new__old__4.tsv`: `# seed 26 failed: untranslated branches at frame 1991: [193ef3]`
- `eagle__sf5ce-skill1trigger1__new__old__4.tsv`: `# seed 39 failed: untranslated branches at frame 4425: [193ef3]`
- `eagle__sf5ce-skill1trigger1__old__new__4.tsv`: `# seed 6 failed: untranslated branches at frame 1680: [193ef3]`
- `eagle__sf5ce-skill1trigger1__old__new__4.tsv`: `# seed 26 failed: untranslated branches at frame 4569: [193ef3]`
- `eagle__sf5ce-skill1trigger2__new__old__4.tsv`: `# seed 6 failed: untranslated branches at frame 4744: [193ef3]`
- `eagle__sf5ce-skill1trigger2__new__old__4.tsv`: `# seed 7 failed: untranslated branches at frame 4161: [193ef3]`
- `eagle__sf5ce-skill1trigger2__new__old__4.tsv`: `# seed 8 failed: untranslated branches at frame 6166: [193220]`
- `eagle__sf5ce-skill1trigger2__new__old__4.tsv`: `# seed 17 failed: untranslated branches at frame 3711: [193ef3]`
- `eagle__sf5ce-skill1trigger2__new__old__4.tsv`: `# seed 25 failed: untranslated branches at frame 4155: [193ef3]`
- `eagle__sf5ce-skill1trigger2__new__old__4.tsv`: `# seed 26 failed: untranslated branches at frame 1991: [193ef3]`
- `eagle__sf5ce-skill1trigger2__new__old__4.tsv`: `# seed 39 failed: untranslated branches at frame 4425: [193ef3]`
- `eagle__sf5ce-skill1trigger2__old__new__4.tsv`: `# seed 6 failed: untranslated branches at frame 1680: [193ef3]`
- `eagle__sf5ce-skill1trigger2__old__new__4.tsv`: `# seed 26 failed: untranslated branches at frame 4569: [193ef3]`
- `eagle__sf5ce-skill2trigger1__new__old__4.tsv`: `# seed 7 failed: untranslated branches at frame 4226: [193efb]`
- `eagle__sf5ce-skill2trigger1__old__new__4.tsv`: `# seed 6 failed: untranslated branches at frame 5769: [193efb]`
- `eagle__sf5ce-skill2trigger1__old__new__4.tsv`: `# seed 9 failed: untranslated branches at frame 8427: [193220]`
- `eagle__sf5ce-skill2trigger1__old__new__4.tsv`: `# seed 36 failed: untranslated branches at frame 4837: [193efb]`
- `eagle__sf5ce-skill2trigger2__new__old__4.tsv`: `# seed 7 failed: untranslated branches at frame 4226: [193efb]`
- `eagle__sf5ce-skill2trigger2__old__new__4.tsv`: `# seed 6 failed: untranslated branches at frame 5769: [193efb]`
- `eagle__sf5ce-skill2trigger2__old__new__4.tsv`: `# seed 9 failed: untranslated branches at frame 8427: [193220]`
- `adon__sf5ce-skill1trigger1__new__old__4.tsv`: `# seed 32 failed: untranslated branches at frame 4232: [1a37df]`
- `adon__sf5ce-skill1trigger2__new__old__4.tsv`: `# seed 32 failed: untranslated branches at frame 4232: [1a37df]`
- `retsu__usf4-ultra1__old__new__4.tsv`: `# seed 7 failed: untranslated branches at frame 1745: [122d35]`
- `retsu__usf4-ultra1__old__new__4.tsv`: `# seed 28 failed: untranslated branches at frame 1583: [122d35]`
- `retsu__usf4-ultra1__old__new__4.tsv`: `# seed 35 failed: untranslated branches at frame 3726: [122d45]`
- `retsu__usf4-ultra2__old__new__4.tsv`: `# seed 7 failed: untranslated branches at frame 1745: [122d35]`
- `retsu__usf4-ultra2__old__new__4.tsv`: `# seed 28 failed: untranslated branches at frame 1583: [122d35]`
- `retsu__usf4-ultradouble__old__new__4.tsv`: `# seed 7 failed: untranslated branches at frame 1745: [122d35]`
- `retsu__usf4-ultradouble__old__new__4.tsv`: `# seed 28 failed: untranslated branches at frame 1583: [122d35]`
- `joe__usf4-ultra1__new__old__4.tsv`: `# seed 17 failed: untranslated branches at frame 3890: [143708]`
- `joe__usf4-ultra1__old__new__4.tsv`: `# seed 18 failed: untranslated branches at frame 2291: [143708]`
- `gen__sf4-ultra1__new__old__4.tsv`: `# seed 4 failed: untranslated branches at frame 5081: [1736c8]`
- `gen__sf4-ultra1__new__old__4.tsv`: `# seed 35 failed: untranslated branches at frame 2906: [1736c8]`
- `gen__sf4-ultra1__old__new__4.tsv`: `# seed 18 failed: untranslated branches at frame 3593: [1736c8]`
- `gen__sf4-ultra1__old__new__4.tsv`: `# seed 27 failed: untranslated branches at frame 2352: [172f15]`
- `gen__ssf4-ultra1__new__old__4.tsv`: `# seed 4 failed: untranslated branches at frame 5081: [1736c8]`
- `gen__ssf4-ultra1__new__old__4.tsv`: `# seed 35 failed: untranslated branches at frame 2906: [1736c8]`
- `gen__ssf4-ultra1__old__new__4.tsv`: `# seed 18 failed: untranslated branches at frame 3593: [1736c8]`
- `gen__ssf4-ultra1__old__new__4.tsv`: `# seed 27 failed: untranslated branches at frame 2352: [172f15]`
- `gen__ssf4-ultra2__new__old__4.tsv`: `# seed 35 failed: untranslated branches at frame 2906: [1736c8]`
- `gen__ssf4-ultra2__old__new__4.tsv`: `# seed 27 failed: untranslated branches at frame 2226: [1736c8]`
- `gen__ae2012-ultra1__new__old__4.tsv`: `# seed 4 failed: untranslated branches at frame 5081: [1736c8]`
- `gen__ae2012-ultra1__new__old__4.tsv`: `# seed 35 failed: untranslated branches at frame 2906: [1736c8]`
- `gen__ae2012-ultra1__old__new__4.tsv`: `# seed 18 failed: untranslated branches at frame 3593: [1736c8]`
- `gen__ae2012-ultra1__old__new__4.tsv`: `# seed 27 failed: untranslated branches at frame 2352: [172f15]`
- `gen__ae2012-ultra2__new__old__4.tsv`: `# seed 35 failed: untranslated branches at frame 2906: [1736c8]`
- `gen__ae2012-ultra2__old__new__4.tsv`: `# seed 27 failed: untranslated branches at frame 2226: [1736c8]`
- `gen__usf4-ultra1__new__old__4.tsv`: `# seed 4 failed: untranslated branches at frame 5081: [1736c8]`
- `gen__usf4-ultra1__new__old__4.tsv`: `# seed 35 failed: untranslated branches at frame 2906: [1736c8]`
- `gen__usf4-ultra1__old__new__4.tsv`: `# seed 18 failed: untranslated branches at frame 3593: [1736c8]`
- `gen__usf4-ultra2__new__old__4.tsv`: `# seed 35 failed: untranslated branches at frame 2906: [1736c8]`
- `gen__usf4-ultra2__old__new__4.tsv`: `# seed 27 failed: untranslated branches at frame 2226: [1736c8]`
- `gen__usf4-ultradouble__new__old__4.tsv`: `# seed 35 failed: untranslated branches at frame 2906: [1736c8]`
- `gen__usf4-ultradouble__old__new__4.tsv`: `# seed 40 failed: untranslated branches at frame 3547: [1736c8]`
- `birdie__usf4-ultra1__new__old__4.tsv`: `# seed 29 failed: untranslated branches at frame 3084: [1823c7]`
- `birdie__usf4-ultra1__new__old__4.tsv`: `# seed 34 failed: untranslated branches at frame 4798: [18366b]`
- `birdie__usf4-ultra1__old__new__4.tsv`: `# seed 2 failed: untranslated branches at frame 1556: [18366b]`
- `birdie__usf4-ultra1__old__new__4.tsv`: `# seed 4 failed: untranslated branches at frame 1706: [18366b]`
- `birdie__usf4-ultra1__old__new__4.tsv`: `# seed 8 failed: untranslated branches at frame 1573: [18366b]`
- `birdie__usf4-ultra1__old__new__4.tsv`: `# seed 39 failed: untranslated branches at frame 2883: [18366b]`
- `birdie__usf4-ultra2__new__old__4.tsv`: `# seed 20 failed: untranslated branches at frame 1935: [18366b]`
- `birdie__usf4-ultra2__old__new__4.tsv`: `# seed 2 failed: untranslated branches at frame 1556: [18366b]`
- `birdie__usf4-ultra2__old__new__4.tsv`: `# seed 5 failed: untranslated branches at frame 4323: [18366b]`
- `birdie__usf4-ultra2__old__new__4.tsv`: `# seed 8 failed: untranslated branches at frame 1573: [18366b]`
- `birdie__usf4-ultra2__old__new__4.tsv`: `# seed 39 failed: untranslated branches at frame 2883: [18366b]`
- `birdie__usf4-ultradouble__new__old__4.tsv`: `# seed 19 failed: untranslated branches at frame 3352: [1823c7]`
- `birdie__usf4-ultradouble__new__old__4.tsv`: `# seed 20 failed: untranslated branches at frame 3505: [18366b]`
- `birdie__usf4-ultradouble__new__old__4.tsv`: `# seed 29 failed: untranslated branches at frame 3084: [1823c7]`
- `birdie__usf4-ultradouble__new__old__4.tsv`: `# seed 34 failed: untranslated branches at frame 4798: [18366b]`
- `birdie__usf4-ultradouble__old__new__4.tsv`: `# seed 2 failed: untranslated branches at frame 1556: [18366b]`
- `birdie__usf4-ultradouble__old__new__4.tsv`: `# seed 8 failed: untranslated branches at frame 1573: [18366b]`
- `birdie__usf4-ultradouble__old__new__4.tsv`: `# seed 39 failed: untranslated branches at frame 2883: [18366b]`
- `eagle__usf4-ultra1__new__old__4.tsv`: `# seed 3 failed: untranslated branches at frame 2138: [193d72]`
- `eagle__usf4-ultra1__new__old__4.tsv`: `# seed 22 failed: untranslated branches at frame 3999: [193d72]`
- `eagle__usf4-ultra1__new__old__4.tsv`: `# seed 30 failed: untranslated branches at frame 1475: [193d72]`
- `eagle__usf4-ultra1__new__old__4.tsv`: `# seed 35 failed: untranslated branches at frame 2249: [193d72]`
- `eagle__usf4-ultra1__new__old__4.tsv`: `# seed 37 failed: untranslated branches at frame 1791: [193d72]`
- `eagle__usf4-ultra1__old__new__4.tsv`: `# seed 11 failed: untranslated branches at frame 3872: [193d72]`
- `eagle__usf4-ultra1__old__new__4.tsv`: `# seed 16 failed: untranslated branches at frame 3715: [193d72]`
- `eagle__usf4-ultra1__old__new__4.tsv`: `# seed 22 failed: untranslated branches at frame 3643: [193d72]`
- `eagle__usf4-ultra1__old__new__4.tsv`: `# seed 27 failed: untranslated branches at frame 1508: [193d72]`
- `eagle__usf4-ultra1__old__new__4.tsv`: `# seed 36 failed: untranslated branches at frame 1872: [193d72]`
- `eagle__usf4-ultra2__new__old__4.tsv`: `# seed 4 failed: untranslated branches at frame 2069: [193d72]`
- `eagle__usf4-ultra2__new__old__4.tsv`: `# seed 15 failed: untranslated branches at frame 3841: [1930e7]`
- `eagle__usf4-ultra2__new__old__4.tsv`: `# seed 24 failed: untranslated branches at frame 3473: [193d72]`
- `eagle__usf4-ultra2__new__old__4.tsv`: `# seed 27 failed: untranslated branches at frame 6604: [1930e7]`
- `eagle__usf4-ultra2__new__old__4.tsv`: `# seed 37 failed: untranslated branches at frame 6173: [193d72]`
- `eagle__usf4-ultra2__old__new__4.tsv`: `# seed 11 failed: untranslated branches at frame 6344: [193d72]`
- `eagle__usf4-ultra2__old__new__4.tsv`: `# seed 13 failed: untranslated branches at frame 2030: [193d72]`
- `eagle__usf4-ultra2__old__new__4.tsv`: `# seed 15 failed: untranslated branches at frame 2106: [193d72]`
- `eagle__usf4-ultra2__old__new__4.tsv`: `# seed 27 failed: untranslated branches at frame 1508: [193d72]`
- `eagle__usf4-ultra2__old__new__4.tsv`: `# seed 34 failed: untranslated branches at frame 3359: [193d72]`
- `eagle__usf4-ultradouble__new__old__4.tsv`: `# seed 1 failed: untranslated branches at frame 3858: [1930e7]`
- `eagle__usf4-ultradouble__new__old__4.tsv`: `# seed 3 failed: untranslated branches at frame 2138: [193d72]`
- `eagle__usf4-ultradouble__new__old__4.tsv`: `# seed 14 failed: untranslated branches at frame 2358: [192b55]`
- `eagle__usf4-ultradouble__new__old__4.tsv`: `# seed 27 failed: untranslated branches at frame 2542: [192b65]`
- `eagle__usf4-ultradouble__new__old__4.tsv`: `# seed 30 failed: untranslated branches at frame 1475: [193d72]`
- `eagle__usf4-ultradouble__new__old__4.tsv`: `# seed 35 failed: untranslated branches at frame 2249: [193d72]`
- `eagle__usf4-ultradouble__new__old__4.tsv`: `# seed 37 failed: untranslated branches at frame 1791: [193d72]`
- `eagle__usf4-ultradouble__new__old__4.tsv`: `# seed 40 failed: untranslated branches at frame 3652: [193d72]`
- `eagle__usf4-ultradouble__old__new__4.tsv`: `# seed 2 failed: untranslated branches at frame 4204: [193d72]`
- `eagle__usf4-ultradouble__old__new__4.tsv`: `# seed 11 failed: untranslated branches at frame 1884: [192b55]`
- `eagle__usf4-ultradouble__old__new__4.tsv`: `# seed 13 failed: untranslated branches at frame 2030: [193d72]`
- `eagle__usf4-ultradouble__old__new__4.tsv`: `# seed 27 failed: untranslated branches at frame 1508: [193d72]`
- `eagle__usf4-ultradouble__old__new__4.tsv`: `# seed 36 failed: untranslated branches at frame 1872: [193d72]`
- `adon__ssf4-ultra1__new__old__4.tsv`: `# seed 4 failed: untranslated branches at frame 2175: [1a2d6e]`
- `adon__ssf4-ultra1__new__old__4.tsv`: `# seed 18 failed: untranslated branches at frame 6096: [1a25c7]`
- `adon__ssf4-ultra2__new__old__4.tsv`: `# seed 40 failed: untranslated branches at frame 4363: [1a2d6e]`
- `adon__ssf4-ultra2__old__new__4.tsv`: `# seed 8 failed: untranslated branches at frame 2155: [1a2eef]`
- `adon__ae2012-ultra1__new__old__4.tsv`: `# seed 4 failed: untranslated branches at frame 2175: [1a2d6e]`
- `adon__ae2012-ultra1__new__old__4.tsv`: `# seed 18 failed: untranslated branches at frame 6096: [1a25c7]`
- `adon__ae2012-ultra2__new__old__4.tsv`: `# seed 40 failed: untranslated branches at frame 4363: [1a2d6e]`
- `adon__ae2012-ultra2__old__new__4.tsv`: `# seed 8 failed: untranslated branches at frame 2155: [1a2eef]`
- `adon__usf4-ultra1__new__old__4.tsv`: `# seed 4 failed: untranslated branches at frame 2175: [1a2d6e]`
- `adon__usf4-ultra1__new__old__4.tsv`: `# seed 18 failed: untranslated branches at frame 6096: [1a25c7]`
- `adon__usf4-ultra2__new__old__4.tsv`: `# seed 37 failed: untranslated branches at frame 5709: [1a2d6e]`
- `adon__usf4-ultra2__new__old__4.tsv`: `# seed 40 failed: untranslated branches at frame 4363: [1a2d6e]`
- `adon__usf4-ultra2__old__new__4.tsv`: `# seed 8 failed: untranslated branches at frame 2155: [1a2eef]`
- `sagat__sf4-ultra1__new__old__4.tsv`: `# seed 10 failed: untranslated branches at frame 2019: [1b74e1]`
- `sagat__ssf4-ultra1__new__old__4.tsv`: `# seed 10 failed: untranslated branches at frame 2019: [1b74e1]`
- `sagat__ssf4-ultra2__old__new__4.tsv`: `# seed 1 failed: untranslated branches at frame 5061: [1b78c2]`
- `sagat__ae2012-ultra1__new__old__4.tsv`: `# seed 10 failed: untranslated branches at frame 2019: [1b74e1]`
- `sagat__ae2012-ultra2__old__new__4.tsv`: `# seed 1 failed: untranslated branches at frame 5061: [1b78c2]`
- `sagat__usf4-ultra1__new__old__4.tsv`: `# seed 10 failed: untranslated branches at frame 2019: [1b74e1]`
- `sagat__usf4-ultra2__new__old__4.tsv`: `# seed 31 failed: untranslated branches at frame 2940: [1b78c2]`
- `sagat__usf4-ultradouble__new__old__4.tsv`: `# seed 20 failed: untranslated branches at frame 4862: [1b78c2]`
- `sagat__usf4-ultradouble__new__old__4.tsv`: `# seed 31 failed: untranslated branches at frame 2940: [1b78c2]`
- `sagat__usf4-ultradouble__new__old__4.tsv`: `# seed 39 failed: untranslated branches at frame 4802: [1b78c2]`
- `ryu__sf3ng-art2__old__new__4.tsv`: `# seed 26 failed: untranslated branches at frame 4268: [10192f]`
- `adon__sf3threes-art3__old__new__4.tsv`: `# seed 19 failed: untranslated branches at frame 3911: [1a30c8]`
- `ryu__st__new__old__4.tsv`: `# seed 20 failed: untranslated branches at frame 3070: [10491c]`
- `ryu__st__new__old__4.tsv`: `# seed 28 failed: untranslated branches at frame 4888: [10491c]`
- `ryu__st__old__old__4.tsv`: `# seed 5 failed: untranslated branches at frame 3025: [10491c]`
- `ryu__st__old__old__4.tsv`: `# seed 7 failed: untranslated branches at frame 2988: [10491c]`
- `ryu__st__old__old__4.tsv`: `# seed 12 failed: untranslated branches at frame 4586: [10491c]`
- `ryu__st__old__old__4.tsv`: `# seed 28 failed: untranslated branches at frame 3144: [10491c]`
- `ryu__st__old__old__4.tsv`: `# seed 33 failed: untranslated branches at frame 2940: [10491c]`
- `ryu__st__old__old__4.tsv`: `# seed 34 failed: untranslated branches at frame 3108: [10491c]`
- `retsu__st__old__old__4.tsv`: `# seed 18 failed: untranslated branches at frame 2919: [122d35]`
- `gen__st__new__old__4.tsv`: `# seed 28 failed: untranslated branches at frame 1793: [17302c]`
- `gen__sfz3__new__old__4.tsv`: `# seed 3 failed: untranslated branches at frame 4939: [172f15]`
- `gen__sfz3__old__old__4.tsv`: `# seed 1 failed: untranslated branches at frame 7511: [172f15]`
- `gen__sfz3__old__old__4.tsv`: `# seed 8 failed: untranslated branches at frame 4697: [172f15]`
- `gen__sfz3__old__old__4.tsv`: `# seed 31 failed: untranslated branches at frame 6101: [172f15]`
- `eagle__st__new__old__4.tsv`: `# seed 17 failed: untranslated branches at frame 4164: [193220]`
- `eagle__st__new__old__4.tsv`: `# seed 26 failed: untranslated branches at frame 3481: [193220]`
- `eagle__sfz3__new__old__4.tsv`: `# seed 35 failed: untranslated branches at frame 1898: [192b65]`
- `eagle__sfz3__old__old__4.tsv`: `# seed 2 failed: untranslated branches at frame 3767: [192b55]`
- `eagle__sfz3__old__old__4.tsv`: `# seed 29 failed: untranslated branches at frame 4549: [192b65]`
- `eagle__sfz3__old__old__4.tsv`: `# seed 30 failed: untranslated branches at frame 1595: [192b55]`
- `eagle__sfz3__old__old__4.tsv`: `# seed 37 failed: untranslated branches at frame 4496: [192b55]`

No unfinished rows.

### Engine-stop address occurrences

| Run | Fighter | Group | Address | Occurrences |
| --- | --- | --- | --- | --- |
| Baseline controls | adon | primary | 0x1a25c7 | 3 |
| Baseline controls | adon | primary | 0x1a2d6e | 7 |
| Baseline controls | adon | primary | 0x1a2eef | 3 |
| Baseline controls | adon | primary | 0x1a30c8 | 1 |
| Baseline controls | adon | primary | 0x1a37df | 2 |
| Baseline controls | birdie | primary | 0x1823c7 | 3 |
| Baseline controls | birdie | primary | 0x18366b | 15 |
| Baseline controls | eagle | old-old | 0x192b55 | 3 |
| Baseline controls | eagle | old-old | 0x192b65 | 1 |
| Baseline controls | eagle | primary | 0x192b55 | 2 |
| Baseline controls | eagle | primary | 0x192b65 | 2 |
| Baseline controls | eagle | primary | 0x1930e7 | 3 |
| Baseline controls | eagle | primary | 0x193220 | 6 |
| Baseline controls | eagle | primary | 0x193d72 | 27 |
| Baseline controls | eagle | primary | 0x193ef3 | 16 |
| Baseline controls | eagle | primary | 0x193efb | 5 |
| Baseline controls | gen | old-old | 0x172f15 | 3 |
| Baseline controls | gen | primary | 0x172f15 | 4 |
| Baseline controls | gen | primary | 0x17302c | 1 |
| Baseline controls | gen | primary | 0x1736c8 | 20 |
| Baseline controls | joe | primary | 0x143708 | 2 |
| Baseline controls | joe | primary | 0x143889 | 12 |
| Baseline controls | ken | primary | 0x112455 | 2 |
| Baseline controls | mike | primary | 0x1539f3 | 2 |
| Baseline controls | retsu | old-old | 0x122d35 | 1 |
| Baseline controls | retsu | primary | 0x122d35 | 10 |
| Baseline controls | retsu | primary | 0x122d45 | 1 |
| Baseline controls | retsu | primary | 0x124057 | 20 |
| Baseline controls | ryu | old-old | 0x10491c | 6 |
| Baseline controls | ryu | primary | 0x10192f | 1 |
| Baseline controls | ryu | primary | 0x10491c | 2 |
| Baseline controls | sagat | primary | 0x1b74e1 | 4 |
| Baseline controls | sagat | primary | 0x1b78c2 | 6 |
| Current controls | adon | primary | 0x1a25c7 | 3 |
| Current controls | adon | primary | 0x1a2d6e | 7 |
| Current controls | adon | primary | 0x1a2eef | 3 |
| Current controls | adon | primary | 0x1a30c8 | 1 |
| Current controls | adon | primary | 0x1a37df | 2 |
| Current controls | birdie | primary | 0x1823c7 | 3 |
| Current controls | birdie | primary | 0x18366b | 15 |
| Current controls | eagle | old-old | 0x192b55 | 3 |
| Current controls | eagle | old-old | 0x192b65 | 1 |
| Current controls | eagle | primary | 0x192b55 | 2 |
| Current controls | eagle | primary | 0x192b65 | 2 |
| Current controls | eagle | primary | 0x1930e7 | 3 |
| Current controls | eagle | primary | 0x193220 | 6 |
| Current controls | eagle | primary | 0x193d72 | 27 |
| Current controls | eagle | primary | 0x193ef3 | 16 |
| Current controls | eagle | primary | 0x193efb | 5 |
| Current controls | gen | old-old | 0x172f15 | 3 |
| Current controls | gen | primary | 0x172f15 | 4 |
| Current controls | gen | primary | 0x17302c | 1 |
| Current controls | gen | primary | 0x1736c8 | 20 |
| Current controls | joe | primary | 0x143708 | 2 |
| Current controls | joe | primary | 0x143889 | 12 |
| Current controls | ken | primary | 0x112455 | 2 |
| Current controls | mike | primary | 0x1539f3 | 2 |
| Current controls | retsu | old-old | 0x122d35 | 1 |
| Current controls | retsu | primary | 0x122d35 | 10 |
| Current controls | retsu | primary | 0x122d45 | 1 |
| Current controls | retsu | primary | 0x124057 | 20 |
| Current controls | ryu | old-old | 0x10491c | 6 |
| Current controls | ryu | primary | 0x10192f | 1 |
| Current controls | ryu | primary | 0x10491c | 2 |
| Current controls | sagat | primary | 0x1b74e1 | 4 |
| Current controls | sagat | primary | 0x1b78c2 | 6 |
| Final | adon | adjacent | 0x1a25cf | 2 |
| Final | adon | old-old | 0x1a25c7 | 2 |
| Final | birdie | adjacent | 0x1823c7 | 36 |
| Final | birdie | old-old | 0x1823c7 | 1 |
| Final | birdie | primary | 0x1823c7 | 12 |
| Final | birdie | primary | 0x1823cf | 1 |
| Final | eagle | adjacent | 0x192b55 | 18 |
| Final | eagle | adjacent | 0x192b65 | 3 |
| Final | eagle | adjacent | 0x193220 | 12 |
| Final | eagle | adjacent | 0x1942f3 | 9 |
| Final | eagle | old-old | 0x192b55 | 3 |
| Final | eagle | old-old | 0x193220 | 1 |
| Final | eagle | old-old | 0x1942f3 | 1 |
| Final | eagle | primary | 0x192b55 | 11 |
| Final | eagle | primary | 0x192b5d | 1 |
| Final | eagle | primary | 0x193220 | 23 |
| Final | eagle | primary | 0x1942f3 | 6 |
| Final | geki | adjacent | 0x1328c9 | 3 |
| Final | gen | adjacent | 0x172467 | 8 |
| Final | gen | adjacent | 0x17246f | 1 |
| Final | gen | adjacent | 0x1725cd | 5 |
| Final | gen | adjacent | 0x17302c | 17 |
| Final | gen | adjacent | 0x1730e1 | 2 |
| Final | gen | adjacent | 0x174361 | 3 |
| Final | gen | old-old | 0x1725cd | 1 |
| Final | gen | old-old | 0x17302c | 1 |
| Final | gen | primary | 0x1725cd | 2 |
| Final | gen | primary | 0x17302c | 21 |
| Final | gen | primary | 0x174361 | 1 |
| Final | joe | primary | 0x142577 | 2 |
| Final | mike | adjacent | 0x152bd0 | 4 |
| Final | mike | primary | 0x152bd0 | 2 |
| Final | retsu | adjacent | 0x1229db | 1 |
| Final | ryu | adjacent | 0x103eb8 | 1 |
| Final | sagat | primary | 0x1b8365 | 1 |

Failure lines not classified as untranslated-branch stops: 0. All remain listed verbatim above.

## Raw final-file checksums

| Raw final file | SHA-256 |
| --- | --- |
| adon__sf6__new__new__1-2.tsv | cc2ae6e73ac19ecc8a79ebb86dd1c3b9c43d2e3fba1dbafb173b98bbcd2e3868 |
| adon__sf6__new__new__2-1.tsv | 63c9af0853c9ae45597cd2a984e7069c8a406e74c9c0624a9e696ab5e8dcee2a |
| adon__sf6__new__new__2-3.tsv | a0e16a30c55a584fa7d2455c239de486798e2e31d9ec0c103e2005d23292bde3 |
| adon__sf6__new__new__3-2.tsv | 044a65e8f9d8d5e1f5be92f9bd20f5fdf6110a4a5d9cb209d99702163886bae2 |
| adon__sf6__new__new__3-4.tsv | 590d4088c16b9520eab8cceac47ee01b21ddb63aafb7574a75d610f93a00e8f1 |
| adon__sf6__new__new__4-3.tsv | 9292dbcefebe252f3317d7f79c0e5b4a7e825330dd0c4c855ca62f91ad12529c |
| adon__sf6__new__old__1.tsv | eaf712abe6acc075d0ebf85bdb85bda8a46ebc97372389b862d6c878cf57f3a6 |
| adon__sf6__new__old__2.tsv | f672814f8e45efcbe3207c21f6ad38c4dfdf85c01afefa11bbb2c139f2f40e78 |
| adon__sf6__new__old__3.tsv | a4ccf4190f4ad707d9c5c1d499a51d626a39b8d1c265b65f43d95c2f65fb6be4 |
| adon__sf6__new__old__4.tsv | 36eb37029659dac4032a74af418c4de96691288855d3d3882502d061c18df775 |
| adon__sf6__old__new__1.tsv | 4fa0123da2f270bdb1d8f11874086a4e902036f3f69ceace82ce389385b7227e |
| adon__sf6__old__new__2.tsv | a50e55b83d2c4497019efa90af1c7b4b4fa32581d826adf102aeaede7745694c |
| adon__sf6__old__new__3.tsv | 2e7371fa722081296510000cef3c0f53cd6a3ece58ead5e94692b0de3ede450b |
| adon__sf6__old__new__4.tsv | e078106b4a49a9c5f9f6d06d91ade9d3aa6ff42e1822064cc197eb5c9b02f5cc |
| adon__sf6__old__old__4.tsv | 637499a316a1055275893c592a5c4134c4f85de1de360245089663beaad90104 |
| birdie__sf6__new__new__1-2.tsv | c8dfec3f4fb7c141b8a5d6663c3ae78aa0b1044dd5965447506e88373f33e3d6 |
| birdie__sf6__new__new__2-1.tsv | d86e93140422611be8a3dcf688c222049803bfd58a6f009850412929b97951a8 |
| birdie__sf6__new__new__2-3.tsv | 7afbba33b1158c8b3c82506fd11164e4eb5f77469e615eea5668e7670bccc357 |
| birdie__sf6__new__new__3-2.tsv | c5cc599b4c44fb7581e3722255860f92d7ef58ebfdd1ac907a74a3994eb882c8 |
| birdie__sf6__new__new__3-4.tsv | f1d2674428fe0a6066a1e1c5df7162051bc43c310fe51f9d441f98bd40aa95a4 |
| birdie__sf6__new__new__4-3.tsv | dd945508dc8ccb3d7ef4f41fe91fd2b0fe35f6241b794aa484eecf74b1c50df3 |
| birdie__sf6__new__old__1.tsv | 73e76f784db78dc8b2266341609f79069c1757b1fba8e43dd1841680f858e55e |
| birdie__sf6__new__old__2.tsv | 55eefc5b78432e8e6b040ca468fc85ec39c9c9bdd95ac91b56ca1646e9b400ad |
| birdie__sf6__new__old__3.tsv | 742f6058634af9fb0645877bdbcbdd9515f56e259e6ce27a22432f399853e2fd |
| birdie__sf6__new__old__4.tsv | 4039265210799c581c030c4fa7db8dc70276f2cf2358663c6ad2ac18b10e3f0c |
| birdie__sf6__old__new__1.tsv | 95f6c34a63746b0e21b48eb1a1bba71605b65bd3899b2acf6327301a1bf574b4 |
| birdie__sf6__old__new__2.tsv | 6332cd864ddbae300bf41835a4c4fa98c569ed6e5f483d936a7491113b157df2 |
| birdie__sf6__old__new__3.tsv | fdd13f58dbfdfe4dd55c78f8a07065c9259bfe7e1bbca8aef0d5d94e15df3228 |
| birdie__sf6__old__new__4.tsv | 120da2b25248bdd26ef06f55c1d05967d0ddb1abff900433434b8cd3e6d789a2 |
| birdie__sf6__old__old__4.tsv | f1176460fc4cb18a3366ffde0fa08a49cb8ad9d83bfe76ba6b57c96416f16861 |
| eagle__sf6__new__new__1-2.tsv | 85c5b758920de203e8d7f1e5881cd056820f7a508d37791c6c80da62d090a756 |
| eagle__sf6__new__new__2-1.tsv | 929931fa3d2e06ab96fa44bfa9e59c85c5259e8184fbb3733ea76739abcb9f4f |
| eagle__sf6__new__new__2-3.tsv | 415aa2c62d09bc8a2d4aef2c57589bd2eb4ac54635394f9089218aa4d20b8418 |
| eagle__sf6__new__new__3-2.tsv | 728da76727f5e215baf13daa884819cfa535b34465610e616ad102a2a484b337 |
| eagle__sf6__new__new__3-4.tsv | ed4afe5c8bc6d55a6d958d05885638111b458a9a101406b9963ec40d0fb7d27d |
| eagle__sf6__new__new__4-3.tsv | a645db2b944b3d82213c109631b4288a0bc4ab4d862363ca22b307900c1e3ad0 |
| eagle__sf6__new__old__1.tsv | 22cdf22c32919f0a25939c855cc9c5637cb7206b1a321786339ef22613e74964 |
| eagle__sf6__new__old__2.tsv | 372be82694c1b040eb7f254b2e307d853f48044ed013658f0128d06f87ecdcc5 |
| eagle__sf6__new__old__3.tsv | 18ea8fcf84a7270ee97753899950600168c0cc50c1fc110281eaf86fd081de16 |
| eagle__sf6__new__old__4.tsv | e26327e5c1b8a8570012913e64a808c097250aa78c957a49e9647d81d70d58b2 |
| eagle__sf6__old__new__1.tsv | 34cbae3d4dadf5f6e4d0c4a87e4f3ef392c9dac9398c78bcc2e0bf95a2d4fee7 |
| eagle__sf6__old__new__2.tsv | fd983a119303fce6219845aaf47c5b917b2a22354fef21f5549cbff0de8ca64e |
| eagle__sf6__old__new__3.tsv | 2f6a9c0584639067a234f37923f06eae78e0224219898e280bff80d79b22e94e |
| eagle__sf6__old__new__4.tsv | aca78d4189bf965aaee7eeea56e9e020470dddc09087f6f7048fff1fdb1a8b67 |
| eagle__sf6__old__old__4.tsv | d120ec80bf48b07d6d8fadcdc321e5f2a834ebc62ac0dbc61b67dc1411b9b407 |
| geki__sf6__new__new__1-2.tsv | bc5d353b19a1d04f7c4fb3c609f3075896039ab5de12f8b3d863e4f037114393 |
| geki__sf6__new__new__2-1.tsv | 22b4ffdd8fa97389434b57062ade33e18a404c0dca26fb752ed27b8d40d0c68a |
| geki__sf6__new__new__2-3.tsv | 210e947a84ce98118245b8cd9126c044b19ab0796d85d5f5e9f775a4894e593c |
| geki__sf6__new__new__3-2.tsv | ddb790c37ee1b9f46fb66269e8b769cc284d8d0cb5ad164c6b2a68ef3e1742af |
| geki__sf6__new__new__3-4.tsv | 5c18c2cdc2427e9049ffbda7372011a33a6a363ac049e0a31c08442c19b059e1 |
| geki__sf6__new__new__4-3.tsv | c74c92c72810abbd900e692c548c321921be31e1e67dae83c1aa7475bef69682 |
| geki__sf6__new__old__1.tsv | 6aa34c598ed1bea725acfdd492ab45f4afcdc22af75cc3d32b346853af4a787f |
| geki__sf6__new__old__2.tsv | d0160d9b11244f0406557c7c69cec85856a2653cfc99cd5d7785b9915fbc2fb1 |
| geki__sf6__new__old__3.tsv | 2fdece41be796960b65afce8cb374f7f2a7896bae28340d8b12962bd80d50d47 |
| geki__sf6__new__old__4.tsv | d6c420f54d02ffa23eaae487d389649d0763db330f3de3ba056a52939c807b67 |
| geki__sf6__old__new__1.tsv | 0ae3a1788ed3b7cc54d1655bd562008e3daee95dd8aae7cb5d9071d660df08dc |
| geki__sf6__old__new__2.tsv | 8246b06ebfdbeb5b4a7354b22aa74a8de11492e3ac47496d7105da52dec68840 |
| geki__sf6__old__new__3.tsv | b4da7979daae3d9b98c089687874ced30b0144dc969e53acb9213cc5a71dd981 |
| geki__sf6__old__new__4.tsv | 47813f20763839a96c9b10b45e54942cfdcf0e2d0d12ebfc4b3dbe44a27aab77 |
| geki__sf6__old__old__4.tsv | dc67484954b14b28e323e5ad8474d151b866963bd21089e3592184a542e2dded |
| gen__sf6__new__new__1-2.tsv | 55a4e42cb1d054f86ec785aa8b48251722804f13a1b437cffe7a039509d9d4bd |
| gen__sf6__new__new__2-1.tsv | 8236429bb2d2ce8b0fae55f51ec1ffb19752be471ccf83a88b7d26679032f7ab |
| gen__sf6__new__new__2-3.tsv | bb1d21af238d2cd38498311b74244d71bfae590e4d4ca77ab689c46f4027422f |
| gen__sf6__new__new__3-2.tsv | c20eb7625770f7b6cfb7674fc83a78d26b85d87cf7b8f39433e44d22cf2f02dd |
| gen__sf6__new__new__3-4.tsv | d91c271b35451b8d7750633f003cb784087ab0cc7ba0a4eb7fb246e8cb9f6f50 |
| gen__sf6__new__new__4-3.tsv | 6a0e396a0b6f425ad706c7db67cea831d82d876668ff10e4edaad954724abd2e |
| gen__sf6__new__old__1.tsv | 3e8e70ff180df6f6555f80f5961fcc089bc150ca95a320d9ba430c98b357548c |
| gen__sf6__new__old__2.tsv | b654c6d83e4087f5a45285e920d1eec7ef4b9d83080ae0d304bd38c8850f54f7 |
| gen__sf6__new__old__3.tsv | 70154087bd7dc362821164bb223996ef35b304b3aabd6a5fc3ba142c5817bce6 |
| gen__sf6__new__old__4.tsv | cdd705b478d1582f5ea8a3b191a613ce71c1c920bbe6b29100f4cdb508d9ee5b |
| gen__sf6__old__new__1.tsv | a9e3b971e81296fdf7ae5d6c408ae92692b52602761fa78b1332567822ad03e4 |
| gen__sf6__old__new__2.tsv | 0b7ff7b6c04ae03c49457dacd75cccb5166d1940a2d3ffb242f8035df5ef1d02 |
| gen__sf6__old__new__3.tsv | 55af772f02f79402b876f22417b36e9f66bd7d91685a4669cb519d715bf87e17 |
| gen__sf6__old__new__4.tsv | d82d7e573a71a6a61578d739208340cb4daa12cacf7e87980be88782cb72efed |
| gen__sf6__old__old__4.tsv | c6a15ce8db866a4ebf204e91eff25e6e0c59aaf80824824597394e0e93a160e1 |
| joe__sf6__new__new__1-2.tsv | d4d34e21c3090e81a5ecbb2e7bc047747657226206f65b5a435f43d3a4ff0049 |
| joe__sf6__new__new__2-1.tsv | cddfc4bcf62763f49a58e86ef0d6fe955cd402a37944bc83b6702d7e9e550db6 |
| joe__sf6__new__new__2-3.tsv | 332e86f6ed3dc644611119a5bbb4238b5b7c9f2b760760c3f91954dacf0e73f6 |
| joe__sf6__new__new__3-2.tsv | 9bbe857ea685bddb0ff253aaaae05ad2d7b46178651aa25331daeaedc49d8e9b |
| joe__sf6__new__new__3-4.tsv | fe796aba87304896f06c5b1fd3b143453b511acfe65e93612c5a43e07720d7c4 |
| joe__sf6__new__new__4-3.tsv | b02f7101dd17b5ceada819b37c2fac7d8c447d868b7bf7311a191d8035c2b469 |
| joe__sf6__new__old__1.tsv | acdc4b6ead761cf8c7d36f0f4f205abddb2e17f80949d5023a6a46c7fb039f96 |
| joe__sf6__new__old__2.tsv | 7f9480e5487b0a13dd07f5f50cfc39ed4391fdfd022ba300f52ca7355f871750 |
| joe__sf6__new__old__3.tsv | 7d4935011557ea2fb62d85cbd0b464f50416b8d79842069d883e1670bc4709ff |
| joe__sf6__new__old__4.tsv | a4ec2269b02004ac0e9de0dab6f462dfbea3e2e08a479b56018c8051f14814b8 |
| joe__sf6__old__new__1.tsv | c8b1a422eb72d473dc5894ddc946f7dab578e2fcffc45bc508e652035396f9fc |
| joe__sf6__old__new__2.tsv | 2796848b3d7de7e1abb49c23b0be8f8f779bcbe08ad4446b83a1973408f40353 |
| joe__sf6__old__new__3.tsv | 94b4a08c3816065eb5e5aadf0efb5657a9d31f14a992121998eeeb4faf77eb0d |
| joe__sf6__old__new__4.tsv | b8546b2c88e5c57901c7bfee238e4fd939e209ab3131a250fab25ca7acea2a74 |
| joe__sf6__old__old__4.tsv | fc59968b25f7f3c181af852db9f991a95b5fb9b3e26aa0aa9536c7d0da78f0a0 |
| ken__sf6__new__new__1-2.tsv | 58a5575d294960b653d69f2aa72c1da47120708fe53325b4d57063c2cfb70563 |
| ken__sf6__new__new__2-1.tsv | 0277cfdc085bd441be91776a18bf7182de58249284630e70f9ea922f0d80f64f |
| ken__sf6__new__new__2-3.tsv | fdcea45d1e321788fe1b465119efc6d37890db376765ed1843634a4b834bf1eb |
| ken__sf6__new__new__3-2.tsv | 0449701745f4bf4beb4292d28bd74bb66cbe812250a3740bd98c4fa5c9099e9b |
| ken__sf6__new__new__3-4.tsv | ea297202f7e77e14fcfbecbafcc279eaf4708a33c4030743d5366ba3697c2cf4 |
| ken__sf6__new__new__4-3.tsv | cda5063092318f56eb8c8802749dbfff0c228cb39516eb9d0b26013741424c11 |
| ken__sf6__new__old__1.tsv | 0f70cdedaed93ddc1be54dfaae5817fb5f04354b4eab6f2fa47b1788721cd9e7 |
| ken__sf6__new__old__2.tsv | 55c7c1e955d2ce83f96c600f074d1584f57880026024440707ba5197803f7c48 |
| ken__sf6__new__old__3.tsv | 7b28d9674af1de1d09ff2b34dd57ec84bcb495722ed94a170802abf57d68f79a |
| ken__sf6__new__old__4.tsv | 7ead91d137400e602f95408a80e37f6f2a16e5ff8ce48013bd44786b3088c82b |
| ken__sf6__old__new__1.tsv | e38023ecced7980d8fc4b93bc3ede8f2dbcbf656da30ab0fce9756a536e8e8c9 |
| ken__sf6__old__new__2.tsv | 688e2e23aa8044320ca8412dae0b188cdf32f8372f7b821659c25f3ccf609ff9 |
| ken__sf6__old__new__3.tsv | 565002028ad9113dfade6a3a1e18d1f36c2f93538feb3e777dbf7f69da5ff56b |
| ken__sf6__old__new__4.tsv | 179ef6bbbc3159ebdaeccded24b5a9eacb3e71fbb98e3716568139212cc572fc |
| ken__sf6__old__old__4.tsv | 18a4dce7b1d2a21db6186ae21a62ee340f11f6c8aafe2667c518ed50a55b97d6 |
| lee__sf6__new__new__1-2.tsv | 1ce9dc26b6598442f361004648cc283d79a1633a94bd55859a427b4ea604663c |
| lee__sf6__new__new__2-1.tsv | 03986b107874f89d0725332953502e2b90c791ee74cd4f3ce8a5efbc732e92f4 |
| lee__sf6__new__new__2-3.tsv | 49d1dc6edfdb36cd14f8bc7e01d35ce47255fc11a97e342dbddc853600ac17de |
| lee__sf6__new__new__3-2.tsv | fafd71369a25997bb6eed4b0556b5e3f8a7e1c7895179fd5baa7bea3f9fca5aa |
| lee__sf6__new__new__3-4.tsv | b1a0eaeb22c4d2899bdeaf39bc3778dcb01f220e215a7892601c0733897745c0 |
| lee__sf6__new__new__4-3.tsv | 7af4fd7193efcac1214538a0bec03619bcbb0756f707ba94963193f0f37de803 |
| lee__sf6__new__old__1.tsv | 800423079bc252bc2a80a88c5de1bc49a41b080516e6d681b25715e761f3d86c |
| lee__sf6__new__old__2.tsv | bfd86a7c37202784b54eea54d15699c5641606a4e82128fcd63da5af98e38dd2 |
| lee__sf6__new__old__3.tsv | cab7b82e00437b64f57bab4e6af685915a760b987691eab1d52d2065fde75710 |
| lee__sf6__new__old__4.tsv | 59b65913fa37b54f34dc9d7c6de104b9e474209425a6e3bec5ce44ed69d5f656 |
| lee__sf6__old__new__1.tsv | 38776a61954c24018dafbd3bad387b7806f5ffdcfb995691fb1d8894aa9c45fd |
| lee__sf6__old__new__2.tsv | f3b7600c692fd6e6cca7b3786fea483f95f769bb42627de5aa4dd71ed26af050 |
| lee__sf6__old__new__3.tsv | a40dd7ffe434e687ec798f1e6bb5fbf30c9c7dfab6c0f3041817590ea126e57d |
| lee__sf6__old__new__4.tsv | 9081233a0f8784cf722d425333a2f76596636b2260d3b4258fd90f65bf40ca38 |
| lee__sf6__old__old__4.tsv | 26b45fdcc9b1212aa1a6a6e7230bc3cdee321d20a30eadd90409acde580532b2 |
| mike__sf6__new__new__1-2.tsv | 0ab35de88074336cfc5acc7951f35f885b483bac2148a01e17d25ebd48fec113 |
| mike__sf6__new__new__2-1.tsv | cf270c72b7431186b46c1d2f690514290e768e017c913f3284e3ddc52a598b55 |
| mike__sf6__new__new__2-3.tsv | 722652f98540f8e544395b2e485dda3dc46185d2b86654493109ea206f834965 |
| mike__sf6__new__new__3-2.tsv | 7da5b6cf96c978249dc5e9bc94fb4c2d7738771fdda73d03737d9e910b502b5d |
| mike__sf6__new__new__3-4.tsv | 32aa95041409938a4f9056fce09a1a739dc9d369cf0e7f169bd5e6a541a960a2 |
| mike__sf6__new__new__4-3.tsv | 949960f60a41f8c738e476aa534d2211bf41fca5739330a6e92c067dcdc32cc8 |
| mike__sf6__new__old__1.tsv | 61c879cb5f794e5ec31014ae0657f4a220ec2ec4ebf8dbb0f4c8bb13c5aeca3d |
| mike__sf6__new__old__2.tsv | 8f24f55027c8532feb4489d387991a958e682704a9702cdb6d9dddec2b2ee46d |
| mike__sf6__new__old__3.tsv | 776efbc0176499d4db8105cbf5a42cf7164dc3f71196250722e3f322469b11a8 |
| mike__sf6__new__old__4.tsv | b48569c3803d2dce352e5c8118c83103cf4695afacbef4f3646d8cec65494f2a |
| mike__sf6__old__new__1.tsv | 443ab8438aa9d7e3a0c1f6a69ebd40fac31294a49b385ab86199e60fe2d737d8 |
| mike__sf6__old__new__2.tsv | eaafd0a61e8a53a763684922df3ddb5223f31b0727bb33142323a573e4bc3506 |
| mike__sf6__old__new__3.tsv | ccc36006e835953d6a2df6842e45f0c9998156b8129b36c78bce4d8b6b182f9b |
| mike__sf6__old__new__4.tsv | 248d4635bb2b6c1c9cdc776e67d45e459f43b84635e5b38f896fbd943050654b |
| mike__sf6__old__old__4.tsv | 7e6abcd3a4bf9277eb3c90789403bfc75345503281cbd5eff30cb5f9672b1782 |
| retsu__sf6__new__new__1-2.tsv | 2e0497000b5d66bf6551905424ddedc25896fb2701faa2990d262d98ddd2faca |
| retsu__sf6__new__new__2-1.tsv | 68a023ecd05285f303768d66676c8a6b151fc7085b31b7a0e9cc3a4461fc71f3 |
| retsu__sf6__new__new__2-3.tsv | 0885628d8e9616c830a2c85c7148e1ae902a9f91d40a909b57721bb9068f8d51 |
| retsu__sf6__new__new__3-2.tsv | c52256b426cf429a4d439fd2dcc2cf4ff71fb435fe1fb6d0117931eaf3e1d983 |
| retsu__sf6__new__new__3-4.tsv | 26da54263c7f81c7bac50986723b61aae896cdd67f414acf1edd9d7e8bc8b402 |
| retsu__sf6__new__new__4-3.tsv | faa13f9730df834b3568ad9e6697af59c0a9bce332d8b661c7b7b7001da1da46 |
| retsu__sf6__new__old__1.tsv | e0724423e6e12c1a4858910e6360625e32c732f140e1cb8d5a3e0f97d50607b4 |
| retsu__sf6__new__old__2.tsv | ffe7bc3029aaa04e4e4f3dc96a2ebbecd6ffc0e60d01efa6a35826b8f1c5bde6 |
| retsu__sf6__new__old__3.tsv | 2295751371988f529ab3dfd5458c8ca3f5fc6c134966282a2d2219426799f0a1 |
| retsu__sf6__new__old__4.tsv | cf0045dd81cc60a5b06ff7a11567ab41bb102e156a2f32f55b8d15befd685634 |
| retsu__sf6__old__new__1.tsv | 21e30a395e9a4003103329b17109bb9d2eabb115e7747fef4d77814229acd6ac |
| retsu__sf6__old__new__2.tsv | 344327993ddb5fa668805b63b229a0ee1abf4ae61be5c12f9f301b44d8fc4400 |
| retsu__sf6__old__new__3.tsv | 53e477cf87487e92002d90bec3247ca79cfd114d50049c6674eee248eb46565f |
| retsu__sf6__old__new__4.tsv | a3e4d461d109962fffb64a17f08da8c32a7ffa35604312ac70d4d548e5b80a4d |
| retsu__sf6__old__old__4.tsv | 3644e6a1605739f514c74386f67b92de88adbce34f943df5ec422c21363f888d |
| ryu__sf6__new__new__1-2.tsv | 6173cfc1dcd0adc525cb8f2513509c11170153a47de08be1a5177ce3a28082d3 |
| ryu__sf6__new__new__2-1.tsv | d858b0f54618a9f725ce535a5a18da4b586fa3da36246085093f304e2f1847b6 |
| ryu__sf6__new__new__2-3.tsv | 2a3fdf377f374d2edb194e77079ca7316f776e31d873df9fc32fe7ced29c2c5b |
| ryu__sf6__new__new__3-2.tsv | 68c2cbfc22f7423f226bf504c93218e9f2812a8d2ab645cef6dbfde8b7324cfd |
| ryu__sf6__new__new__3-4.tsv | c5e7ee10b87f810ecbf47c9346dd94d29ef73f80f17d1a7e66b3f64d5be8ee9e |
| ryu__sf6__new__new__4-3.tsv | 020f02fe45f45195d6d6585890810cffdc825d72093a6e76e0d58d0ca1f42bf5 |
| ryu__sf6__new__old__1.tsv | a31395c17b242fac824798f724e4863dfaea11fba11b4a37f23e3170c99ad88e |
| ryu__sf6__new__old__2.tsv | 597bd119dcc978ab7d5c4717df6a82703d1278dfc9c1f1e57a735b530f5e4b1c |
| ryu__sf6__new__old__3.tsv | cafe337a4d9616edc21485264fc7c53ad842b159bc4268b295e0ca1416aa5acc |
| ryu__sf6__new__old__4.tsv | 634c5b68f3e1972aaf14c8ff4bb8cceee0b69096099a833c634a118451553928 |
| ryu__sf6__old__new__1.tsv | 81346602870be23e70a5bb9bf5b2f4c77c5474b65ef211b0b4f7644bef75fd42 |
| ryu__sf6__old__new__2.tsv | 34c0bb2286af817e57e964d9958410d7e49ad558af8c3961d71a8740e3512c9c |
| ryu__sf6__old__new__3.tsv | 113feffe21cf76e062a75c966dcec4f3c13826ebcebecc4f8a57922b991a5e25 |
| ryu__sf6__old__new__4.tsv | d8a142b71a62759e03477c96f485a6d090e8b6dd833948ce2ce60ee8bd99bd1c |
| ryu__sf6__old__old__4.tsv | d9c24695769e3cbfdfe217895a98aaca47ae003c70fbd330181a9a8d79aa1f45 |
| sagat__sf6__new__new__1-2.tsv | 93aeb3c7166429dd85b8c74e54141186be625993f930097d60a80180f2dea260 |
| sagat__sf6__new__new__2-1.tsv | 05952ad784ebe111b8cde07aa583dc59f69ade9e29f8d7820c7988015b2142e4 |
| sagat__sf6__new__new__2-3.tsv | b917f5306d2dc56503e6d6bd73f163295e000573414ec4045bcc02f708e25804 |
| sagat__sf6__new__new__3-2.tsv | 04f91f61474a2d4b94445dd98bf13badbb11a3b8cf903f65b09a8d3de6a2a33b |
| sagat__sf6__new__new__3-4.tsv | 3e59cc2825952998e8f3bee8541662dc58f2ce7c9eaf86ef0785df7aa50ad3c1 |
| sagat__sf6__new__new__4-3.tsv | 9ec7d42d3e80f79bf30b10ed35cfd24f5617cf3439408c87522178597ef5b2be |
| sagat__sf6__new__old__1.tsv | 027ed8c2da39ac280e32cbfa7ac3bbabafad6b640b52d28cbdaad3da0bf12043 |
| sagat__sf6__new__old__2.tsv | 571973cf7e39da2755a747089180913dc983c6a7dc424e916c6a404955dbbd3e |
| sagat__sf6__new__old__3.tsv | 2fe74aabec22a024a8e4ef745c35253f48288ef11ba7a153ff24ccdc98f6a814 |
| sagat__sf6__new__old__4.tsv | dd3b20c0e392ed3be2d4d6d9b88275a60b113ac821e427a976b2940de460dc7b |
| sagat__sf6__old__new__1.tsv | 9f6476f6c052f3dca67d2960fe749cc5929a389b28d75978d0b73267689d4dc6 |
| sagat__sf6__old__new__2.tsv | 40305c9d4f86c0e66cfd29237891662dc4ac88829f638982c383976a96abfc9b |
| sagat__sf6__old__new__3.tsv | 0747c1d3c62b711205790ba9a6b9a52c3d4f57108e5bba0c2dfb283429cbe7ce |
| sagat__sf6__old__new__4.tsv | bf8da6b90c1bb9982e43995bcb0527b1a7dd7ddae48ff8d42b9ad7f33833eacf |
| sagat__sf6__old__old__4.tsv | 19bbd0aa5b04fd79624ace3bc8b25090976f4b7ec98bba24a4207f0b382f6c3b |


## Separate Dramatic Battle diagnostic

[Complete Dramatic Battle report](sc20-dramatic-measuring-run.md). The separate 44-cell grid covers all eleven new SF6 fighters as boss at each of the four levels, with stronger Ryu and a different stronger new-cast ally. All three controllers act independently.

There were **1,760 attempts: 1,757 completed, three native branch stops, zero unfinished, and one completed draw**. The completed outcomes were 297 boss wins and 1,459 allies wins. Every sampled boss/level recorded at least one round loss; there are no never-lost-round flags. This is a bounded diagnostic observation, with no numerical Dramatic win bar or human playtest inferred. The three stops remain separate and are listed verbatim in that report under the existing [native branch tracker #270](https://github.com/johnstreetfighter/sf1-port/issues/270).

The same frozen source `1018d722f308930758e12f092df29e0675dfd822` and executable SHA-256 `44c0d3a5bca73ed126dba846f90518e6af18efa0387037bb3033f55e7b4bb5da` produced this grid. All 44 cells passed complete seed, identity, native footer, exact exit, stdout, helper and owned-build provenance checks. All three fighters have separate exact frame-weighted activity metrics. Raw fingerprint: `48b975d8d6026f25c9b579ef760ca0156c1ad71bcc74b41003e552c16ee40350`.

Preserved evidence: `target/sc20/dramatic-1018d72/analysis.json`, `target/sc20/dramatic-1018d72/manifest.json`, `target/sc20/dramatic-driver.log` (observed exit 0, 88.1 seconds), and `target/sc20/dramatic-analysis.log` (complete strict analysis, observed exit 0). The delivered Dramatic Markdown is byte-identical to the strictly regenerated report.

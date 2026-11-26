# SC21: separate SF1+-only Dramatic Battle V2 diagnostic measuring report

Source `13a3e219b131e924e469d8d6f29d6369c8ee177e`, version `1.5.33`, frozen EXE SHA256 `95a31d9ba80719d77d7317fed5ef832338f20509dc6f5fd4f4f38cf6f200efcc`. 48/48 cells finalized,40 seeds per cell,2 independent allies and one boss, all three stronger controllers in the supported Sf1Plus/Standard style; actual workers:2.

All twelve fighters appear as boss in SF1+ at all four levels. Arcade1987 is outside the product's permitted Dramatic styles and is not measured here. For non-Ryu bosses, Ally1 is Ryu and Ally2 is the next different non-Ryu fighter cyclically, preserving the SC20 assignments. For Ryu as boss, Ally1 is Ken and Ally2 is Retsu. Every cell has three distinct fighters. This1920-attempt diagnostic is separate from the360-cell standard grid. Native setup is solo Dramatic under Definitive Overhaul, Players::One, independent ally lifebars and no friendly fire; all three Fair CPUs are independently seeded. No numerical win bar is invented. A never-lost-round flag is an observation for the maintainer, not proof a boss is unbeatable.

Completed wins/draws require ended=yes. Unfinished rows remain separate even when the native winner field names a partial-round leader. Round counts, damage and frame-weighted activity include every recorded row, including unfinished progress; stopped seeds have no such metrics. Damage is in reported health-bar points. The native footer uses all recorded rows and is checked with those semantics. Decimal division/formatting preserves exact integer-hundredths frame weights through display, including zero-live cells.

Totals: attempted=1920; recorded=1920; completed=1920; stopped=0; unfinished=0; boss_wins=341; allies_wins=1577; completed_draws=2; boss_rounds_won=1054; boss_rounds_lost=3331; rounds_drawn=2; boss_damage_hundredths=27354415; team_damage_hundredths=31293315; live_frames=4404384.

Aggregate descriptive activity across recorded live frames: ally1 idle%=20.8967, attacks/min=41.5895; boss idle%=25.5743, attacks/min=41.4676; ally2 idle%=30.3887, attacks/min=37.0160. Round counts are integer sums and do not weight activity. Stopped attempts add no metrics.

| Boss | Style | Level | Ally1 / Ally2 | Attempted/completed/stopped/unfinished | Completed boss/allies/draw | Recorded rounds boss/allies/draw | Recorded damage boss/team | Ally1 idle% / attacks-min | Boss idle% / attacks-min | Ally2 idle% / attacks-min | Never lost round |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| ryu | sf1plus | 1 | ken/retsu | 40/40/0/0 | 8/32/0 | 30/69/0 | 6936.25/6936.20 | 19.6816/31.5061 | 24.3781/40.9989 | 35.9232/31.2862 | no |
| ryu | sf1plus | 2 | ken/retsu | 40/40/0/0 | 4/36/0 | 12/75/0 | 4965.50/6372.00 | 18.7224/36.1129 | 22.1160/42.1895 | 34.2876/35.2196 | no |
| ryu | sf1plus | 3 | ken/retsu | 40/40/0/0 | 1/39/0 | 12/79/0 | 4962.75/6747.20 | 15.7009/37.5819 | 20.7553/43.7826 | 31.4535/39.5131 | no |
| ryu | sf1plus | 4 | ken/retsu | 40/40/0/0 | 0/39/1 | 12/80/1 | 5054.75/6879.80 | 14.9977/37.2778 | 19.4148/46.7181 | 31.2137/41.6041 | no |
| ken | sf1plus | 1 | ryu/retsu | 40/40/0/0 | 5/35/0 | 20/74/0 | 6291.20/6850.20 | 22.3882/34.1337 | 23.6752/42.0342 | 35.7378/32.1310 | no |
| ken | sf1plus | 2 | ryu/retsu | 40/40/0/0 | 1/39/0 | 8/78/0 | 4503.00/6440.40 | 19.7140/38.9672 | 21.3022/40.9682 | 34.1623/36.6556 | no |
| ken | sf1plus | 3 | ryu/retsu | 40/40/0/0 | 0/40/0 | 5/80/0 | 4213.00/6420.00 | 17.7040/40.6669 | 18.9090/44.6195 | 28.9401/41.5501 | no |
| ken | sf1plus | 4 | ryu/retsu | 40/40/0/0 | 0/40/0 | 6/80/0 | 3860.25/6469.00 | 16.7485/43.2507 | 18.0151/46.9303 | 29.1341/43.6395 | no |
| retsu | sf1plus | 1 | ryu/geki | 40/40/0/0 | 10/30/0 | 33/64/0 | 6772.00/6676.20 | 28.5078/39.0162 | 30.4533/39.3194 | 32.0805/34.3243 | no |
| retsu | sf1plus | 2 | ryu/geki | 40/40/0/0 | 17/23/0 | 44/56/0 | 7482.75/6484.80 | 24.2565/43.0298 | 28.8372/43.9288 | 28.3944/36.5618 | no |
| retsu | sf1plus | 3 | ryu/geki | 40/40/0/0 | 26/14/0 | 57/45/0 | 8057.50/6441.00 | 23.4543/45.9751 | 27.3720/46.0815 | 25.6133/34.6061 | no |
| retsu | sf1plus | 4 | ryu/geki | 40/40/0/0 | 21/19/0 | 54/46/0 | 7967.70/5973.80 | 23.6793/45.3069 | 25.4308/50.9443 | 27.0898/37.5661 | no |
| geki | sf1plus | 1 | ryu/joe | 40/40/0/0 | 0/40/0 | 3/80/0 | 4662.50/6336.00 | 25.1817/40.5142 | 23.7570/41.2879 | 34.3488/31.3531 | no |
| geki | sf1plus | 2 | ryu/joe | 40/40/0/0 | 0/40/0 | 1/80/0 | 3148.50/6210.00 | 23.2020/47.1106 | 22.4619/40.9016 | 34.4310/33.8538 | no |
| geki | sf1plus | 3 | ryu/joe | 40/40/0/0 | 0/40/0 | 3/80/0 | 3880.25/6318.00 | 21.5661/46.8462 | 21.3978/42.5533 | 34.3952/34.0917 | no |
| geki | sf1plus | 4 | ryu/joe | 40/40/0/0 | 0/40/0 | 1/80/0 | 3900.50/6208.00 | 20.5138/51.0377 | 22.1237/43.6913 | 35.7656/34.7653 | no |
| joe | sf1plus | 1 | ryu/mike | 40/40/0/0 | 4/36/0 | 17/75/0 | 5728.75/6722.00 | 21.9679/39.8466 | 23.9392/36.8073 | 30.1444/34.5691 | no |
| joe | sf1plus | 2 | ryu/mike | 40/40/0/0 | 2/38/0 | 9/77/0 | 4725.50/6451.60 | 20.4155/44.5899 | 22.7363/38.5269 | 29.7435/39.5984 | no |
| joe | sf1plus | 3 | ryu/mike | 40/40/0/0 | 0/40/0 | 9/80/0 | 5015.00/6716.00 | 19.6866/44.5378 | 21.2543/38.7245 | 29.5578/39.5365 | no |
| joe | sf1plus | 4 | ryu/mike | 40/40/0/0 | 5/35/0 | 16/72/0 | 5472.00/6465.60 | 19.5379/48.2576 | 21.4726/43.1936 | 29.7311/38.0425 | no |
| mike | sf1plus | 1 | ryu/lee | 40/40/0/0 | 16/24/0 | 39/55/0 | 7194.50/6251.00 | 20.0939/37.4369 | 31.0268/37.6947 | 34.3863/26.5300 | no |
| mike | sf1plus | 2 | ryu/lee | 40/40/0/0 | 17/23/0 | 44/54/0 | 7246.00/6459.20 | 16.9518/38.5878 | 25.8590/41.3087 | 31.6626/27.5394 | no |
| mike | sf1plus | 3 | ryu/lee | 40/40/0/0 | 22/18/0 | 50/49/0 | 7806.00/6332.20 | 15.4131/43.0934 | 24.6841/42.1655 | 29.9258/29.8003 | no |
| mike | sf1plus | 4 | ryu/lee | 40/40/0/0 | 28/12/0 | 64/34/0 | 8269.50/5771.20 | 13.5205/39.1751 | 22.6236/44.2951 | 27.4306/31.4120 | no |
| lee | sf1plus | 1 | ryu/gen | 40/40/0/0 | 2/38/0 | 12/77/0 | 5251.00/6601.60 | 24.7487/40.4037 | 34.3738/36.2175 | 37.2547/34.7214 | no |
| lee | sf1plus | 2 | ryu/gen | 40/40/0/0 | 0/40/0 | 9/80/0 | 4397.00/6628.00 | 23.2199/48.3288 | 33.2532/38.8629 | 36.1056/38.5124 | no |
| lee | sf1plus | 3 | ryu/gen | 40/40/0/0 | 1/39/0 | 6/79/0 | 3945.50/6418.20 | 21.2394/54.7245 | 32.7103/39.9447 | 33.7324/40.2819 | no |
| lee | sf1plus | 4 | ryu/gen | 40/40/0/0 | 1/39/0 | 6/79/0 | 3743.25/6451.20 | 22.3611/58.3056 | 30.2810/42.4958 | 33.7284/44.3817 | no |
| gen | sf1plus | 1 | ryu/birdie | 40/40/0/0 | 3/37/0 | 16/75/0 | 5454.00/6644.00 | 29.3474/40.4744 | 31.1671/38.9837 | 25.1513/43.2221 | no |
| gen | sf1plus | 2 | ryu/birdie | 40/40/0/0 | 0/40/0 | 7/80/0 | 4554.25/6605.00 | 27.2053/44.4216 | 29.6275/40.5304 | 21.8077/51.8655 | no |
| gen | sf1plus | 3 | ryu/birdie | 40/40/0/0 | 0/40/0 | 8/80/0 | 4691.25/6636.00 | 25.4517/49.3329 | 28.3873/44.0408 | 19.7604/57.3503 | no |
| gen | sf1plus | 4 | ryu/birdie | 40/40/0/0 | 8/32/0 | 24/65/0 | 5604.00/6412.00 | 25.0790/49.3152 | 27.4972/50.4073 | 17.5646/52.4963 | no |
| birdie | sf1plus | 1 | ryu/eagle | 40/40/0/0 | 5/35/0 | 18/72/0 | 5818.00/6525.60 | 21.3685/39.1049 | 23.6994/45.1281 | 33.1864/35.7026 | no |
| birdie | sf1plus | 2 | ryu/eagle | 40/40/0/0 | 6/34/0 | 24/69/0 | 6078.50/6678.20 | 18.4597/41.7009 | 20.1220/51.3389 | 30.4275/36.6974 | no |
| birdie | sf1plus | 3 | ryu/eagle | 40/40/0/0 | 6/34/0 | 21/70/0 | 5809.50/6408.00 | 17.2382/47.8545 | 19.3482/53.8022 | 30.5431/39.1192 | no |
| birdie | sf1plus | 4 | ryu/eagle | 40/40/0/0 | 4/36/0 | 20/76/0 | 5699.50/6909.80 | 16.6327/52.0948 | 18.7909/58.4853 | 30.0702/43.5922 | no |
| eagle | sf1plus | 1 | ryu/adon | 40/40/0/0 | 6/34/0 | 17/71/0 | 5389.25/6405.80 | 22.7526/37.6889 | 32.3838/34.5837 | 37.0566/27.1502 | no |
| eagle | sf1plus | 2 | ryu/adon | 40/40/0/0 | 6/34/0 | 21/73/0 | 5140.75/6792.40 | 20.1914/38.0162 | 29.5029/31.2025 | 34.5521/31.4360 | no |
| eagle | sf1plus | 3 | ryu/adon | 40/40/0/0 | 12/28/0 | 35/63/0 | 6912.75/6664.40 | 17.5666/38.0473 | 30.5220/35.7656 | 37.1636/28.5610 | no |
| eagle | sf1plus | 4 | ryu/adon | 40/40/0/0 | 20/20/0 | 48/47/0 | 7359.75/6171.60 | 15.7625/38.6776 | 30.7886/36.3732 | 36.2212/27.7659 | no |
| adon | sf1plus | 1 | ryu/sagat | 40/40/0/0 | 6/34/0 | 18/70/0 | 5448.00/6450.00 | 27.1117/38.2239 | 32.3838/32.6938 | 28.5615/42.2836 | no |
| adon | sf1plus | 2 | ryu/sagat | 40/40/0/0 | 0/40/0 | 11/80/0 | 4765.25/6860.75 | 24.2550/42.4327 | 31.9469/32.7483 | 27.4726/46.2330 | no |
| adon | sf1plus | 3 | ryu/sagat | 40/40/0/0 | 0/40/0 | 7/80/0 | 4347.50/6565.00 | 23.3336/43.4931 | 30.1123/34.1650 | 24.2911/50.0958 | no |
| adon | sf1plus | 4 | ryu/sagat | 40/40/0/0 | 0/40/0 | 5/80/0 | 3940.25/6472.00 | 21.0213/44.3963 | 29.8360/32.9890 | 21.4592/55.4786 | no |
| sagat | sf1plus | 1 | ryu/ken | 40/40/0/0 | 13/27/0 | 37/61/0 | 7550.75/6663.80 | 23.3086/34.3662 | 26.1353/36.6880 | 24.5296/33.9378 | no |
| sagat | sf1plus | 2 | ryu/ken | 40/40/0/0 | 20/19/1 | 50/52/1 | 8111.50/6864.40 | 19.4212/35.2688 | 21.7154/40.8916 | 19.5990/36.5636 | no |
| sagat | sf1plus | 3 | ryu/ken | 40/40/0/0 | 14/26/0 | 34/63/0 | 7443.75/6733.40 | 17.3483/39.0880 | 19.5492/42.7458 | 18.7154/42.0950 | no |
| sagat | sf1plus | 4 | ryu/ken | 40/40/0/0 | 21/19/0 | 51/47/0 | 7973.25/6440.60 | 17.3746/39.9196 | 18.6229/45.9432 | 18.3546/42.9824 | no |

## Every native comment and unfinished row

### ryu__sf1plus__ally1-ken__ally2-retsu__level-1.tsv

- `# boss Ryu: 40 matches, 8 won by the boss, 32 by the allies; rounds 30 won, 69 lost, 0 drawn`

### ryu__sf1plus__ally1-ken__ally2-retsu__level-2.tsv

- `# boss Ryu: 40 matches, 4 won by the boss, 36 by the allies; rounds 12 won, 75 lost, 0 drawn`

### ryu__sf1plus__ally1-ken__ally2-retsu__level-3.tsv

- `# boss Ryu: 40 matches, 1 won by the boss, 39 by the allies; rounds 12 won, 79 lost, 0 drawn`

### ryu__sf1plus__ally1-ken__ally2-retsu__level-4.tsv

- `# boss Ryu: 40 matches, 0 won by the boss, 39 by the allies; rounds 12 won, 80 lost, 1 drawn`

### ken__sf1plus__ally1-ryu__ally2-retsu__level-1.tsv

- `# boss Ken: 40 matches, 5 won by the boss, 35 by the allies; rounds 20 won, 74 lost, 0 drawn`

### ken__sf1plus__ally1-ryu__ally2-retsu__level-2.tsv

- `# boss Ken: 40 matches, 1 won by the boss, 39 by the allies; rounds 8 won, 78 lost, 0 drawn`

### ken__sf1plus__ally1-ryu__ally2-retsu__level-3.tsv

- `# boss Ken: 40 matches, 0 won by the boss, 40 by the allies; rounds 5 won, 80 lost, 0 drawn`

### ken__sf1plus__ally1-ryu__ally2-retsu__level-4.tsv

- `# boss Ken: 40 matches, 0 won by the boss, 40 by the allies; rounds 6 won, 80 lost, 0 drawn`

### retsu__sf1plus__ally1-ryu__ally2-geki__level-1.tsv

- `# boss Retsu: 40 matches, 10 won by the boss, 30 by the allies; rounds 33 won, 64 lost, 0 drawn`

### retsu__sf1plus__ally1-ryu__ally2-geki__level-2.tsv

- `# boss Retsu: 40 matches, 17 won by the boss, 23 by the allies; rounds 44 won, 56 lost, 0 drawn`

### retsu__sf1plus__ally1-ryu__ally2-geki__level-3.tsv

- `# boss Retsu: 40 matches, 26 won by the boss, 14 by the allies; rounds 57 won, 45 lost, 0 drawn`

### retsu__sf1plus__ally1-ryu__ally2-geki__level-4.tsv

- `# boss Retsu: 40 matches, 21 won by the boss, 19 by the allies; rounds 54 won, 46 lost, 0 drawn`

### geki__sf1plus__ally1-ryu__ally2-joe__level-1.tsv

- `# boss Geki: 40 matches, 0 won by the boss, 40 by the allies; rounds 3 won, 80 lost, 0 drawn`

### geki__sf1plus__ally1-ryu__ally2-joe__level-2.tsv

- `# boss Geki: 40 matches, 0 won by the boss, 40 by the allies; rounds 1 won, 80 lost, 0 drawn`

### geki__sf1plus__ally1-ryu__ally2-joe__level-3.tsv

- `# boss Geki: 40 matches, 0 won by the boss, 40 by the allies; rounds 3 won, 80 lost, 0 drawn`

### geki__sf1plus__ally1-ryu__ally2-joe__level-4.tsv

- `# boss Geki: 40 matches, 0 won by the boss, 40 by the allies; rounds 1 won, 80 lost, 0 drawn`

### joe__sf1plus__ally1-ryu__ally2-mike__level-1.tsv

- `# boss Joe: 40 matches, 4 won by the boss, 36 by the allies; rounds 17 won, 75 lost, 0 drawn`

### joe__sf1plus__ally1-ryu__ally2-mike__level-2.tsv

- `# boss Joe: 40 matches, 2 won by the boss, 38 by the allies; rounds 9 won, 77 lost, 0 drawn`

### joe__sf1plus__ally1-ryu__ally2-mike__level-3.tsv

- `# boss Joe: 40 matches, 0 won by the boss, 40 by the allies; rounds 9 won, 80 lost, 0 drawn`

### joe__sf1plus__ally1-ryu__ally2-mike__level-4.tsv

- `# boss Joe: 40 matches, 5 won by the boss, 35 by the allies; rounds 16 won, 72 lost, 0 drawn`

### mike__sf1plus__ally1-ryu__ally2-lee__level-1.tsv

- `# boss Mike: 40 matches, 16 won by the boss, 24 by the allies; rounds 39 won, 55 lost, 0 drawn`

### mike__sf1plus__ally1-ryu__ally2-lee__level-2.tsv

- `# boss Mike: 40 matches, 17 won by the boss, 23 by the allies; rounds 44 won, 54 lost, 0 drawn`

### mike__sf1plus__ally1-ryu__ally2-lee__level-3.tsv

- `# boss Mike: 40 matches, 22 won by the boss, 18 by the allies; rounds 50 won, 49 lost, 0 drawn`

### mike__sf1plus__ally1-ryu__ally2-lee__level-4.tsv

- `# boss Mike: 40 matches, 28 won by the boss, 12 by the allies; rounds 64 won, 34 lost, 0 drawn`

### lee__sf1plus__ally1-ryu__ally2-gen__level-1.tsv

- `# boss Lee: 40 matches, 2 won by the boss, 38 by the allies; rounds 12 won, 77 lost, 0 drawn`

### lee__sf1plus__ally1-ryu__ally2-gen__level-2.tsv

- `# boss Lee: 40 matches, 0 won by the boss, 40 by the allies; rounds 9 won, 80 lost, 0 drawn`

### lee__sf1plus__ally1-ryu__ally2-gen__level-3.tsv

- `# boss Lee: 40 matches, 1 won by the boss, 39 by the allies; rounds 6 won, 79 lost, 0 drawn`

### lee__sf1plus__ally1-ryu__ally2-gen__level-4.tsv

- `# boss Lee: 40 matches, 1 won by the boss, 39 by the allies; rounds 6 won, 79 lost, 0 drawn`

### gen__sf1plus__ally1-ryu__ally2-birdie__level-1.tsv

- `# boss Gen: 40 matches, 3 won by the boss, 37 by the allies; rounds 16 won, 75 lost, 0 drawn`

### gen__sf1plus__ally1-ryu__ally2-birdie__level-2.tsv

- `# boss Gen: 40 matches, 0 won by the boss, 40 by the allies; rounds 7 won, 80 lost, 0 drawn`

### gen__sf1plus__ally1-ryu__ally2-birdie__level-3.tsv

- `# boss Gen: 40 matches, 0 won by the boss, 40 by the allies; rounds 8 won, 80 lost, 0 drawn`

### gen__sf1plus__ally1-ryu__ally2-birdie__level-4.tsv

- `# boss Gen: 40 matches, 8 won by the boss, 32 by the allies; rounds 24 won, 65 lost, 0 drawn`

### birdie__sf1plus__ally1-ryu__ally2-eagle__level-1.tsv

- `# boss Birdie: 40 matches, 5 won by the boss, 35 by the allies; rounds 18 won, 72 lost, 0 drawn`

### birdie__sf1plus__ally1-ryu__ally2-eagle__level-2.tsv

- `# boss Birdie: 40 matches, 6 won by the boss, 34 by the allies; rounds 24 won, 69 lost, 0 drawn`

### birdie__sf1plus__ally1-ryu__ally2-eagle__level-3.tsv

- `# boss Birdie: 40 matches, 6 won by the boss, 34 by the allies; rounds 21 won, 70 lost, 0 drawn`

### birdie__sf1plus__ally1-ryu__ally2-eagle__level-4.tsv

- `# boss Birdie: 40 matches, 4 won by the boss, 36 by the allies; rounds 20 won, 76 lost, 0 drawn`

### eagle__sf1plus__ally1-ryu__ally2-adon__level-1.tsv

- `# boss Eagle: 40 matches, 6 won by the boss, 34 by the allies; rounds 17 won, 71 lost, 0 drawn`

### eagle__sf1plus__ally1-ryu__ally2-adon__level-2.tsv

- `# boss Eagle: 40 matches, 6 won by the boss, 34 by the allies; rounds 21 won, 73 lost, 0 drawn`

### eagle__sf1plus__ally1-ryu__ally2-adon__level-3.tsv

- `# boss Eagle: 40 matches, 12 won by the boss, 28 by the allies; rounds 35 won, 63 lost, 0 drawn`

### eagle__sf1plus__ally1-ryu__ally2-adon__level-4.tsv

- `# boss Eagle: 40 matches, 20 won by the boss, 20 by the allies; rounds 48 won, 47 lost, 0 drawn`

### adon__sf1plus__ally1-ryu__ally2-sagat__level-1.tsv

- `# boss Adon: 40 matches, 6 won by the boss, 34 by the allies; rounds 18 won, 70 lost, 0 drawn`

### adon__sf1plus__ally1-ryu__ally2-sagat__level-2.tsv

- `# boss Adon: 40 matches, 0 won by the boss, 40 by the allies; rounds 11 won, 80 lost, 0 drawn`

### adon__sf1plus__ally1-ryu__ally2-sagat__level-3.tsv

- `# boss Adon: 40 matches, 0 won by the boss, 40 by the allies; rounds 7 won, 80 lost, 0 drawn`

### adon__sf1plus__ally1-ryu__ally2-sagat__level-4.tsv

- `# boss Adon: 40 matches, 0 won by the boss, 40 by the allies; rounds 5 won, 80 lost, 0 drawn`

### sagat__sf1plus__ally1-ryu__ally2-ken__level-1.tsv

- `# boss Sagat: 40 matches, 13 won by the boss, 27 by the allies; rounds 37 won, 61 lost, 0 drawn`

### sagat__sf1plus__ally1-ryu__ally2-ken__level-2.tsv

- `# boss Sagat: 40 matches, 20 won by the boss, 19 by the allies; rounds 50 won, 52 lost, 1 drawn`

### sagat__sf1plus__ally1-ryu__ally2-ken__level-3.tsv

- `# boss Sagat: 40 matches, 14 won by the boss, 26 by the allies; rounds 34 won, 63 lost, 0 drawn`

### sagat__sf1plus__ally1-ryu__ally2-ken__level-4.tsv

- `# boss Sagat: 40 matches, 21 won by the boss, 19 by the allies; rounds 51 won, 47 lost, 0 drawn`

## Provenance

Raw fingerprint:`96ebafdcad3fbd5f145229a3f77a89e4beae9627708867fa750b68af31f53810`. Complete raw bytes, every execution command/exit, stdout digest/payload, manifest, runner/parser and owned frozen build identity were checked. The native exit must be1 iff explicit stopped seeds exist, otherwise0; abort/OOM/access-violation codes are rejected.

Native observation limits remain: rounds/outcomes come from the native health-bar observer rather than the native round-result event; damage and activity retain that observer's attribution limits. No native Arcade/World Tour routing, other-preset gate, parity, human playtest or default enablement approval is implied by this Dramatic sample. Full-suite results are separate. The developer switch remains off pending SC24 and maintainer approval. Fresh independent post-run data audit is required before delivery.

# Roster key art, in the 1987 advertising's two styles

## Intent

Promotional art for *Street Fighter: The World Tournament*, for the top of the README and for posts on social media and forums. It should look the way the 1987 game was advertised, not the way it looks on screen (the maintainer, 2026-11-06). The first piece is the whole cast of twelve together, like the ensemble art of later entries. It is made in both of the 1987 advertising's styles so the maintainer can see which is better (the maintainer). Each bonus-challenger wave gets its own piece later (the maintainer).

The research behind it is `docs/research/reports/Street Fighter 1987 promotional art.md`.

## Goals

- One group portrait (the **key art**) of the twelve tournament fighters: Ryu, Ken, Retsu, Geki, Joe, Mike, Lee, Gen, Birdie, Eagle, Adon, Sagat (the maintainer).
- Two versions of the same composition, the **painted style** and the **cartoon style** (the maintainer):
  - **Painted** (*replaced by the illustrated style; see "Style, revised" below*): gritty and hand-painted, like the export flyer's Ryu and the PC Engine cover. Flame gradient against cool blue, world landmarks behind.
  - **Cartoon:** flat cartoon figures on hot magenta-red-orange-yellow stripes, like the US cabinet's side art and marquee.
- **The arrangement (the maintainer: "Approved"):**
  - Sagat largest, behind everyone in the centre.
  - Ryu and Ken front and centre.
  - The other eight in country pairs on either side, each near its own landmark: Retsu and Geki with Japan, Joe and Mike with the USA, Lee and Gen with China, Birdie and Eagle with England, Adon with Sagat for Thailand.
- **Formats:**
  - A wide banner for the README header.
  - A square crop for posts, from the same picture, with a safe area around Ryu, Ken and Sagat.
- **Text:** none on the art (the maintainer, 2026-11-06; see "Text, revised"). The README and the posts carry the name and the fan-made note.

## Non-goals

- The bonus challengers (later waves get their own pieces).
- Animated or in-game art; pixel art.
- A logo for the project (the art carries no title).
- Printed or sold material.

## Anti-goals (failure even if it looks good)

- Capcom's flame-brush STREET FIGHTER logo or any redraw of it. Capcom's November 2025 Fan Content Guidelines bar the game's logo.
- Capcom's marks: the CAPCOM wordmark or box, a ©CAPCOM notice, the European packaging's striped design.
- Anything that could pass as official Capcom art.
- A recognisable copy of any of the five compositions the report lists, or of later official art such as the Bengus cast paintings or SF2 key art. The original slogans are out too.
- Fighters who don't look like themselves in the 1987 game.

## Constraints

- Fan Content Guidelines: no game or company logo, not official-looking, no direct copying, not sold.
- **The sprite guide stays local.** The layout built from ROM sprites is a working file only. It never goes in the public repository, like the art pack ("no Capcom-derived pixels in the public repository"). Only the finished paintings are published.
- Image generation costs credits. Each round's cost is confirmed with the maintainer before it runs.

## Approaches

1. **Generate from a layout built on the game's own sprites (chosen; the maintainer: "your rec").** Each fighter's ROM sprites and portraits are enlarged and placed in the agreed arrangement as a guide to pose, likeness and colour. An image model (OpenArt) paints over it once per style. Likeness stays tied to the 1987 game rather than to the model's memory of later art.
2. **Generate from a description alone.** Quicker, but Retsu, Mike and Lee would be guesses, and the model leans on later Capcom art for Ryu, Ken and Sagat. That risks the near-copies the anti-goals forbid.
3. **Commission an illustrator.** The most authentic painted style and the cleanest on originality, but real money and weeks, doubled for two styles.

## Decided

- The twelve, the arrangement above, both styles, the two formats, and title lettering with a fan-made line (the plain lettering first decided here is replaced by "Lettering, revised" below).
- **My call:** the key art goes at the very top of the README, above the existing two GIFs, which stay.
- **My call:** the lettering is set in code over the paintings, never by the image model, because models garble text. The banner and the square crop are cut from one master image.
- **My call:** the finished images go in `docs/media/` as `key-art-painted-*.png` and `key-art-cartoon-*.png`. The chosen style's banner goes in the README after the maintainer picks.

## Settled in grilling (the maintainer, 2026-11-06)
- **Terms:** this piece is *key art*, and its versions are the *painted style* and the *cartoon style*. "Look" stays the monitor picture (`CONTEXT.md`).
- **Colours:** both styles use each fighter's 1987 sprite colours, even where the cabinet art recoloured them.
- **Landmarks:** each pair stands before its own 1987 stage's backdrop. The flame-and-Rushmore-behind-Ryu pairing of the export flyer and the PC Engine cover is never used.
- **Poses:**
  - the ten others in their own 1987 fighting stances;
  - Ryu and Ken mid-move up front, a Hadoken and a Shoryuken;
  - Sagat looming still behind.
- **Sizes:**
  - a 2:1 master at 2560×1280, which is also the repository's social preview (GitHub's size; the maintainer sets it in the repository settings, since the API can't upload it);
  - a 1080×1080 square cut from it for posts.
- **Both styles are kept** in `docs/media/`. The README shows the one the maintainer picks, and posts can use either.

## Left to the builder

- The model and its settings on OpenArt, the prompts, and how many rounds to run, each round within a cost the maintainer confirmed.
- How the sprite guide is built: which poses and portraits, and the scaling.
- The lettering's exact face, colours and placement within the period style.
- Retouching the generated images: fixing hands, faces and costume details against the sprites.

## Lettering, revised (the maintainer, 2026-11-06)

The maintainer saw the first lettered banner and replaced the plain condensed-italic lettering: "Make it styled more like the street fighter logo, but just make it say the world tournament."

- **Text:** the title is *THE WORLD TOURNAMENT* alone. STREET FIGHTER is dropped from the art. The "Fan-made · not affiliated with Capcom" line stays as it was.
- **Style:** jagged, torn-edged brush-stroke capitals with slightly irregular sizes and a forward lean, a hot orange-to-yellow flame gradient, a dark red-brown inner edge, a heavy black outline and a black drop shadow.
- **Our own letters:** `tools/key_art/letter.py` (now `formats.py --lettering`) built them procedurally from a heavy condensed system font (Impact): wobbled outlines, zigzag tears and spikes at the edges and stroke ends, and dry-brush streaks running back from the stroke ends, from a fixed seed in `tools/key_art/layout.py`. Nothing traces, redraws or samples Capcom's letterforms or any image of their logo.
- **Layout:** the banner sets the title on one line, which reads bolder at README width than a stack in the same band; the square stacks *THE* over *WORLD TOURNAMENT*.
- **Risk, noted:** the research report warns that any jagged red-to-yellow brush wordmark with a black drop shadow may read as the property logo, which the Fan Content Guidelines bar. This revision takes that style on deliberately, keeping the risk down with different words, our own procedurally built letters and the visible fan-made line. It replaces the plain-lettering decision above; the anti-goal against Capcom's logo or a redraw of it still stands.

## Style, revised (the maintainer, 2026-11-06, after two painted rounds)

The maintainer's verdict on two painted rounds: "The look just isn't landing with me." He pointed instead at the early Ryu character illustration: pencil linework with watercolour and marker washes on white paper.

- **The illustrated style replaces the painted style** (the maintainer). It's late-1980s Japanese character-illustration work:
  - pencil and ink lines with loose hatching;
  - transparent watercolour and marker washes;
  - lavender and blue-violet shadows;
  - white paper showing through.
- **The poses are the guide's own.** We never use that illustration's pose or composition. It is described to the model in words and never uploaded as a reference (*my call*, so the result can't copy it).
- **Backgrounds are full-colour scenes** (the maintainer). The four side pairs sit in framed, slightly tilted vignettes, and everything is drawn in the same line-and-wash style. Off-white paper shows between the panels.
- **The cartoon style (KA2) still follows**, for comparison (the maintainer).
- **Rounds 1 and 2 (painted)** stay as local drafts. Their lessons carry over:
  - Ken keeps his back to the viewer and his head turned away;
  - Geki keeps his three-bladed claw;
  - Ken stands clear of Sagat;
  - the side scenes are framed vignettes.

## Text, revised (the maintainer, 2026-11-06)

After a round in the illustrated style, the maintainer dropped the title lettering and chose no text at all on the art. This replaces "Lettering, revised" above and the lettering in "Decided".

- **No text on the art:** neither the title nor the fan-made line. The banner and the square are the master and its safe-area cut as they are.
- **The fan-made note moves** to the README, beside the banner, and into the text of each post, so the project still says plainly that it is fan-made and not affiliated with Capcom, keeping within Capcom's guidelines on not looking official.
- **The tooling:** `tools/key_art/formats.py` (formerly `letter.py`) writes both formats with no lettering by default. The torn-brush lettering stays behind its `--lettering` flag, unused.

## Cartoon style dropped; the cast's illustrations as references (the maintainer, 2026-11-06)

- **The cartoon style is dropped** after one round (the maintainer: "Drop the cartoon style"). KA2 ends without a master, and the illustrated style is the key art. Round 1 stays as a local draft.
- **the maintainer supplied a matching illustration of each of the twelve,** in the same pencil-and-wash style, in the local `key-art/reference/` folder. They are used as likeness and technique references for the illustrated key art, put together on one sheet and never committed. The poses still come from the sprite guide.
- **They are never used for game art.** ADR-0006 keeps fighter poses to the ROM's own frames.

## The chosen master (the maintainer, 2026-11-06)

The key art is illustrated round 5. It was drawn with the cast's illustrations as references, then given a colour pass: reds and warm tones pushed up (saturation doubled for hues under 20° or over 330°, value down 7%), and all colour up by a quarter. The maintainer picked this version over round 4 and over a narrower red-only fix.

The pass also warms the skin a little, and Ken's gi reads red-pink rather than full red. Both are noted on the art-upgrades list.

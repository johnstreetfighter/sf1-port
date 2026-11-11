# Throws, specials and kick-button moves: what each looks like

AP2 (#166) for spec #164. Before any generation is spent on these moves, the user approves:

- one line on each fighter's ground throw;
- one line on each special that has no Capcom original to follow;
- which existing ROM streams stand in for named specials;
- what Mike's and Eagle's kick buttons do instead of kicking.

**Rules for these descriptions:**
- A line describes what the move looks like, so a pose list can be drawn from it. Its timing and damage are the SF1+ project's.
- Inputs and frame data come from the earlier batch's SF1+ list. What a move does may also draw on the fan M.U.G.E.N research notes; how it was drawn may not (ADR-0006).
- Every throw's victim uses his own generic being-thrown set (grilling, item 2), so none is described here.
- **Stand-ins:** where a ROM stream is named as standing in for a special, that special needs no new art; the SF1+ project re-times it.
- **Projectiles and effects are out of this project.** The bodies of projectile specials are drawn here. A projectile or effect that does not already exist in the ROM is flagged for the SF1+ project.

**Status:** approved by the user on 2026-11-02, as written.

## Ryu and Ken

- **Throw (both):** a shoulder throw. He grabs the collar and sleeve, turns his back and throws the opponent over his shoulder to land in front of him. Ken's poses are Ryu's, recoloured, with Ken's hair and face (decision 5).
- **Specials:** Hadoken, Shoryuken and Tatsumaki are their ROM art. Nothing new.

## Retsu

- **Throw:** a kempo wrist throw. He seizes the opponent's wrist, steps across and flips him over onto his back in front of him.
- **Kiai Shot** (quarter-circle forward + punch; no Capcom original): he draws both palms to his hip, then thrusts them forward at chest height, back leg straight, as a short burst of ki leaves his hands. *New:* 3 to 4 body poses. **Flag:** the ki burst is a new effect, so it is for the SF1+ project.

## Geki

- **Throw:** a circle throw. He grabs the opponent, falls back, plants a foot in his stomach and kicks him over his head, rolling back up.
- **Shuriken** (quarter-circle forward + punch): his ROM hop-and-throw, stream `0xbf98`, with its stars. Nothing new.
- **Teleport Strike** (quarter-circle back + punch): his ROM teleport (moves 4 and 8) into his ROM claw thrust `0x0b968`, reappearing beside the opponent. **Proposed stand-in:** nothing new, if the claw thrust reads well as the strike after the teleport.

## Joe

- **Throw:** a hip toss. He grabs the opponent's waist, loads him onto his hip and throws him over to land in front.
- **Rolling Sobat** (quarter-circle back + kick; no Capcom original): a spinning back kick. He turns his back, hops off the front foot and whips the rear heel round at head height, landing facing forward. *New:* 4 to 5 poses.
- **Dynamite Straight** (dragon punch + punch; no Capcom original): he dips low and springs up into an upward-angled right straight, rear foot leaving the ground at the top. *New:* 3 to 4 poses; his ROM straight `0x0e44a` can guide the arm.

## Mike

- **Throw:** a clinch headbutt. He wraps the opponent in a boxer's clinch, headbutts him once and shoves him away.
- **Bison Rush** (quarter-circle forward + punch; no Capcom original): a dashing straight. He drops into a low boxer's crouch, charges forward and finishes with a full-extension right straight. *New:* 3 to 4 poses; his ROM lunging hook `0x0db1c` can guide the lunge.
- **Iron Upper** (dragon punch + punch; no Capcom original): a rising uppercut. He crouches, then drives upward with a full-body right uppercut, both feet briefly off the ground. *New:* 3 to 4 poses.
- **Kick buttons** (no kicks, decision 6):
  - **Standing:** LK a short body jab; MK a left hook to the body; HK an overhand right.
  - **Crouching:** LK a low jab; MK a low body hook; HK a crouching uppercut.
  - **Jumping:** LK a downward jab; MK a jumping hook; HK a two-fisted hammer.

  With his three punch buttons, these give him Ryu's number of distinct animations, all punches.

## Lee

- **Throw:** a kung fu over-the-back throw. He grabs both of the opponent's arms, turns under him and throws him over his back.
- **Zesshouohken** (quarter-circle forward + punch; no Capcom original, since Lee never appeared again): a double-palm strike. He steps in and drives both open palms into the opponent's chest, one above the other. *New:* 3 poses; his ROM palm `0x0f366` can guide the hands.
- **Double Kick** (quarter-circle back + kick; no Capcom original): his ROM side kick `0x0f5c8` straight into his ROM high kick `0x0f678`, stepping forward. **Proposed stand-in:** nothing new, pieced from the two ROM kicks.

## Gen

- **Throw:** a palm throw. He grabs the opponent's shoulder, spins him round and sends him away with a palm strike to the back.
- **Hyakurenko** (quarter-circle forward + punch): a rapid open-hand flurry. **Proposed stand-in:** his ROM arm flurry `0x10b4e`. Nothing new.
- **Gekiro** (dragon punch + punch): a rising somersault of kicks. He springs up, flipping, with kicks spun upward, and lands crouched. *New:* 4 to 5 poses; Ryu's Tatsumaki and Shoryuken can guide the airborne poses.

## Birdie

- **Throw:** a head slam. He grabs the opponent by the head and slams him down in front of him with both hands.
- **Bull Head** (quarter-circle forward + punch): a charging headbutt. **Proposed stand-in:** his ROM lunging headbutt `0x11b42`. Nothing new.
- **Bull Horn** (quarter-circle back + kick): a low charge. He lowers his head, puts both fists forward like horns and drives in at waist height. *New:* 3 poses; the headbutt can guide the body.

## Eagle

- **Throw:** a stick hook. He hooks a stick behind the opponent's neck and flings him over to land in front.
- **Manchester Black** (quarter-circle forward + punch): a stick barrage. **Proposed stand-in:** his ROM spinning-sticks barrage `0x124d8`. Nothing new.
- **St. Andrew's Green** (quarter-circle back + kick): a leaping stick strike. **Proposed stand-in:** his ROM leaping attack `0x129bc`. Nothing new.
- **Kick buttons** (no kicks, decision 6):
  - **Standing:** LK a low stick poke; MK a rising stick strike; HK a two-stick overhead smash.
  - **Crouching:** LK a low stick sweep; MK a low two-stick thrust; HK a rising stick uppercut.
  - **Jumping:** LK a downward stick poke; MK a horizontal stick swing; HK a two-stick downward smash.

  All are stick moves.

## Adon

- **Throw:** a clinch knee. He grabs the opponent's head in a Muay Thai clinch, drives a knee into his body and shoves him away.
- **Jaguar Kick** (quarter-circle back + kick): an arcing flying kick. **Proposed stand-in:** his ROM flying kick, if it really leaves the ground. The inventory found that his "leaping" kicks keep `dy` 0, so this needs checking. If it doesn't, the kick is *new:* 4 poses.
- **Rising Jaguar** (dragon punch + punch): a rising knee. He springs straight up with one knee driven high and both fists by his face. *New:* 3 poses.

## Sagat

- **Throw:** a tiger clinch. He grabs the opponent's head with both hands, drives a knee into him and throws him down in front.
- **High Tiger Shot** and **Tiger Knee:** his ROM art (`0x144b4`, `0x14242`). Nothing new.
- **Low Tiger Shot** (quarter-circle forward + kick): from a deep crouch he thrusts both arms forward at shin height. The shot is his ROM projectile, launched lower. *New:* 2 to 3 body poses, built on his new crouch.

## Summary

| | New specials | Proposed ROM stand-ins | New effects for SF1+ |
| --- | --- | --- | --- |
| Count | Kiai Shot, Rolling Sobat, Dynamite Straight, Bison Rush, Iron Upper, Zesshouohken, Gekiro, Bull Horn, Rising Jaguar, low Tiger Shot, and Jaguar Kick unless its stand-in holds | Shuriken, Teleport Strike, Double Kick (pieced), Hyakurenko, Bull Head, Manchester Black, St. Andrew's Green, and Jaguar Kick if it holds | the Kiai Shot burst |

Twelve throws, one per fighter (Ken's from Ryu's), all new.

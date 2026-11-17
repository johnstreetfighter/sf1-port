use super::fight::FightKind;
use super::{
    draw::{fit, framed_at},
    Transition,
};
use crate::replay::MatchReplay;
use crate::roster::MenuInput;
use sf_video::{
    compositor::{ink, RomFont},
    render::Frame,
};

const ROWS: usize = 8;

#[derive(Clone, Default)]
pub(super) struct ReplaysScreen {
    cursor: usize,
}

impl ReplaysScreen {
    pub fn apply(&mut self, input: MenuInput, count: usize) -> Option<Transition> {
        match input {
            MenuInput::Back => Some(Transition::Back),
            MenuInput::Up | MenuInput::Down if count != 0 => {
                self.cursor = (self.cursor
                    + if input == MenuInput::Down {
                        1
                    } else {
                        count - 1
                    })
                    % count;
                Some(Transition::CursorMoved)
            }
            MenuInput::Confirm | MenuInput::Start if self.cursor < count => {
                Some(Transition::WatchReplay(self.cursor))
            }
            _ => None,
        }
    }

    pub(super) fn render_with_title(
        &self,
        font: &RomFont<'_>,
        replays: impl ExactSizeIterator<Item = impl AsRef<MatchReplay>>,
        notice: Option<&str>,
        title_offset: usize,
    ) -> Frame {
        let mut frame = framed_at(font, "MATCH REPLAYS", "", title_offset);
        if replays.len() == 0 {
            font.text_centred(&mut frame.argb, 11, "NO SAVED MATCHES", ink::BLUE);
        }
        let start = self.cursor / ROWS * ROWS;
        for (i, item) in replays.skip(start).take(ROWS).enumerate() {
            let replay = item.as_ref();
            let color = if start + i == self.cursor {
                ink::YELLOW
            } else {
                ink::BLUE
            };
            let seconds = replay.frames.len() / 60;
            let row = 6 + i * 2;
            let mark = if start + i == self.cursor { '>' } else { ' ' };
            let title = fit(&replay.title.to_ascii_uppercase(), 21);
            let line = format!(
                "{mark}{:02} {:21} {:02}:{:02}",
                start + i + 1,
                title,
                seconds / 60,
                seconds % 60
            );
            font.text(&mut frame.argb, 7, row, &line, color);
            let mode = match replay.kind {
                FightKind::Arcade => "ARCADE",
                FightKind::Local if replay.online => "ONLINE",
                FightKind::Local => "VERSUS",
                FightKind::Dramatic { .. } if replay.online => "ONLINE 2V1",
                FightKind::Dramatic { .. } => "2V1",
                FightKind::TrainingCpu | FightKind::TrainingDummy => "TRAINING",
                FightKind::WorldTour => "WORLD TOUR",
            };
            let styles = if let Some(rules) = replay.config.dramatic {
                let [ally1, boss, ally2] = rules.styles;
                format!(
                    "{}+{} VS {}  {mode}",
                    ally1.name(),
                    ally2.name(),
                    boss.name()
                )
            } else if replay.kind == FightKind::Arcade {
                format!("{}  {mode}", replay.p1_style.revision.name())
            } else {
                format!(
                    "{} VS {}  {mode}",
                    replay.p1_style.revision.name(),
                    replay.p2_style.revision.name()
                )
            };
            font.text(
                &mut frame.argb,
                11,
                row + 1,
                &fit(&styles.to_ascii_uppercase(), 28),
                ink::BLUE,
            );
        }
        let help = notice.unwrap_or("CONFIRM WATCH    BACK MENU");
        font.text_centred(&mut frame.argb, 23, &fit(help, 36), ink::YELLOW);
        font.text_centred(
            &mut frame.argb,
            25,
            "L/R SEEK  TAB L/R  START PAUSE",
            ink::BLUE,
        );
        frame
    }
}

//! Touch and pen: what several pointers at once mean. One finger uses the tool; two fingers pan and zoom the view and never edit;
//! a finger that lands while the pen is at work is a palm and is ignored; a finger or a pen held still for half a second asks for
//! the context menu and, over something, selects what is
//! under it; the eraser end of a pen erases. The page forwards every pointer with its id and its time stamp and decides nothing.

use crate::editor::{
    Editor, Hit, Tool, CHANGED, HANDLED, MENU, MIDDLE, MOUSE, PEN, REPAINT, SECONDARY, TOUCH,
};
use crate::m;

/// Phases of a pointer event.
pub const DOWN: u32 = 0;
pub const MOVE: u32 = 1;
pub const UP: u32 = 2;
/// The system took the pointer away (a call, a system gesture): nothing is committed.
pub const CANCEL: u32 = 3;

/// Modifier bit: the eraser end or eraser button of a pen.
pub const ERASER: u32 = 4;

/// A finger within this many milliseconds of the last pen event is a resting palm.
const PALM_MS: f64 = 500.0;
/// A finger held still this long selects instead of using the tool.
const LONG_PRESS_MS: f64 = 500.0;

/// A press of the middle or the secondary button of a mouse: it pans the view when it is dragged and, for the secondary button, asks
/// for the context menu when it is released without having moved.
#[derive(Clone, Debug)]
struct Aux {
    start: (f64, f64),
    last: (f64, f64),
    moved: bool,
    menu: bool,
}

/// A mouse press with the middle or the secondary button is a drag after this many pixels.
const AUX_SLOP: f64 = 4.0;

/// The pointers at work and what they are doing together.
#[derive(Clone, Debug, Default)]
pub struct Gesture {
    /// The fingers on the canvas: id and position in pixels (at most two are followed).
    touches: Vec<(u32, f64, f64)>,
    /// Fingers taken for a palm: ignored until they lift.
    palms: Vec<u32>,
    /// Two fingers are (or were, until all lift) on the canvas: the view moves, nothing is drawn.
    pinch: bool,
    /// Time of the last pen event, and whether the pen is pressed.
    pen: Option<f64>,
    pen_down: bool,
    /// When the single finger went down.
    down_at: f64,
    /// The tool to come back to after a stroke with the eraser end of the pen.
    before_eraser: Option<Tool>,
    /// The press of the middle or secondary button of the mouse in progress.
    aux: Option<Aux>,
}

impl Editor {
    /// One pointer event: `phase` DOWN / MOVE / UP / CANCEL, the pointer's `id`, its position in pixels, its `kind` (mouse, pen,
    /// finger), the modifier bits and the time stamp `t` in milliseconds. Returns the flags of the editor.
    #[allow(clippy::too_many_arguments)]
    pub fn pointer_event(
        &mut self,
        phase: u32,
        id: u32,
        x: f64,
        y: f64,
        kind: u32,
        mods: u32,
        t: f64,
    ) -> u32 {
        if let Some(f) = self.proposal_pointer(phase, id, x, y, kind, mods) {
            return f;
        }
        let f = match kind {
            TOUCH => self.touch(phase, id, x, y, mods, t),
            PEN => self.pen(phase, x, y, mods, t),
            _ => self.single(phase, x, y, kind, mods),
        };
        f | self.settle_proposal()
    }

    /// One pointer on its own, as the tools take it.
    fn single(&mut self, phase: u32, x: f64, y: f64, kind: u32, mods: u32) -> u32 {
        if kind == MOUSE
            && (self.gesture.aux.is_some() || (phase == DOWN && mods & (SECONDARY | MIDDLE) != 0))
        {
            return self.aux_press(phase, x, y, mods);
        }
        match phase {
            DOWN => self.pointer_down(x, y, kind, mods),
            MOVE => self.pointer_move(x, y, kind),
            UP => self.pointer_up(x, y),
            _ => self.cancel_press(),
        }
    }

    /// The middle and the secondary button of the mouse: a drag pans the view, a click of the secondary button asks for the menu.
    fn aux_press(&mut self, phase: u32, x: f64, y: f64, mods: u32) -> u32 {
        match phase {
            DOWN => {
                self.cancel_press();
                self.gesture.aux = Some(Aux {
                    start: (x, y),
                    last: (x, y),
                    moved: false,
                    menu: mods & SECONDARY != 0,
                });
                HANDLED
            }
            MOVE => {
                let Some(a) = self.gesture.aux.as_mut() else {
                    return 0;
                };
                a.moved |= m::hypot(x - a.start.0, y - a.start.1) > AUX_SLOP;
                if !a.moved {
                    return HANDLED;
                }
                let (dx, dy) = (x - a.last.0, y - a.last.1);
                a.last = (x, y);
                self.pan(dx, dy) | HANDLED
            }
            UP => match self.gesture.aux.take() {
                Some(a) if a.menu && !a.moved => MENU | HANDLED | REPAINT,
                _ => HANDLED,
            },
            _ => {
                self.gesture.aux = None;
                HANDLED
            }
        }
    }

    /// Drops a press in progress without committing anything.
    pub fn cancel_press(&mut self) -> u32 {
        let had = self.press.take().is_some();
        self.over.drag = (0.0, 0.0);
        self.over.preview = None;
        self.over.band = None;
        if had {
            REPAINT
        } else {
            0
        }
    }

    /// A press held still for `LONG_PRESS_MS` (finger or pen) is not a use of the tool: it selects what is under it when that is
    /// something and the tool is not Select, and asks for the context menu (`MENU`). `None`: not a long press.
    fn long_press(&mut self, t: f64) -> Option<u32> {
        let p = self.press.as_ref()?;
        if p.moved || t - self.gesture.down_at < LONG_PRESS_MS {
            return None;
        }
        let hit = p.hit;
        self.cancel_press();
        let mut f = REPAINT | HANDLED | MENU;
        if hit != Hit::None && self.tool != Tool::Select {
            // a long press selects what is under the finger, whatever the tool
            self.select_hit(hit);
            self.said = ("draw2.say.selected", String::new());
            f |= CHANGED;
        }
        Some(f)
    }

    fn pen(&mut self, phase: u32, x: f64, y: f64, mods: u32, t: f64) -> u32 {
        self.gesture.pen = Some(t);
        // the barrel button asks for the menu and does not use the tool
        if phase == DOWN && mods & SECONDARY != 0 && mods & ERASER == 0 {
            self.cancel_press();
            return MENU | HANDLED | REPAINT;
        }
        match phase {
            DOWN => {
                self.gesture.pen_down = true;
                self.gesture.down_at = t;
                // the pen takes over: fingers already down were a hand coming to rest
                let resting: Vec<u32> = self.gesture.touches.drain(..).map(|p| p.0).collect();
                if !resting.is_empty() {
                    self.gesture.palms.extend(resting);
                    self.gesture.pinch = false;
                    self.cancel_press();
                }
                if mods & ERASER != 0 && self.gesture.before_eraser.is_none() {
                    self.gesture.before_eraser =
                        Some(std::mem::replace(&mut self.tool, Tool::Erase));
                }
            }
            UP | CANCEL => self.gesture.pen_down = false,
            _ => {}
        }
        // a pen held still (not its eraser end) asks for the menu as a finger does
        if phase == UP
            && self.gesture.before_eraser.is_none()
            && let Some(f) = self.long_press(t)
        {
            return f;
        }
        let f = self.single(phase, x, y, PEN, mods);
        if matches!(phase, UP | CANCEL)
            && let Some(tool) = self.gesture.before_eraser.take()
        {
            self.tool = tool;
        }
        f
    }

    fn touch(&mut self, phase: u32, id: u32, x: f64, y: f64, mods: u32, t: f64) -> u32 {
        let g = &mut self.gesture;
        if let Some(i) = g.palms.iter().position(|&p| p == id) {
            if matches!(phase, UP | CANCEL) {
                g.palms.remove(i);
            }
            return 0;
        }
        match phase {
            DOWN => {
                let near_pen = g.pen_down || g.pen.is_some_and(|p| t - p < PALM_MS && t >= p);
                if near_pen {
                    g.palms.push(id);
                    return 0;
                }
                if g.touches.len() >= 2 {
                    // a third finger: not followed
                    g.palms.push(id);
                    return 0;
                }
                g.touches.push((id, x, y));
                if g.touches.len() == 2 {
                    // the second finger: what the first one was doing is dropped, the view moves from now on
                    g.pinch = true;
                    return self.cancel_press() | REPAINT | HANDLED;
                }
                g.down_at = t;
                self.pointer_down(x, y, TOUCH, mods)
            }
            MOVE => {
                let Some(i) = g.touches.iter().position(|p| p.0 == id) else {
                    return 0;
                };
                if !g.pinch {
                    g.touches[i] = (id, x, y);
                    return self.pointer_move(x, y, TOUCH);
                }
                if g.touches.len() < 2 {
                    return 0;
                }
                let before = pair(&g.touches);
                g.touches[i] = (id, x, y);
                let after = pair(&g.touches);
                self.pan(after.0 - before.0, after.1 - before.1);
                if before.2 > 1.0 && after.2 > 1.0 {
                    self.zoom(after.0, after.1, after.2 / before.2);
                }
                REPAINT | HANDLED
            }
            _ => {
                let Some(i) = g.touches.iter().position(|p| p.0 == id) else {
                    return 0;
                };
                g.touches.remove(i);
                if g.pinch {
                    // the view gesture ends when the last finger lifts; a finger left over does not start to draw
                    if g.touches.is_empty() {
                        g.pinch = false;
                    }
                    return REPAINT | HANDLED;
                }
                if phase == CANCEL {
                    return self.cancel_press();
                }
                if let Some(f) = self.long_press(t) {
                    return f;
                }
                self.pointer_up(x, y)
            }
        }
    }
}

/// The middle of two fingers and their distance.
fn pair(t: &[(u32, f64, f64)]) -> (f64, f64, f64) {
    let (a, b) = (t[0], t[1]);
    (
        (a.1 + b.1) / 2.0,
        (a.2 + b.2) / 2.0,
        m::hypot(a.1 - b.1, a.2 - b.2),
    )
}

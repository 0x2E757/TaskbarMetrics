//! Tray-style reordering of taskbar tiles: press a tile and drag it past a
//! small threshold; the neighbours slide aside and releasing drops it there.
use super::{events::Subscription, *};
use std::rc::Rc;

const TRANSLATE: Guid = Guid::from_u128(0xc975905c_3c36_4229_817b_178f64c0e113);
const STORYBOARD: Guid = Guid::from_u128(0xd45c1e6e_3594_460e_981a_32271bd3aa06);
const DOUBLE_ANIMATION: Guid = Guid::from_u128(0x7e9f3d59_0f07_4bc9_977d_03763ff8154f);
const ELEMENT_COLLECTION: Guid = Guid::from_u128(0xd6602d54_88f6_43f6_85d8_a9d914a6dd3b);
const CANVAS_STATICS: Guid = Guid::from_u128(0x40ce5c46_2962_446f_aafb_4cdc486939c9);
const PROPERTY_VALUE: Guid = Guid::from_u128(0x629bdbc8_d932_4ff4_96b9_8d96c5c1e858);
const DOUBLE_REFERENCE: Guid = Guid::from_u128(0x2f2d6c29_5473_5f3e_92e7_96572bb990e2);
/// Pointer travel before a press becomes a drag, as SM_CXDRAG.
const THRESHOLD: f64 = 4.0;
/// Receives the tile ids after a drop changed their order.
pub(super) type SaveOrder = Rc<dyn Fn(&[String])>;

/// Tile slots along the strip, independent of XAML.
#[derive(Clone, Debug, PartialEq)]
struct Strip {
    widths: Vec<f64>,
    gap: f64,
    /// Tiles on view, from the first; the rest are left out for lack of room.
    shown: usize,
}
impl Strip {
    fn left(&self, index: usize) -> f64 {
        self.widths[..index]
            .iter()
            .map(|width| width + self.gap)
            .sum()
    }
    fn center(&self, index: usize) -> f64 {
        self.left(index) + self.widths[index] / 2.0
    }
    /// The tile under `x`; gaps belong to no tile.
    fn hit(&self, x: f64) -> Option<usize> {
        (0..self.shown).find(|&i| (self.left(i)..self.left(i) + self.widths[i]).contains(&x))
    }
    /// Keeps the dragged tile inside the strip.
    fn clamp(&self, from: usize, offset: f64) -> f64 {
        let last = self.shown - 1;
        let end = self.left(last) + self.widths[last];
        offset.clamp(-self.left(from), end - self.left(from) - self.widths[from])
    }
    /// The slot the dragged tile takes: past every neighbour whose centre its
    /// leading edge crossed, so a wide tile can pass a narrow one at the ends.
    fn target(&self, from: usize, offset: f64) -> usize {
        let left = self.left(from) + offset;
        let right = left + self.widths[from];
        let after = (from + 1..self.shown)
            .filter(|&i| right > self.center(i))
            .count();
        let before = (0..from).filter(|&i| left < self.center(i)).count();
        from + after - before
    }
    /// How far tile `index` steps aside while `from` hovers over slot `to`.
    fn shift(&self, from: usize, to: usize, index: usize) -> f64 {
        let step = self.widths[from] + self.gap;
        if from < to && (from + 1..=to).contains(&index) {
            -step
        } else if to < from && (to..from).contains(&index) {
            step
        } else {
            0.0
        }
    }
    fn moved(&self, from: usize, to: usize) -> Self {
        let mut widths = self.widths.clone();
        let width = widths.remove(from);
        widths.insert(to, width);
        Self {
            widths,
            ..self.clone()
        }
    }
    /// The dragged tile's new slot relative to its old one.
    fn landing(&self, from: usize, to: usize) -> f64 {
        self.moved(from, to).left(to) - self.left(from)
    }
}

/// A tile's horizontal offset and the animation that eases it.
pub(super) struct Slide {
    element: Com,
    shift: Com,
    storyboard: Com,
    animation: Com,
}
impl Slide {
    /// `shift` is the tile's TranslateTransform, `storyboard` animates its X.
    pub fn new(element: &Com, shift: &Com, storyboard: &Com) -> Result<Self> {
        let storyboard = storyboard.query(&STORYBOARD)?;
        let children = storyboard.object(6)?;
        let animation = unsafe {
            let get: unsafe extern "system" fn(Raw, u32, *mut Raw) -> Hr = children.slot(6);
            let mut raw = std::ptr::null_mut();
            check(get(children.raw(), 0, &mut raw))?;
            Com::owned(raw)?.query(&DOUBLE_ANIMATION)?
        };
        Ok(Self {
            element: element.query(&UI_ELEMENT)?,
            shift: shift.query(&TRANSLATE)?,
            storyboard,
            animation,
        })
    }
    fn offset(&self) -> Result<f64> {
        XamlElement(self.shift.clone()).number(6)
    }
    /// Jumps to `x`, dropping any running or held animation.
    fn set(&self, x: f64) -> Result<()> {
        self.call(8)?;
        XamlElement(self.shift.clone()).set_number(7, x)
    }
    /// Eases from wherever the tile is now to `x`.
    fn glide(&self, x: f64) -> Result<()> {
        self.set(self.offset()?)?;
        unsafe {
            let factory = factory("Windows.Foundation.PropertyValue", &PROPERTY_VALUE)?;
            let create: unsafe extern "system" fn(Raw, f64, *mut Raw) -> Hr = factory.slot(15);
            let mut value = std::ptr::null_mut();
            check(create(factory.raw(), x, &mut value))?;
            let value = Com::owned(value)?.query(&DOUBLE_REFERENCE)?;
            let put: unsafe extern "system" fn(Raw, Raw) -> Hr = self.animation.slot(9);
            check(put(self.animation.raw(), value.raw()))?;
        }
        self.call(9)
    }
    /// Draws the dragged tile above the neighbours it passes.
    fn raise(&self, raised: bool) -> Result<()> {
        unsafe {
            let canvas = factory("Windows.UI.Xaml.Controls.Canvas", &CANVAS_STATICS)?;
            let set: unsafe extern "system" fn(Raw, Raw, i32) -> Hr = canvas.slot(14);
            check(set(canvas.raw(), self.element.raw(), i32::from(raised)))
        }
    }
    /// Storyboard Stop (8) and Begin (9).
    fn call(&self, slot: usize) -> Result<()> {
        unsafe {
            let call: unsafe extern "system" fn(Raw) -> Hr = self.storyboard.slot(slot);
            check(call(self.storyboard.raw()))
        }
    }
}

enum Gesture {
    Idle,
    Pressed {
        index: usize,
        x: f64,
    },
    Dragging {
        index: usize,
        x: f64,
        offset: f64,
        target: usize,
    },
}
#[derive(PartialEq)]
enum Step {
    Ignored,
    Started,
    Dragged,
}

/// The gesture over the tiles of one strip, in their current order.
struct Order {
    strip: Strip,
    slides: Vec<Slide>,
    ids: Vec<String>,
    gesture: Gesture,
    children: Com,
    save: SaveOrder,
}
impl Order {
    fn press(&mut self, x: f64) {
        self.gesture = match self.strip.hit(x) {
            Some(index) => Gesture::Pressed { index, x },
            None => Gesture::Idle,
        };
    }
    fn drag(&mut self, x: f64) -> Result<Step> {
        let step = match self.gesture {
            Gesture::Idle => return Ok(Step::Ignored),
            Gesture::Pressed { index, x: start } if (x - start).abs() >= THRESHOLD => {
                for (i, slide) in self.slides.iter().enumerate() {
                    slide.raise(i == index)?;
                }
                self.gesture = Gesture::Dragging {
                    index,
                    x: start,
                    offset: 0.0,
                    target: index,
                };
                Step::Started
            }
            Gesture::Pressed { .. } => return Ok(Step::Ignored),
            Gesture::Dragging { .. } => Step::Dragged,
        };
        let Gesture::Dragging {
            index,
            x: start,
            offset,
            target,
        } = &mut self.gesture
        else {
            return Ok(step);
        };
        *offset = self.strip.clamp(*index, x - *start);
        self.slides[*index].set(*offset)?;
        let next = self.strip.target(*index, *offset);
        if next != *target {
            *target = next;
            for (i, slide) in self.slides.iter().enumerate() {
                if i != *index {
                    slide.glide(self.strip.shift(*index, next, i))?;
                }
            }
        }
        Ok(step)
    }
    /// Drops the dragged tile into its slot; true when a drag ended.
    fn release(&mut self) -> Result<bool> {
        let Gesture::Dragging {
            index,
            offset,
            target,
            ..
        } = std::mem::replace(&mut self.gesture, Gesture::Idle)
        else {
            return Ok(false);
        };
        if target == index {
            self.slides[index].glide(0.0)?;
            return Ok(true);
        }
        // The neighbours already stand where the new layout puts them; only
        // the dropped tile eases the rest of the way into its slot.
        unsafe {
            let collection = self.children.query(&ELEMENT_COLLECTION)?;
            let call: unsafe extern "system" fn(Raw, u32, u32) -> Hr = collection.slot(6);
            check(call(collection.raw(), index as u32, target as u32))?;
        }
        for (i, slide) in self.slides.iter().enumerate() {
            if i != index {
                slide.set(0.0)?;
            }
        }
        self.slides[index].set(offset - self.strip.landing(index, target))?;
        self.slides[index].glide(0.0)?;
        let slide = self.slides.remove(index);
        self.slides.insert(target, slide);
        let id = self.ids.remove(index);
        self.ids.insert(target, id);
        self.strip = self.strip.moved(index, target);
        (self.save)(&self.ids);
        Ok(true)
    }
    fn cancel(&mut self) -> Result<()> {
        if let Gesture::Dragging { .. } = std::mem::replace(&mut self.gesture, Gesture::Idle) {
            for slide in &self.slides {
                slide.glide(0.0)?;
            }
        }
        Ok(())
    }
}

/// Pointer handling on the strip. Handlers attach to the strip itself so they
/// also see presses the tile buttons already handled.
#[derive(Clone)]
pub(super) struct TileReorder {
    order: Rc<RefCell<Order>>,
    _events: Vec<Subscription>,
}
impl TileReorder {
    /// `dragging` is raised while a drag lasts, so tiles can skip their click.
    pub fn new(
        strip: &Com,
        slides: Vec<Slide>,
        ids: Vec<String>,
        widths: Vec<f64>,
        gap: f64,
        dragging: Rc<Cell<bool>>,
        save: SaveOrder,
    ) -> Result<Self> {
        let order = Rc::new(RefCell::new(Order {
            strip: Strip {
                shown: widths.len(),
                widths,
                gap,
            },
            slides,
            ids,
            gesture: Gesture::Idle,
            children: strip.query(&PANEL)?.object(6)?,
            save,
        }));
        // A handler finding the order busy is re-entered from our own capture
        // or release call and has nothing to add.
        let pressed = order.clone();
        let moved = order.clone();
        let released = order.clone();
        let (lost, canceled) = (order.clone(), order.clone());
        let (drag_flag, release_flag, lost_flag, cancel_flag) = (
            dragging.clone(),
            dragging.clone(),
            dragging.clone(),
            dragging,
        );
        let events = vec![
            Subscription::pointer_handled(strip, 9, move |sender, args| {
                if Subscription::primary(sender, args)? {
                    let (x, _) = Subscription::position(sender, args)?;
                    if let Ok(mut order) = pressed.try_borrow_mut() {
                        order.press(x as f64);
                    }
                }
                Ok(())
            })?,
            Subscription::pointer_handled(strip, 10, move |sender, args| {
                if moved
                    .try_borrow()
                    .map_or(true, |order| matches!(order.gesture, Gesture::Idle))
                {
                    return Ok(());
                }
                let (x, _) = Subscription::position(sender, args)?;
                let step = match moved.try_borrow_mut() {
                    Ok(mut order) => order.drag(x as f64)?,
                    Err(_) => return Ok(()),
                };
                if step == Step::Started {
                    drag_flag.set(true);
                    // Taking the pointer from the button also cancels its click;
                    // the borrow keeps the button's capture loss from ending the drag.
                    let _busy = moved.borrow();
                    Subscription::capture(sender, args)?;
                }
                if step != Step::Ignored {
                    Subscription::handle(args)?;
                }
                Ok(())
            })?,
            Subscription::pointer_handled(strip, 11, move |sender, args| {
                let dropped = match released.try_borrow_mut() {
                    Ok(mut order) => order.release()?,
                    Err(_) => return Ok(()),
                };
                release_flag.set(false);
                if dropped {
                    Subscription::handle(args)?;
                    Subscription::release(sender)?;
                }
                Ok(())
            })?,
            Subscription::pointer_handled(strip, 13, move |sender, _| {
                // The button losing the pointer to the strip starts a drag;
                // only the strip losing it ends one. Releasing the button
                // takes the capture before PointerReleased arrives, so the
                // loss is the drop; only PointerCanceled puts tiles back.
                if Subscription::captured(sender)? {
                    return Ok(());
                }
                if let Ok(mut order) = lost.try_borrow_mut() {
                    lost_flag.set(false);
                    order.release()?;
                }
                Ok(())
            })?,
            Subscription::pointer_handled(strip, 14, move |_, _| {
                cancel_flag.set(false);
                if let Ok(mut order) = canceled.try_borrow_mut() {
                    order.cancel()?;
                }
                Ok(())
            })?,
        ];
        Ok(Self {
            order,
            _events: events,
        })
    }
    /// Drags the first tile past the last one without a pointer, for
    /// `--verify-tiles`; returns the order after the drop.
    pub fn rehearse(&self) -> Result<Vec<String>> {
        let mut order = self.order.borrow_mut();
        order.press(1.0);
        if order.drag(1.0 + THRESHOLD)? != Step::Started || order.drag(1e4)? != Step::Dragged {
            return Err(E_FAIL);
        }
        order.release()?;
        Ok(order.ids.clone())
    }
    /// Only the first `shown` tiles are on view: a drag stays among them, and one
    /// in progress is put back.
    pub fn show(&self, shown: usize) -> Result<()> {
        let mut order = self.order.borrow_mut();
        order.cancel()?;
        order.strip.shown = shown;
        Ok(())
    }
    /// Tile ids in their current order on the taskbar.
    pub fn ids(&self) -> Vec<String> {
        self.order.borrow().ids.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::Strip;
    fn strip() -> Strip {
        // cpu 104, ram 84, net 112 with a 12 px gap: slots at 0, 116, 212.
        Strip {
            widths: vec![104.0, 84.0, 112.0],
            gap: 12.0,
            shown: 3,
        }
    }
    #[test]
    fn hits_tiles_but_not_gaps() {
        let s = strip();
        assert_eq!(s.hit(0.0), Some(0));
        assert_eq!(s.hit(110.0), None);
        assert_eq!(s.hit(120.0), Some(1));
        assert_eq!(s.hit(323.0), Some(2));
        assert_eq!(s.hit(324.0), None);
    }
    #[test]
    fn tiles_left_out_take_no_part_in_a_drag() {
        let s = Strip {
            shown: 2,
            ..strip()
        };
        assert_eq!(s.hit(323.0), None);
        assert_eq!(s.clamp(0, 500.0), 96.0);
        assert_eq!(s.target(0, 96.0), 1);
        assert_eq!(s.moved(0, 1).shown, 2);
    }
    #[test]
    fn keeps_the_dragged_tile_inside_the_strip() {
        let s = strip();
        assert_eq!(s.clamp(0, -30.0), 0.0);
        assert_eq!(s.clamp(0, 500.0), 220.0);
        assert_eq!(s.clamp(2, -500.0), -212.0);
    }
    #[test]
    fn crossing_a_neighbours_centre_takes_its_slot() {
        let s = strip();
        // Centres: cpu 52, ram 158, net 268. cpu's right edge starts at 104.
        assert_eq!(s.target(0, 54.0), 0);
        assert_eq!(s.target(0, 55.0), 1);
        assert_eq!(s.target(0, 165.0), 2);
        assert_eq!(s.target(1, 0.0), 1);
        // net's left edge starts at 212.
        assert_eq!(s.target(2, -54.0), 2);
        assert_eq!(s.target(2, -55.0), 1);
        assert_eq!(s.target(2, -161.0), 0);
    }
    #[test]
    fn every_slot_is_reachable_within_the_strip() {
        let s = strip();
        for from in 0..3 {
            assert_eq!(s.target(from, s.clamp(from, -1e3)), 0);
            assert_eq!(s.target(from, s.clamp(from, 1e3)), 2);
        }
    }
    #[test]
    fn neighbours_step_aside_by_the_dragged_width() {
        let s = strip();
        assert_eq!(
            (0..3).map(|i| s.shift(0, 2, i)).collect::<Vec<_>>(),
            [0.0, -116.0, -116.0]
        );
        assert_eq!(
            (0..3).map(|i| s.shift(2, 0, i)).collect::<Vec<_>>(),
            [124.0, 124.0, 0.0]
        );
        assert_eq!(s.shift(1, 1, 0), 0.0);
    }
    #[test]
    fn landing_matches_the_reordered_layout() {
        let s = strip();
        // cpu after net: ram 0, net 96, cpu 220.
        assert_eq!(s.landing(0, 2), 220.0);
        assert_eq!(s.moved(0, 2).widths, [84.0, 112.0, 104.0]);
        // net first: its slot moves from 212 to 0.
        assert_eq!(s.landing(2, 0), -212.0);
        assert_eq!(s.landing(1, 1), 0.0);
    }
}

//! View structured data (i.e., `json::JsonValue`s) in an `unsegen` widget.
//!
//! # Example:
//! ```no_run
//! extern crate unsegen;
//!
//! use std::io::{stdin, stdout};
//! use unsegen::base::Terminal;
//! use unsegen::input::{Input, Key, ScrollBehavior};
//! use unsegen::widget::{RenderingHints, Widget};
//!
//! use unsegen_jsonviewer::json_ext::{JsonValue, Object};
//! use unsegen_jsonviewer::JsonViewer;
//!
//! fn main() {
//!     let stdout = stdout();
//!     let stdin = stdin();
//!     let stdin = stdin.lock();
//!
//!     let mut json_viewer = JsonViewer::new(&JsonValue::Null);
//!
//!     let mut term = Terminal::new(stdout.lock());
//!
//!     for input in Input::read_all(stdin) {
//!         let input = input.unwrap();
//!         input
//!             .chain(
//!                 ScrollBehavior::new(&mut json_viewer)
//!                     .forwards_on(Key::Down)
//!                     .forwards_on(Key::Char('j'))
//!                     .backwards_on(Key::Up)
//!                     .backwards_on(Key::Char('k'))
//!                     .to_beginning_on(Key::Home)
//!                     .to_end_on(Key::End),
//!             )
//!             .chain((Key::Char('s'), || {
//!                 let mut object = Object::new();
//!                 object.insert("foo", JsonValue::String("String!".to_owned()));
//!                 object.insert("bar", JsonValue::Boolean(true));
//!                 json_viewer.update(&JsonValue::Object(object));
//!             }))
//!             .chain((Key::Char('n'), || {
//!                 let mut object = Object::new();
//!                 object.insert("foo", JsonValue::Number((27 * 37).into()));
//!                 object.insert("bar", JsonValue::Boolean(true));
//!                 // Notice that foo is highlighted when pressing 'n' after 's'!
//!                 json_viewer.update(&JsonValue::Object(object));
//!             }));
//!         // Put more application logic here...
//!
//!         {
//!             let win = term.create_root_window();
//!             json_viewer.draw(win, RenderingHints::default());
//!         }
//!         term.present();
//!     }
//! }
//! ```
#[cfg(test)] //Only tests use macros. Otherwise we get unused_imports warnings.
#[macro_use]
extern crate json;

#[cfg(not(test))]
extern crate json;

extern crate unsegen;

use unsegen::base::basic_types::*;
use unsegen::base::{BoolModifyMode, Color, Cursor, ExtentEstimationWindow, StyleModifier, Window};
use unsegen::widget::{Demand, Demand2D, RenderingHints, Widget};

use unsegen::input::{OperationResult, Scrollable};

/// Convenience reexport of `json` types.
pub mod json_ext {
    pub use json::{number::Number, object::Object, Array, JsonValue};
}

use json::JsonValue;

mod displayvalue;
mod path;

use self::displayvalue::*;
use self::path::*;

/// A widget for viewing `json` data.
///
/// Set an initial value during construction (via `new) and replace it either using `update` or `reset`.
///
/// The widgets provides multiple interaction points, for example to fold or unfold structures, or
/// to grow or shrink the number of elements shown of an array. At any time, one of these knobs is
/// active. Use `select_next` or `select_previous` or the `Scrollable` implementation to change it.
/// Call `toggle_active_element` to interact with the currently active element.
/// Interacting with an element might hide it, but the widget takes care to select another element
/// in that case.
pub struct JsonViewer {
    value: DisplayValue,
    active_element: Path,
    indentation: Width,
    active_focused_style: StyleModifier,
    inactive_focused_style: StyleModifier,
    item_changed_style: StyleModifier,
}

impl JsonViewer {
    /// Create a new `JsonViewer` widget that will display the specified value.
    ///
    /// It follows that it is impossible to not have content. However, it *is* possible to show an
    /// empty String, so there is that.
    pub fn new(value: &JsonValue) -> Self {
        let mut res = JsonViewer {
            value: DisplayValue::from_json(&value),
            active_element: Path::Scalar, //Will be fixed ...
            indentation: Width::new(2).unwrap(),
            active_focused_style: StyleModifier::new()
                .invert(BoolModifyMode::Toggle)
                .bold(true),
            inactive_focused_style: StyleModifier::new().bold(true),
            item_changed_style: StyleModifier::new().bg_color(Color::Red),
        };
        res.fix_active_element_path(); //... here!
        res
    }

    /// Set a new value to display and do not highlight any changes (in contrast to `update`).
    pub fn reset(&mut self, value: &JsonValue) {
        self.value = DisplayValue::from_json(value);
        self.fix_active_element_path();
    }

    /// Set a new value to display and highlight changes from the previous value (which will be
    /// shown until the next `update` or `reset`.
    pub fn update(&mut self, value: &JsonValue) {
        self.value = self.value.update(value);
        self.fix_active_element_path();
    }

    /// Select the next interaction point of the widget (generally "down" from the current one).
    pub fn select_next(&mut self) -> Result<(), ()> {
        if let Some(new_path) = self.active_element.clone().find_next_path(&self.value) {
            self.active_element = new_path;
            Ok(())
        } else {
            Err(())
        }
    }

    /// Select the previous interaction point of the widget (generally "up" from the current one).
    pub fn select_previous(&mut self) -> Result<(), ()> {
        if let Some(new_path) = self.active_element.clone().find_previous_path(&self.value) {
            self.active_element = new_path;
            Ok(())
        } else {
            Err(())
        }
    }

    fn fix_active_element_path(&mut self) {
        let mut tmp = Path::Scalar;
        ::std::mem::swap(&mut self.active_element, &mut tmp);
        self.active_element = tmp.fix_path_for_value(&self.value)
    }

    /// Interact with the currently active interaction point and, for example, fold/unfold
    /// structures.
    pub fn toggle_active_element(&mut self) -> Result<(), ()> {
        let res = self.active_element.find_and_act_on_element(&mut self.value);
        self.fix_active_element_path();
        res
    }
}

impl Widget for JsonViewer {
    fn space_demand(&self) -> Demand2D {
        let mut window = ExtentEstimationWindow::unbounded();
        //TODO: We may want to consider passing hints to space_demand as well for an accurate estimate
        {
            let mut cursor = Cursor::<ExtentEstimationWindow>::new(&mut window);
            let info = RenderingInfo {
                hints: RenderingHints::default(),
                active_focused_style: self.active_focused_style,
                inactive_focused_style: self.inactive_focused_style,
                item_changed_style: self.item_changed_style,
            };
            self.value.draw(
                &mut cursor,
                Some(&self.active_element),
                &info,
                self.indentation,
            );
        }
        Demand2D {
            width: Demand::at_least(window.extent_x()),
            height: Demand::exact(window.extent_y()),
        }
    }
    fn draw(&self, mut window: Window, hints: RenderingHints) {
        let mut cursor = Cursor::new(&mut window);
        let info = RenderingInfo {
            hints,
            active_focused_style: self.active_focused_style,
            inactive_focused_style: self.inactive_focused_style,
            item_changed_style: self.item_changed_style,
        };
        self.value.draw(
            &mut cursor,
            Some(&self.active_element),
            &info,
            self.indentation,
        );
    }
}

impl Scrollable for JsonViewer {
    fn scroll_forwards(&mut self) -> OperationResult {
        self.select_next()
    }
    fn scroll_backwards(&mut self) -> OperationResult {
        self.select_previous()
    }
}

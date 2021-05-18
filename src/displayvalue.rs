use std::collections::BTreeMap;
use unsegen::base::basic_types::*;
use unsegen::base::{Cursor, CursorTarget, StyleModifier};
use unsegen::widget::RenderingHints;

use crate::{Value, ValueVariant};

use std::cmp::min;

use super::path::*;

pub struct RenderingInfo {
    pub hints: RenderingHints,
    pub active_focused_style: StyleModifier,
    pub inactive_focused_style: StyleModifier,
    pub item_changed_style: StyleModifier,
}

impl RenderingInfo {
    fn get_focused_style(&self) -> StyleModifier {
        if self.hints.active {
            self.active_focused_style
        } else {
            self.inactive_focused_style
        }
    }
}

pub struct DisplayObject {
    description: Option<String>,
    pub members: BTreeMap<String, DisplayValue>,
    pub extended: bool,
    description_changed: bool,
}

const OPEN_SYMBOL: &'static str = "[+]";
const CLOSE_SYMBOL: &'static str = "[-]";

impl DisplayObject {
    pub fn toggle_visibility(&mut self) {
        self.extended ^= true;
    }

    fn update<'s, V: Value>(
        &self,
        description: Option<String>,
        obj: Box<dyn Iterator<Item = (String, V)> + 's>,
    ) -> Self {
        let description_changed = self.description != description;
        let mut result = DisplayObject {
            description,
            members: BTreeMap::new(),
            extended: self.extended,
            description_changed,
        };
        for (key, value) in obj.into_iter() {
            let new_value = if let Some(old_val) = self.members.get(&key) {
                old_val.update(value)
            } else {
                DisplayValue::new(value)
            };
            result.members.insert(key.to_string(), new_value);
        }
        result
    }

    fn new<'s, V: Value>(
        description: Option<String>,
        obj: Box<dyn Iterator<Item = (String, V)> + 's>,
    ) -> Self {
        let mut result = DisplayObject {
            description,
            members: BTreeMap::new(),
            extended: true,
            description_changed: false,
        };
        for (key, value) in obj.into_iter() {
            result
                .members
                .insert(key.to_string(), DisplayValue::new(value));
        }
        result
    }

    fn draw<T: CursorTarget>(
        &self,
        cursor: &mut Cursor<T>,
        path: Option<&ObjectPath>,
        info: &RenderingInfo,
        indentation: Width,
    ) {
        use std::fmt::Write;
        {
            let mut cursor = cursor.save().style_modifier();
            if self.description_changed {
                cursor.apply_style_modifier(info.item_changed_style);
            }
            if let Some(description) = &self.description {
                write!(cursor, "{} ", description).unwrap();
            }
        }
        if self.extended {
            {
                write!(cursor, "{{ ").unwrap();
                let mut cursor = cursor.save().style_modifier();
                if let Some(&ObjectPath::Toggle) = path {
                    cursor.apply_style_modifier(info.get_focused_style());
                }
                write!(cursor, "{}", CLOSE_SYMBOL).unwrap();
            }
            {
                let mut cursor = cursor.save().line_start_column();
                cursor.move_line_start_column(indentation.into());
                for (key, value) in self.members.iter() {
                    cursor.wrap_line();
                    write!(cursor, "{}: ", key).unwrap();
                    let subpath = if let Some(&ObjectPath::Item(ref active_key, ref subpath)) = path
                    {
                        if active_key == key {
                            Some(subpath.as_ref())
                        } else {
                            None
                        }
                    } else {
                        None
                    };
                    value.draw(&mut cursor, subpath, info, indentation);
                    write!(cursor, ",").unwrap();
                }
            }
            write!(cursor, "\n}}").unwrap();
        } else {
            write!(cursor, "{{ ").unwrap();
            {
                let mut cursor = cursor.save().style_modifier();
                if let Some(&ObjectPath::Toggle) = path {
                    cursor.apply_style_modifier(info.get_focused_style());
                }
                write!(cursor, "{}", OPEN_SYMBOL).unwrap();
            }
            write!(cursor, " }}").unwrap();
        }
    }
}

pub struct DisplayArray {
    description: Option<String>,
    pub values: Vec<DisplayValue>,
    pub extended: bool,
    pub num_extended: usize,
    pub length_changed: bool,
    description_changed: bool,
}
impl DisplayArray {
    pub fn toggle_visibility(&mut self) {
        self.extended ^= true;
    }
    pub fn grow(&mut self) {
        self.num_extended += 1;
        assert!(self.num_extended <= self.values.len());
    }
    pub fn shrink(&mut self) {
        self.num_extended -= 1;
    }

    pub fn can_grow(&self) -> bool {
        self.num_extended < self.values.len()
    }

    pub fn can_shrink(&self) -> bool {
        self.num_extended > 0
    }

    fn update<'s, V: Value>(
        &self,
        description: Option<String>,
        values: Box<dyn Iterator<Item = V> + 's>,
    ) -> Self {
        let mut old_vals = self.values.iter();
        let values = values
            .into_iter()
            .map(|value| {
                if let Some(old_val) = old_vals.next() {
                    old_val.update(value)
                } else {
                    DisplayValue::new(value)
                }
            })
            .collect::<Vec<_>>();
        let num_extended = min(self.num_extended, values.len());
        let length_changed = self.values.len() != values.len();
        let description_changed = self.description != description;
        DisplayArray {
            description,
            values,
            extended: self.extended,
            num_extended,
            length_changed,
            description_changed,
        }
    }

    fn new<'s, V: Value>(
        description: Option<String>,
        values: Box<dyn Iterator<Item = V> + 's>,
    ) -> Self {
        let values = values
            .into_iter()
            .map(DisplayValue::new)
            .collect::<Vec<_>>();
        let num_extended = min(3, values.len());
        DisplayArray {
            description,
            values,
            extended: true,
            num_extended,
            length_changed: false,
            description_changed: false,
        }
    }

    fn draw<T: CursorTarget>(
        &self,
        cursor: &mut Cursor<T>,
        path: Option<&ArrayPath>,
        info: &RenderingInfo,
        indentation: Width,
    ) {
        use std::fmt::Write;

        {
            let mut cursor = cursor.save().style_modifier();
            if self.description_changed {
                cursor.apply_style_modifier(info.item_changed_style);
            }
            if let Some(description) = &self.description {
                write!(cursor, "{} ", description).unwrap();
            }
        }
        if self.extended {
            write!(cursor, "[ ").unwrap();
            {
                let mut cursor = cursor.save().style_modifier();
                if let Some(&ArrayPath::Toggle) = path {
                    cursor.apply_style_modifier(info.get_focused_style());
                }
                write!(cursor, "{}", CLOSE_SYMBOL).unwrap();
            }
            {
                let mut cursor = cursor.save().line_start_column();
                cursor.move_line_start_column(indentation.into());
                for (i, value) in self.values.iter().enumerate().take(self.num_extended) {
                    cursor.wrap_line();

                    let subpath = if let Some(&ArrayPath::Item(active_i, ref subpath)) = path {
                        if i == active_i {
                            Some(subpath.as_ref())
                        } else {
                            None
                        }
                    } else {
                        None
                    };

                    value.draw(&mut cursor, subpath, info, indentation);
                    write!(cursor, ",",).unwrap();
                }
            }
            write!(cursor, "\n] ").unwrap();
            let mut cursor = cursor.save().style_modifier();
            if self.length_changed {
                cursor.apply_style_modifier(info.item_changed_style);
            }
            write!(cursor, "<").unwrap();
            if self.can_shrink() {
                let mut cursor = cursor.save().style_modifier();
                if let Some(&ArrayPath::Shrink) = path {
                    cursor.apply_style_modifier(info.get_focused_style());
                }
                write!(cursor, "-").unwrap();
            } else {
                write!(cursor, " ").unwrap();
            }
            write!(cursor, "{}/{}", self.num_extended, self.values.len()).unwrap();
            if self.can_grow() {
                let mut cursor = cursor.save().style_modifier();
                if let Some(&ArrayPath::Grow) = path {
                    cursor.apply_style_modifier(info.get_focused_style());
                }
                write!(cursor, "+").unwrap();
            } else {
                write!(cursor, " ").unwrap();
            }
            write!(cursor, ">").unwrap();
        } else {
            write!(cursor, "[ ").unwrap();
            {
                let mut cursor = cursor.save().style_modifier();
                if let Some(&ArrayPath::Toggle) = path {
                    cursor.apply_style_modifier(info.get_focused_style());
                }
                write!(cursor, "{}", OPEN_SYMBOL).unwrap();
            }
            write!(cursor, " ]").unwrap();
        }
    }
}

pub struct DisplayScalar {
    pub value: String,
    pub changed: bool,
}

impl DisplayScalar {
    fn update(&self, new_value: String) -> Self {
        let changed = self.value != new_value;
        DisplayScalar {
            value: new_value,
            changed,
        }
    }

    fn new(value: String) -> Self {
        DisplayScalar {
            value,
            changed: false,
        }
    }

    fn draw<T: CursorTarget>(&self, cursor: &mut Cursor<T>, active: bool, info: &RenderingInfo) {
        let mut cursor = cursor.save().style_modifier();
        if active {
            cursor.apply_style_modifier(info.get_focused_style());
        }
        if self.changed {
            cursor.apply_style_modifier(info.item_changed_style);
        }
        cursor.write(&self.value);
    }
}

pub enum DisplayValue {
    Scalar(DisplayScalar),
    Object(DisplayObject),
    Array(DisplayArray),
}

impl DisplayValue {
    pub fn update(&self, value: impl Value) -> Self {
        match (self, value.clone().visit()) {
            (DisplayValue::Scalar(old), ValueVariant::Scalar(s)) => {
                DisplayValue::Scalar(old.update(s))
            }
            (DisplayValue::Object(old), ValueVariant::Map(d, s)) => {
                DisplayValue::Object(old.update(d, s))
            }
            (DisplayValue::Array(old), ValueVariant::Array(d, s)) => {
                DisplayValue::Array(old.update(d, s))
            }
            _ => {
                // The type of the value has changed
                let mut val = Self::new(value);
                match &mut val {
                    DisplayValue::Scalar(v) => {
                        v.changed = true;
                    }
                    DisplayValue::Object(_) | DisplayValue::Array(_) => { /*TODO: Propagate changed state further*/
                    }
                }
                val
            }
        }
    }

    pub fn new(value: impl Value) -> Self {
        match value.visit() {
            ValueVariant::Scalar(s) => DisplayValue::Scalar(DisplayScalar::new(s.to_owned())),
            ValueVariant::Map(d, s) => DisplayValue::Object(DisplayObject::new(d, s)),
            ValueVariant::Array(d, s) => DisplayValue::Array(DisplayArray::new(d, s)),
        }
    }
    pub fn draw<T: CursorTarget>(
        &self,
        cursor: &mut Cursor<T>,
        path: Option<&Path>,
        info: &RenderingInfo,
        indentation: Width,
    ) {
        match (self, path) {
            (&DisplayValue::Scalar(ref scalar), Some(&Path::Scalar)) => {
                scalar.draw(cursor, true, info)
            }
            (&DisplayValue::Scalar(ref scalar), None) => scalar.draw(cursor, false, info),
            (&DisplayValue::Object(ref obj), Some(&Path::Object(ref op))) => {
                obj.draw(cursor, Some(op), info, indentation)
            }
            (&DisplayValue::Object(ref obj), None) => obj.draw(cursor, None, info, indentation),
            (&DisplayValue::Array(ref array), Some(&Path::Array(ref ap))) => {
                array.draw(cursor, Some(ap), info, indentation)
            }
            (&DisplayValue::Array(ref array), None) => array.draw(cursor, None, info, indentation),
            _ => panic!("Mismatched DisplayValue and path type!"),
        }
    }
}

#[cfg(test)]
impl DisplayValue {
    pub fn unwrap_scalar_ref(&self) -> &DisplayScalar {
        if let &DisplayValue::Scalar(ref val) = self {
            val
        } else {
            panic!("Tried to unwrap non-scalar DisplayValue");
        }
    }

    pub fn unwrap_object_ref(&self) -> &DisplayObject {
        if let &DisplayValue::Object(ref val) = self {
            val
        } else {
            panic!("Tried to unwrap non-object DisplayValue");
        }
    }
    pub fn unwrap_object_ref_mut(&mut self) -> &mut DisplayObject {
        if let &mut DisplayValue::Object(ref mut val) = self {
            val
        } else {
            panic!("Tried to unwrap non-object DisplayValue");
        }
    }

    pub fn unwrap_array_ref(&self) -> &DisplayArray {
        if let &DisplayValue::Array(ref val) = self {
            val
        } else {
            panic!("Tried to unwrap non-array DisplayValue");
        }
    }
    pub fn unwrap_array_ref_mut(&mut self) -> &mut DisplayArray {
        if let &mut DisplayValue::Array(ref mut val) = self {
            val
        } else {
            panic!("Tried to unwrap non-array DisplayValue");
        }
    }
}

use std::fmt::Display;

// Represents a single input item in a report
#[derive(Debug)]
pub struct Input {
    pub usage: (u16, u16),
    pub value: InputValue,
}

#[derive(Debug)]
pub enum InputValue {
    Bool(bool),
    UInt(u32),
    Int(i32),
    None, // "Null state"
}

impl Display for Input {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.value {
            InputValue::Bool(b) => write!(f, "({:02x} {:02x}): {}", self.usage.0, self.usage.1, b),
            InputValue::UInt(u) => write!(f, "({:02x} {:02x}): {}", self.usage.0, self.usage.1, u),
            InputValue::Int(i) => write!(f, "({:02x} {:02x}): {}", self.usage.0, self.usage.1, i),
            InputValue::None => write!(f, "None"),
        }
    }
}

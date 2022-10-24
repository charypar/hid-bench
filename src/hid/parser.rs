use std::{collections::VecDeque, num};

use super::basic::{self, BasicItem, BasicItems};

#[derive(Debug)]
pub struct ReportParser {
    collection: Collection<Report>,
}

impl ReportParser {
    pub fn new<'a>(basic_items: BasicItems<'a>) -> Self {
        ReportParser {
            collection: Self::read_items(basic_items),
        }
    }

    pub fn parse_input(&self, input: &[u8]) -> Collection<Vec<Input>> {
        self.collection.map(|report| report.parse(input))
    }

    // FIXME error handling
    fn read_items(basic_items: BasicItems) -> Collection<Report> {
        let global = GlobalItems::new();
        let local = LocalItems::new();
        let mut state_table = StateTable { global, local };

        let mut collection_stack: VecDeque<Collection<Report>> = VecDeque::new(); // current collection
        let mut bit_offset = 0u32;

        for item in basic_items {
            match item {
                BasicItem::Global(item) => {
                    Self::read_global_item(&mut state_table, item);
                }
                BasicItem::Local(item) => Self::read_local_item(&mut state_table, item),
                BasicItem::Main(item) => match item {
                    basic::MainItem::Input(input) => Self::create_input_item(
                        &mut state_table,
                        &mut collection_stack,
                        &mut bit_offset,
                        input,
                    ),
                    // Output and feature items not yet implemented
                    basic::MainItem::Output(_) => continue,
                    basic::MainItem::Feature(_) => continue,
                    basic::MainItem::Collection(c) => {
                        if state_table.local.usages.len() != 1 {
                            panic!("Too many usages for a collection");
                        }
                        let local_usage = state_table.local.usages[0];

                        // Start a new collection
                        let collection_type = c;
                        let usage =
                            Self::qualify_usage(&state_table.global.usage_page, &local_usage)
                                .expect("Bad usage item");

                        let collection = Collection {
                            collection_type,
                            usage,
                            designator_index: None,
                            string_index: None,
                            items: vec![],
                        };

                        // Make the collection the active one, main items will be pushed into it
                        collection_stack.push_back(collection);

                        // Clear the local state table
                        state_table.local = LocalItems::new();
                    }
                    basic::MainItem::EndCollection => {
                        // close the collection and add it to its parrent collection items
                        if collection_stack.len() == 1 {
                            continue; // nothing to be done about the top level collection
                        }

                        let top = collection_stack.len() - 2;
                        let collection = collection_stack
                            .pop_back()
                            .expect("can't pop collection of a stack with items");

                        collection_stack[top]
                            .items
                            .push(CollectionItem::Collection(collection));
                    }
                    basic::MainItem::Reserved => continue,
                },
                BasicItem::Reserved => continue,
            }
        }

        collection_stack.pop_front().expect("No collection found!")
    }

    // FIXME error handling
    fn read_global_item(state_table: &mut StateTable, item: basic::GlobalItem) {
        match item {
            basic::GlobalItem::UsagePage(up) => state_table.global.usage_page = Some(up),
            basic::GlobalItem::LogicalMinimum(lm) => state_table.global.logical_minimum = Some(lm),
            basic::GlobalItem::LogicalMaximum(lm) => state_table.global.logical_maximum = Some(lm),
            basic::GlobalItem::PhysicalMinimum(pm) => {
                state_table.global.physical_minimum = Some(pm)
            }
            basic::GlobalItem::PhysicalMaximum(pm) => {
                state_table.global.physical_maximum = Some(pm)
            }
            basic::GlobalItem::UnitExponent(ue) => state_table.global.unit_exponent = Some(ue),
            basic::GlobalItem::Unit(u) => state_table.global.unit = Some(u),
            basic::GlobalItem::ReportSize(rs) => state_table.global.report_size = Some(rs),
            basic::GlobalItem::ReportID(rid) => state_table.global.report_id = Some(rid),
            basic::GlobalItem::ReportCount(rc) => state_table.global.report_count = Some(rc),
            basic::GlobalItem::Push => {
                todo!("Item state table stack is not yet implemented")
            }
            basic::GlobalItem::Pop => {
                todo!("Item state table stack is not yet implemented")
            }
            basic::GlobalItem::Reserved => return,
        }
    }

    // FIXME error handling
    fn read_local_item(state_table: &mut StateTable, item: basic::LocalItem) {
        match item {
            basic::LocalItem::Usage(usage) => state_table.local.usages.push((None, Some(usage))),
            basic::LocalItem::UsageMinimum(um) => {
                state_table.local.usage_minimum = (None, Some(um))
            }
            basic::LocalItem::UsageMaximum(um) => {
                state_table.local.usage_maximum = (None, Some(um))
            }
            basic::LocalItem::ExtendedUsage(up, usage) => {
                state_table.local.usages.push((Some(up), Some(usage)))
            }
            basic::LocalItem::ExtendedUsageMinimum(up, um) => {
                state_table.local.usage_minimum = (Some(up), Some(um))
            }
            basic::LocalItem::ExtendedUsageMaximum(up, um) => {
                state_table.local.usage_maximum = (Some(up), Some(um))
            }
            basic::LocalItem::Delimiter(_) => todo!("Delimiters are not yet implemented"),
            // Strings and designators not yet implemented
            basic::LocalItem::DesignatorIndex(_) => return,
            basic::LocalItem::DesignatorMinimum(_) => return,
            basic::LocalItem::DesignatorMaximum(_) => return,
            basic::LocalItem::StringIndex(_) => return,
            basic::LocalItem::StringMinimum(_) => return,
            basic::LocalItem::StringMaximum(_) => return,
            basic::LocalItem::Reserved => return,
        }
    }

    // FIXME error handling!
    fn create_input_item(
        state_table: &mut StateTable,
        collection_stack: &mut VecDeque<Collection<Report>>,
        bit_offset: &mut u32,
        input: basic::InputItemData,
    ) {
        let report_type = ReportType::Input(input.data);
        let usage_page = state_table.global.usage_page;

        let usages = state_table
            .local
            .usages
            .iter()
            .map(|usage| {
                Self::qualify_usage(&usage_page, usage).expect("Missing usage page for input item")
            })
            .collect();
        let usage_maximum = Self::qualify_usage(&usage_page, &state_table.local.usage_maximum);
        let usage_minimum = Self::qualify_usage(&usage_page, &state_table.local.usage_minimum);

        let report_size = state_table
            .global
            .report_size
            .expect("Missing report size for input item");
        let report_count = state_table
            .global
            .report_count
            .expect("Missing report size for input item");

        let logical_minimum = state_table
            .global
            .logical_minimum
            .expect("Missing logical minimum for input item");
        let logical_maximum = state_table
            .global
            .logical_maximum
            .expect("Missing logical minimum for input item");

        let physical_minimum = state_table
            .global
            .physical_minimum
            .unwrap_or(logical_minimum);
        let physical_maximum = state_table
            .global
            .physical_minimum
            .unwrap_or(logical_maximum);

        let report = Report {
            report_type,
            usages,
            usage_minimum,
            usage_maximum,
            bit_offset: *bit_offset as usize, // TODO!
            report_id: state_table.global.report_id,
            report_size,
            report_count,
            logical_minimum,
            logical_maximum,
            physical_minimum,
            physical_maximum,
            unit: state_table.global.unit,
            unit_exponent: state_table.global.unit_exponent,
        };

        let top = collection_stack.len() - 1;
        collection_stack[top]
            .items
            .push(CollectionItem::Item(report));

        *bit_offset += report_count * report_size;
        state_table.local = LocalItems::new();
    }

    // FIXME error handling
    fn qualify_usage(
        usage_page: &Option<u16>,
        usage: &(Option<u16>, Option<u16>),
    ) -> Option<(u16, u16)> {
        match (usage_page, usage) {
            (_, (None, None)) => None,
            (_, (Some(up), Some(us))) => Some((*up, *us)),
            (Some(up), (None, Some(us))) => Some((*up, *us)),
            (None, (None, Some(_))) => panic!("Missing usage page"),
            _ => panic!("Missing usage"),
        }
    }
}

// Collection type, reused for reports
#[derive(Debug)]
pub struct Collection<T> {
    collection_type: basic::Collection,
    usage: (u16, u16),
    // "String and Physical indices, as well as delimiters may be associated with collections."
    // TODO delimiter support (when needed)
    designator_index: Option<u32>,
    string_index: Option<u32>,
    items: Vec<CollectionItem<T>>,
}

impl<T> Collection<T> {
    pub fn map<O, F: Fn(&T) -> O>(&self, f: F) -> Collection<O>
    where
        F: Copy,
        O: IntoIterator,
    {
        Collection {
            collection_type: self.collection_type,
            usage: self.usage,
            designator_index: self.designator_index,
            string_index: self.string_index,
            items: self
                .items
                .iter()
                .map(|item| match item {
                    CollectionItem::Collection(c) => {
                        let col = c.map(f);

                        CollectionItem::Collection(col)
                    }
                    CollectionItem::Item(item) => CollectionItem::Item(f(item)),
                })
                .collect(),
        }
    }
}

#[derive(Debug)]
enum CollectionItem<T> {
    Collection(Collection<T>),
    Item(T),
}

struct StateTable {
    global: GlobalItems,
    local: LocalItems,
}

struct GlobalItems {
    usage_page: Option<u16>,
    logical_minimum: Option<i32>,
    logical_maximum: Option<i32>,
    physical_minimum: Option<i32>,
    physical_maximum: Option<i32>,
    unit_exponent: Option<u32>,
    unit: Option<u32>,
    report_size: Option<u32>,
    report_id: Option<u8>,
    report_count: Option<u32>,
}

impl GlobalItems {
    fn new() -> Self {
        Self {
            usage_page: None,
            logical_minimum: None,
            logical_maximum: None,
            physical_minimum: None,
            physical_maximum: None,
            unit_exponent: None,
            unit: None,
            report_size: None,
            report_id: None,
            report_count: None,
        }
    }
}

struct LocalItems {
    usages: Vec<(Option<u16>, Option<u16>)>,   // page, usage
    usage_minimum: (Option<u16>, Option<u16>), // page, usage
    usage_maximum: (Option<u16>, Option<u16>), // page, usage
    designator_index: Option<u32>,
    designator_minimum: Option<u32>,
    designator_maximum: Option<u32>,
    string_index: Option<u32>,
    string_minimum: Option<u32>,
    string_maximum: Option<u32>,
    // TODO support delimiter
}

impl LocalItems {
    fn new() -> Self {
        Self {
            usages: vec![],
            usage_minimum: (None, None),
            usage_maximum: (None, None),
            designator_index: None,
            designator_minimum: None,
            designator_maximum: None,
            string_index: None,
            string_minimum: None,
            string_maximum: None,
        }
    }
}

// A single report, may read multiple inputs of the same configuration
#[derive(Debug)]
struct Report {
    report_type: ReportType,
    usages: Vec<(u16, u16)>,
    usage_minimum: Option<(u16, u16)>,
    usage_maximum: Option<(u16, u16)>,
    logical_minimum: i32,
    logical_maximum: i32,
    physical_minimum: i32,
    physical_maximum: i32,
    unit: Option<u32>,
    unit_exponent: Option<u32>,
    bit_offset: usize,     // start of the report in the overall report data
    report_id: Option<u8>, // if given, add 8 bits to the offset, check the ID matches
    report_size: u32,
    report_count: u32,
}

impl Report {
    fn parse(&self, report: &[u8]) -> Vec<Input> {
        if let ReportType::Input(flags) = self.report_type {
            if (flags & 1) == 1 {
                return vec![];
            }
        }

        let spec_usages = self.usages.len();

        (0..(self.report_count as usize))
            .map(|i| {
                let usage = if i < spec_usages {
                    self.usages[i]
                } else {
                    // Usage Minimum specifies the usage to be associated with the first unassociated control
                    // in the array or bitmap. Usage Maximum specifies the end of the range of usage values
                    // to be associated with item elements.
                    if let Some((up, u)) = self.usage_minimum {
                        (up, u + spec_usages as u16 + i as u16)
                    } else {
                        // HID 1.11, section 6.2.2.8 Local Items
                        //
                        // While Local items do not carry over to the next Main item,
                        // they may apply to more than one control within a single item.
                        // For example, if an Input item defining five controls is
                        // preceded by three Usage tags, the three usages would be
                        // assigned sequentially to the first three controls, and the
                        // third usage would also be assigned to the fourth and fifth controls.
                        self.usages[self.usages.len() - 1]
                    }
                };

                let offset = self.bit_offset + (self.report_size as usize * i);
                let base_value = Self::extract_value(report, offset, self.report_size);

                let value = match (self.logical_minimum, self.logical_maximum) {
                    (0, 1) => InputValue::Bool(base_value != 0),
                    (a, b) if a >= 0 && b >= 0 => InputValue::UInt(base_value),
                    _ => InputValue::Int(Self::signed(base_value, self.report_size)),
                };

                Input { usage, value }
            })
            .collect()
    }

    fn signed(value: u32, length: u32) -> i32 {
        let sign_mask = 1 << (length - 1);
        let number_mask = !sign_mask; // is also the highest length-bit number

        if value & sign_mask != 0 {
            // TODO make sure this is right
            ((value & number_mask) + number_mask + 1) as i32
        } else {
            (value & number_mask) as i32
        }
    }

    fn extract_value(report: &[u8], bit_offset: usize, bit_length: u32) -> u32 {
        let first_byte = bit_offset / 8; // first byte in which the value is
        let last_byte = (bit_offset + bit_length as usize - 1) / 8;
        let bit_shift = bit_offset % 8;

        // TODO check bounds!
        let bytes = &report[first_byte..=last_byte];

        let mut value = 0u32;
        for byte in 0..bytes.len() {
            // numbers are little-endian!
            value = value | ((bytes[byte as usize] as u32) << (8 * byte));
        }

        value = value >> bit_shift;
        value = value & !(0xFFFFFFFFu32 << bit_length + 1);

        value
    }
}

#[derive(Debug)]
enum ReportType {
    Input(u32),
    Output(u32),
    Feature(u32),
}

impl ReportType {
    // TODO decoding of bit flags
}

// Represents a single input item in a report
#[derive(Debug)]
pub struct Input {
    usage: (u16, u16),
    value: InputValue,
}

#[derive(Debug)]
pub enum InputValue {
    Bool(bool),
    UInt(u32),
    Int(i32),
}

// Not yet supported
pub struct OutputItem {}

// Not yet supported
pub struct FeatureItem {}

#[cfg(test)]
mod test {
    use super::{Report, ReportParser};
    use crate::hid::basic::BasicItems;

    const JOYSTICK: [u8; 101] = [
        0x5, 0x1, 0x9, 0x4, 0xa1, 0x1, 0x9, 0x1, 0xa1, 0x0, 0x5, 0x1, 0x9, 0x30, 0x9, 0x31, 0x15,
        0x0, 0x26, 0xff, 0x3, 0x75, 0xa, 0x95, 0x2, 0x81, 0x2, 0x9, 0x35, 0x15, 0x0, 0x26, 0xff,
        0x0, 0x75, 0x8, 0x95, 0x1, 0x81, 0x2, 0x9, 0x32, 0x9, 0x36, 0x15, 0x0, 0x26, 0xff, 0x0,
        0x75, 0x8, 0x95, 0x2, 0x81, 0x2, 0x5, 0x9, 0x19, 0x1, 0x29, 0xe, 0x15, 0x0, 0x25, 0x1,
        0x75, 0x1, 0x95, 0xe, 0x81, 0x2, 0x5, 0x1, 0x9, 0x39, 0x15, 0x1, 0x25, 0x8, 0x35, 0x0,
        0x46, 0x3b, 0x1, 0x66, 0x14, 0x0, 0x75, 0x4, 0x95, 0x1, 0x81, 0x42, 0x75, 0x2, 0x95, 0x1,
        0x81, 0x1, 0xc0, 0xc0,
    ];

    #[test]
    fn parses_basic_report_descriptor_items() {
        let basic_items = BasicItems::new(&JOYSTICK);
        println!("{:x?}", basic_items);
        println!("{:?}", basic_items.collect::<Vec<_>>());
    }

    #[test]
    fn parses_a_report_descriptor() {
        let basic_items = BasicItems::new(&JOYSTICK);
        let parser = ReportParser::new(basic_items);

        println!("{:#?}", parser);
    }

    #[test]
    fn parses_an_input_report() {
        let basic_items = BasicItems::new(&JOYSTICK);
        let parser = ReportParser::new(basic_items);

        let input_report = [0u8; 64];
        let input = parser.parse_input(&input_report);

        println!("{:#?}", input);
    }

    #[test]
    fn extracts_single_bit_value() {
        let report: [u8; 1] = [0b1];
        let expected = 1;
        let actual = Report::extract_value(&report, 0, 1);

        assert_eq!(actual, expected);
        let report: [u8; 1] = [0b10];
        let expected = 1;
        let actual = Report::extract_value(&report, 1, 1);

        assert_eq!(actual, expected);

        assert_eq!(actual, expected);
        let report: [u8; 3] = [0b0, 0b0, 0b100];
        let expected = 1;
        let actual = Report::extract_value(&report, 18, 1);

        assert_eq!(actual, expected);
    }

    #[test]
    fn extracts_multi_bit_value() {
        let report: [u8; 1] = [0b101];
        let expected = 5;
        let actual = Report::extract_value(&report, 0, 3);

        assert_eq!(actual, expected);

        let report: [u8; 3] = [0b0, 0b0, 0b1010];
        let expected = 5;
        let actual = Report::extract_value(&report, 17, 3);

        assert_eq!(actual, expected);

        let report: [u8; 3] = [0b10000000, 0b10, 0b0];
        let expected = 5;
        let actual = Report::extract_value(&report, 7, 3);

        assert_eq!(actual, expected);

        let report: [u8; 3] = [0b10000000, 0b10, 0b00011];
        let expected = 0b11000000101;
        let actual = Report::extract_value(&report, 7, 11);

        assert_eq!(actual, expected);

        let report: [u8; 2] = [0b10, 0b1000_0000];
        let expected = 0b10000000_0000001;
        let actual = Report::extract_value(&report, 1, 15);

        assert_eq!(actual, expected);
    }
}

use std::{collections::VecDeque, os::macos::raw::stat};

use super::basic::{self, BasicItem, BasicItems};

#[derive(Debug)]
pub struct ReportParser {
    collection: Collection<Report>,
}

impl ReportParser {
    pub fn new<'a>(basic_items: BasicItems<'a>) -> Self {
        let global = GlobalItems::new();
        let local = LocalItems::new();

        let mut state_table = StateTable { global, local };
        let mut collection_stack: VecDeque<Collection<Report>> = VecDeque::new(); // current collection

        for item in basic_items {
            match item {
                BasicItem::Global(item) => match item {
                    basic::GlobalItem::UsagePage(up) => state_table.global.usage_page = Some(up),
                    basic::GlobalItem::LogicalMinimum(lm) => {
                        state_table.global.logical_minimum = Some(lm)
                    }
                    basic::GlobalItem::LogicalMaximum(lm) => {
                        state_table.global.logical_maximum = Some(lm)
                    }
                    basic::GlobalItem::PhysicalMinimum(pm) => {
                        state_table.global.physical_minimum = Some(pm)
                    }
                    basic::GlobalItem::PhysicalMaximum(pm) => {
                        state_table.global.physical_maximum = Some(pm)
                    }
                    basic::GlobalItem::UnitExponent(ue) => {
                        state_table.global.unit_exponent = Some(ue)
                    }
                    basic::GlobalItem::Unit(u) => state_table.global.unit = Some(u),
                    basic::GlobalItem::ReportSize(rs) => state_table.global.report_size = Some(rs),
                    basic::GlobalItem::ReportID(rid) => state_table.global.report_id = Some(rid),
                    basic::GlobalItem::ReportCount(rc) => {
                        state_table.global.report_count = Some(rc)
                    }
                    basic::GlobalItem::Push => {
                        todo!("Item state table stack is not yet implemented")
                    }
                    basic::GlobalItem::Pop => {
                        todo!("Item state table stack is not yet implemented")
                    }
                    basic::GlobalItem::Reserved => continue,
                },
                BasicItem::Local(item) => match item {
                    basic::LocalItem::Usage(usage) => {
                        state_table.local.usages.push((None, Some(usage)))
                    }
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
                    // Strings and designators not yet implemented
                    basic::LocalItem::DesignatorIndex(_) => continue,
                    basic::LocalItem::DesignatorMinimum(_) => continue,
                    basic::LocalItem::DesignatorMaximum(_) => continue,
                    basic::LocalItem::StringIndex(_) => continue,
                    basic::LocalItem::StringMinimum(_) => continue,
                    basic::LocalItem::StringMaximum(_) => continue,
                    basic::LocalItem::Delimiter(_) => todo!("Delimiters are not yet implemented"),
                    basic::LocalItem::Reserved => continue,
                },
                BasicItem::Main(item) => match item {
                    basic::MainItem::Input(input) => {
                        let report_type = ReportType::Input(input.data);
                        let usage_page = state_table.global.usage_page;

                        let usages = state_table
                            .local
                            .usages
                            .iter()
                            .map(|usage| match (usage_page, usage) {
                                (_, (Some(up), Some(us))) => (*up, *us),
                                (Some(up), (None, Some(us))) => (up, *us),
                                (None, (None, Some(us))) => panic!("Missing usage page"),
                                _ => panic!("Missing usage for an input item"),
                            })
                            .collect();
                        let usage_maximum = match (usage_page, state_table.local.usage_maximum) {
                            (_, (None, None)) => None,
                            (_, (Some(up), Some(us))) => Some((up, us)),
                            (Some(up), (None, Some(us))) => Some((up, us)),
                            (None, (None, Some(us))) => panic!("Missing usage page"),
                            _ => panic!("Missing usage minimum for an input item"),
                        };
                        let usage_minimum = match (usage_page, state_table.local.usage_minimum) {
                            (_, (None, None)) => None,
                            (_, (Some(up), Some(us))) => Some((up, us)),
                            (Some(up), (None, Some(us))) => Some((up, us)),
                            (None, (None, Some(us))) => panic!("Missing usage page"),
                            _ => panic!("Missing usage maximum for an input item"),
                        };
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
                            bit_offset: 0, // TODO!
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

                        state_table.local = LocalItems::new();
                    }
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
                        let usage = match (state_table.global.usage_page, local_usage) {
                            (_, (Some(up), Some(usage))) => (up, usage),
                            (Some(up), (None, Some(usage))) => (up, usage),
                            _ => panic!("Bad usage item"),
                        };

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

        ReportParser {
            collection: collection_stack.pop_front().expect("No collection found!"),
        }
    }

    fn parse_input(report: &[u8]) -> Collection<Input> {
        Collection {
            collection_type: basic::Collection::Application,
            usage: (0, 0),
            designator_index: None,
            string_index: None,
            items: vec![],
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
        vec![]
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
pub struct Input {
    usage: (u16, u16),
    value: InputValue,
}

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
    use super::ReportParser;
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
    fn parses_basic_report_items() {
        let basic_items = BasicItems::new(&JOYSTICK);
        println!("{:x?}", basic_items);
        println!("{:?}", basic_items.collect::<Vec<_>>());
    }

    #[test]
    fn parses_a_report() {
        let basic_items = BasicItems::new(&JOYSTICK);
        let parser = ReportParser::new(basic_items);

        println!("{:#?}", parser);
    }
}

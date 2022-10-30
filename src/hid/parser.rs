use std::collections::VecDeque;

use super::basic::{BasicItem, BasicItems, GlobalItem, InputItemData, LocalItem, MainItem};
use super::collection::{Collection, CollectionItem};
use super::input::Input;
use super::report::{Report, ReportType};

#[derive(Debug)]
pub struct Parser {
    collection: Collection<Report>,
}

impl Parser {
    pub fn new(basic_items: BasicItems<'_>) -> Self {
        Parser {
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
                    MainItem::Input(input) => Self::create_input_item(
                        &mut state_table,
                        &mut collection_stack,
                        &mut bit_offset,
                        input,
                    ),
                    // Output and feature items not yet implemented
                    MainItem::Output(_) => continue,
                    MainItem::Feature(_) => continue,
                    MainItem::Collection(c) => {
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
                    MainItem::EndCollection => {
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
                    MainItem::Reserved => continue,
                },
                BasicItem::Reserved => continue,
            }
        }

        collection_stack.pop_front().expect("No collection found!")
    }

    // FIXME error handling
    fn read_global_item(state_table: &mut StateTable, item: GlobalItem) {
        match item {
            GlobalItem::UsagePage(up) => state_table.global.usage_page = Some(up),
            GlobalItem::LogicalMinimum(lm) => state_table.global.logical_minimum = Some(lm),
            GlobalItem::LogicalMaximum(lm) => state_table.global.logical_maximum = Some(lm),
            GlobalItem::PhysicalMinimum(pm) => state_table.global.physical_minimum = Some(pm),
            GlobalItem::PhysicalMaximum(pm) => state_table.global.physical_maximum = Some(pm),
            GlobalItem::UnitExponent(ue) => state_table.global.unit_exponent = Some(ue),
            GlobalItem::Unit(u) => state_table.global.unit = Some(u),
            GlobalItem::ReportSize(rs) => state_table.global.report_size = Some(rs),
            GlobalItem::ReportID(rid) => state_table.global.report_id = Some(rid),
            GlobalItem::ReportCount(rc) => state_table.global.report_count = Some(rc),
            GlobalItem::Push => {
                todo!("Item state table stack is not yet implemented")
            }
            GlobalItem::Pop => {
                todo!("Item state table stack is not yet implemented")
            }
            GlobalItem::Reserved => (),
        }
    }

    // FIXME error handling
    fn read_local_item(state_table: &mut StateTable, item: LocalItem) {
        match item {
            LocalItem::Usage(usage) => state_table.local.usages.push((None, Some(usage))),
            LocalItem::UsageMinimum(um) => state_table.local.usage_minimum = (None, Some(um)),
            LocalItem::UsageMaximum(um) => state_table.local.usage_maximum = (None, Some(um)),
            LocalItem::ExtendedUsage(up, usage) => {
                state_table.local.usages.push((Some(up), Some(usage)))
            }
            LocalItem::ExtendedUsageMinimum(up, um) => {
                state_table.local.usage_minimum = (Some(up), Some(um))
            }
            LocalItem::ExtendedUsageMaximum(up, um) => {
                state_table.local.usage_maximum = (Some(up), Some(um))
            }
            LocalItem::Delimiter(_) => todo!("Delimiters are not yet implemented"),
            // Strings and designators not yet implemented
            LocalItem::DesignatorIndex(di) => state_table.local.designator_index = Some(di),
            LocalItem::DesignatorMinimum(dm) => state_table.local.designator_minimum = Some(dm),
            LocalItem::DesignatorMaximum(dm) => state_table.local.designator_maximum = Some(dm),
            LocalItem::StringIndex(si) => state_table.local.string_index = Some(si),
            LocalItem::StringMinimum(sm) => state_table.local.string_minimum = Some(sm),
            LocalItem::StringMaximum(sm) => state_table.local.string_maximum = Some(sm),
            LocalItem::Reserved => (),
        }
    }

    // FIXME error handling!
    fn create_input_item(
        state_table: &mut StateTable,
        collection_stack: &mut VecDeque<Collection<Report>>,
        bit_offset: &mut u32,
        input: InputItemData,
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
            bit_offset: *bit_offset as usize,
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

#[cfg(test)]
mod test {
    use super::super::BasicItems;
    use super::Parser;

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
        let parser = Parser::new(basic_items);

        println!("{:#?}", parser);
    }

    #[test]
    fn parses_an_input_report() {
        let basic_items = BasicItems::new(&JOYSTICK);
        let parser = Parser::new(basic_items);

        let input_report = [0u8; 64];
        let input = parser.parse_input(&input_report);

        println!("{:#?}", input);
    }
}

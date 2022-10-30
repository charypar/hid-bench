use super::input::{Input, InputValue};

// A single report, may read multiple inputs of the same configuration
#[derive(Debug)]
pub struct Report {
    pub report_type: ReportType,
    pub usages: Vec<(u16, u16)>,
    pub usage_minimum: Option<(u16, u16)>,
    pub usage_maximum: Option<(u16, u16)>,
    pub logical_minimum: i32,
    pub logical_maximum: i32,
    pub physical_minimum: i32,
    pub physical_maximum: i32,
    pub unit: Option<u32>,
    pub unit_exponent: Option<u32>,
    pub bit_offset: usize,     // start of the report in the overall report data
    pub report_id: Option<u8>, // if given, add 8 bits to the offset, check the ID matches
    pub report_size: u32,
    pub report_count: u32,
}

impl Report {
    pub fn parse(&self, report: &[u8]) -> Vec<Input> {
        let ReportType::Input(flags) = self.report_type;
        if (flags & 1) == 1 {
            return vec![];
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
        let number_mask = !(0xFFFF_FFFF << (length - 1));

        let sign = value & sign_mask;
        let unsinged_number = value & number_mask;

        if sign != 0 {
            (unsinged_number | !number_mask) as i32
        } else {
            unsinged_number as i32
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
            value |= (bytes[byte as usize] as u32) << (8 * byte);
        }

        value >>= bit_shift;
        value &= !(0xFFFFFFFFu32 << bit_length);

        value
    }
}

#[derive(Debug)]
pub enum ReportType {
    Input(u32),
    // TODO ready for other types of report
    //
    // Output(u32),
    // Feature(u32),
}

impl ReportType {
    // TODO decoding of bit flags
}

#[cfg(test)]
mod test {
    use super::Report;

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

        let expected = 0;
        let actual = Report::extract_value(&report, 17, 1);

        assert_eq!(actual, expected);

        let expected = 0;
        let actual = Report::extract_value(&report, 19, 1);

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
        let expected = 0b100_0000_0000_0001;
        let actual = Report::extract_value(&report, 1, 15);

        assert_eq!(actual, expected);
    }

    #[test]
    fn convert_any_bit_length_to_i32() {
        let actual = Report::signed((!27u8 + 1) as u32, 8);
        let expected = -27;

        println!("-27i32: {:032b}", -27i32);
        assert_eq!(actual, expected);

        let actual = Report::signed((!1u8 + 1) as u32, 8);
        let expected = -1;

        assert_eq!(actual, expected);

        let actual = Report::signed(1u8 as u32, 8);
        let expected = 1;

        assert_eq!(actual, expected);

        let actual = Report::signed(127u8 as u32, 8);
        let expected = 127;

        assert_eq!(actual, expected);

        let actual = Report::signed((!127u8 + 1) as u32, 8);
        let expected = -127;

        assert_eq!(actual, expected);
    }
}

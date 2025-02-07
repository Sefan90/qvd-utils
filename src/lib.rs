use pyo3::wrap_pyfunction;
use pyo3::{prelude::*, types::PyDict, types::PyList};
use quick_xml::de::from_str;
use qvd_structure::{QvdFieldHeader, QvdTableHeader};
use std::io::SeekFrom;
use std::io::{self, Read};
use std::path::Path;
use std::str;
use std::{fs::File};
use std::{convert::TryInto, io::prelude::*};
pub mod qvd_structure;
use regex::Regex;

#[pymodule]
fn qvd(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(read_qvd, m)?)?;

    Ok(())
}

#[pyfunction]
fn read_qvd(py: Python, files: &PyList, find_string: String, wildcard: bool) -> PyResult<Py<PyDict>> {
    let dict = PyDict::new(py);
    for file_name in files {
        let xml: String = get_xml_data(&file_name.to_string()).expect("Error reading file");
        let binary_section_offset = xml.as_bytes().len();
        let qvd_structure: QvdTableHeader = from_str(&xml).unwrap();

        if let Ok(f) = File::open(&file_name.to_string()) {
            // Seek to the end of the XML section
            let buf = read_qvd_to_buf(f, binary_section_offset);
            let mut strings: Vec<Option<String>> = Vec::new();

            if wildcard == true {
                for field in qvd_structure.fields.headers {
                    let records = get_symbols_as_strings(&buf, &field);
                    let re = Regex::new(&(".*".to_owned()+&find_string.clone()+&".*".to_owned())).unwrap();
                    for rec in records {
                        if re.is_match(&rec.unwrap()){
                            strings.push(Some(field.field_name.clone()));
                            break;
                        }
                    }
                }
            }
            else {
                for field in qvd_structure.fields.headers {
                    let records = get_symbols_as_strings(&buf, &field);
                    if records.into_iter().any(|x| x == Some(find_string.clone())) {
                        strings.push(Some(field.field_name.clone()));
                    }
                }
            }

            if strings.len() > 0 {
                dict.set_item(file_name.clone(), strings).unwrap();
            }
        }
    }
    Ok(dict.into())
}

fn read_qvd_to_buf(mut f: File, binary_section_offset: usize) -> Vec<u8> {
    f.seek(SeekFrom::Start(binary_section_offset as u64))
        .unwrap();
    let mut buf: Vec<u8> = Vec::new();
    f.read_to_end(&mut buf).unwrap();
    buf
}

fn get_symbols_as_strings(buf: &[u8], field: &QvdFieldHeader) -> Vec<Option<String>> {
    let start = field.offset;
    let end = start + field.length;
    let mut string_start: usize = 0;
    let mut strings: Vec<Option<String>> = Vec::new();

    let mut i = start;
    while i < end {
        let byte = &buf[i];
        // Check first byte of symbol. This is not part of the symbol but tells us what type of data to read.
        match byte {
            0 => {
                // Strings are null terminated
                // Read bytes from start fo string (string_start) up to current byte.
                let utf8_bytes = buf[string_start..i].to_vec().to_owned();
                let value = String::from_utf8(utf8_bytes).unwrap_or_else(|_| {
                    panic!(
                    "Error parsing string value in field: {}, field offset: {}, byte offset: {}",
                    field.field_name, start, i
                )
                });
                strings.push(Some(value));
                i += 1;
            }
            1 => {
                // 4 byte integer
                let target_bytes = buf[i + 1..i + 5].to_vec();
                let byte_array: [u8; 4] = target_bytes.try_into().unwrap();
                let numeric_value = i32::from_le_bytes(byte_array);
                strings.push(Some(numeric_value.to_string()));
                i += 5;
            }
            2 => {
                // 4 byte double
                let target_bytes = buf[i + 1..i + 9].to_vec();
                let byte_array: [u8; 8] = target_bytes.try_into().unwrap();
                let numeric_value = f64::from_le_bytes(byte_array);
                strings.push(Some(numeric_value.to_string()));
                i += 9;
            }
            4 => {
                // Beginning of a null terminated string type
                // Mark where string value starts, excluding preceding byte 0x04
                i += 1;
                string_start = i;
            }
            5 => {
                // 4 bytes of unknown followed by null terminated string
                // Skip the 4 bytes before string
                i += 5;
                string_start = i;
            }
            6 => {
                // 8 bytes of unknown followed by null terminated string
                // Skip the 8 bytes before string
                i += 9;
                string_start = i;
            }
            _ => {
                // Part of a string, do nothing until null terminator
                i += 1;
            }
        }
    }
    strings
}

fn get_xml_data(file_name: &str) -> Result<String, io::Error> {
    match read_file(file_name) {
        Ok(mut reader) => {
            let mut buffer = Vec::new();
            // There is a line break, carriage return and a null terminator between the XMl and data
            // Find the null terminator
            reader
                .read_until(0, &mut buffer)
                .expect("Failed to read file");
            let xml_string =
                str::from_utf8(&buffer[..]).expect("xml section contains invalid UTF-8 chars");
            Ok(xml_string.to_owned())
        }
        Err(e) => Err(e),
    }
}

fn read_file<P>(filename: P) -> io::Result<io::BufReader<File>>
where
    P: AsRef<Path>,
{
    let file = File::open(filename)?;
    Ok(io::BufReader::new(file))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_double() {
        let buf: Vec<u8> = vec![
            0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x40, 0x7a, 0x40, 0x02, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x50, 0x7a, 0x40,
        ];
        let field = QvdFieldHeader {
            length: buf.len(),
            offset: 0,
            field_name: String::new(),
            bias: 0,
            bit_offset: 0,
            bit_width: 0,
        };
        let res = get_symbols_as_strings(&buf, &field);
        let expected: Vec<Option<String>> = vec![Some(420.0.to_string()), Some(421.0.to_string())];
        assert_eq!(expected, res);
    }

    #[test]
    fn test_int() {
        let buf: Vec<u8> = vec![0x01, 0x0A, 0x00, 0x00, 0x00, 0x01, 0x14, 0x00, 0x00, 0x00];
        let field = QvdFieldHeader {
            length: buf.len(),
            offset: 0,
            field_name: String::new(),
            bias: 0,
            bit_offset: 0,
            bit_width: 0,
        };
        let res = get_symbols_as_strings(&buf, &field);
        let expected = vec![Some(10.0.to_string()), Some(20.0.to_string())];
        assert_eq!(expected, res);
    }

    #[test]
    #[rustfmt::skip]
    fn test_mixed_numbers() {
        let buf: Vec<u8> = vec![
            0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x40, 0x7a, 0x40, 
            0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x50, 0x7a, 0x40,
            0x01, 0x01, 0x00, 0x00, 0x00, 
            0x01, 0x02, 0x00, 0x00, 0x00,
            0x05, 0x00, 0x00, 0x00, 0x00, 0x37, 0x30, 0x30, 0x30, 0x00,
            0x06, 0x00,0x00,0x00, 0x00,0x00,0x00,0x00,0x00, 0x38, 0x36, 0x35, 0x2e, 0x32, 0x00
        ];
        let field = QvdFieldHeader {
            length: buf.len(),
            offset: 0,
            field_name: String::new(),
            bias: 0,
            bit_offset: 0,
            bit_width: 0,
        };
        let res = get_symbols_as_strings(&buf, &field);
        let expected: Vec<Option<String>> = vec![
            Some(420.to_string()),
            Some(421.to_string()),
            Some(1.to_string()),
            Some(2.to_string()),
            Some(7000.to_string()),
            Some(865.2.to_string())
        ];
        assert_eq!(expected, res);
    }

    #[test]
    fn test_string() {
        let buf: Vec<u8> = vec![
            4, 101, 120, 97, 109, 112, 108, 101, 32, 116, 101, 120, 116, 0, 4, 114, 117, 115, 116,
            0,
        ];
        let field = QvdFieldHeader {
            length: buf.len(),
            offset: 0,
            field_name: String::new(),
            bias: 0,
            bit_offset: 0,
            bit_width: 0,
        };
        let res = get_symbols_as_strings(&buf, &field);
        let expected = vec![Some("example text".into()), Some("rust".into())];
        assert_eq!(expected, res);
    }

    #[test]
    #[rustfmt::skip]
    fn test_utf8_string() {
        let buf: Vec<u8> = vec![
            0x04, 0xE4, 0xB9, 0x9F, 0xE6, 0x9C, 0x89, 0xE4, 0xB8, 0xAD, 0xE6, 0x96, 0x87, 0xE7,
            0xAE, 0x80, 0xE4, 0xBD, 0x93, 0xE5, 0xAD, 0x97, 0x00,
            0x04, 0xF0, 0x9F, 0x90, 0x8D, 0xF0, 0x9F, 0xA6, 0x80, 0x00,
        ];

        let field = QvdFieldHeader {
            length: buf.len(),
            offset: 0,
            field_name: String::new(),
            bias: 0,
            bit_offset: 0,
            bit_width: 0,
        };
        let res = get_symbols_as_strings(&buf, &field);
        let expected = vec![Some("也有中文简体字".into()), Some("🐍🦀".into())];
        assert_eq!(expected, res);
    }

    #[test]
    fn test_mixed_string() {
        let buf: Vec<u8> = vec![
            4, 101, 120, 97, 109, 112, 108, 101, 32, 116, 101, 120, 116, 0, 4, 114, 117, 115, 116,
            0, 5, 42, 65, 80, 1, 49, 50, 51, 52, 0, 6, 1, 1, 1, 1, 1, 1, 1, 1, 100, 111, 117, 98,
            108, 101, 0,
        ];
        let field = QvdFieldHeader {
            length: buf.len(),
            offset: 0,
            field_name: String::new(),
            bias: 0,
            bit_offset: 0,
            bit_width: 0,
        };
        let res = get_symbols_as_strings(&buf, &field);
        let expected = vec![
            Some("example text".into()),
            Some("rust".into()),
            Some("1234".into()),
            Some("double".into()),
        ];
        assert_eq!(expected, res);
    }

    #[test]
    fn test_bitslice_to_vec() {
        let mut x: Vec<u8> = vec![
            0x00, 0x00, 0x00, 0x11, 0x01, 0x22, 0x02, 0x33, 0x13, 0x34, 0x14, 0x35,
        ];
        let bits = BitSlice::<Msb0, _>::from_slice(&mut x[..]).unwrap();
        let target = &bits[27..32];
        let binary_vec = bitslice_to_vec(&target);

        let mut sum: u32 = 0;
        for bit in binary_vec {
            sum <<= 1;
            sum += bit as u32;
        }
        assert_eq!(17, sum);
    }

    #[test]
    fn test_get_row_indexes() {
        let buf: Vec<u8> = vec![
            0x00, 0x14, 0x00, 0x11, 0x01, 0x22, 0x02, 0x33, 0x13, 0x34, 0x24, 0x35,
        ];
        let field = QvdFieldHeader {
            field_name: String::from("name"),
            offset: 0,
            length: 0,
            bit_offset: 10,
            bit_width: 3,
            bias: 0,
        };
        let record_byte_size = buf.len();
        let res = get_row_indexes(&buf, &field, record_byte_size);
        let expected: Vec<i64> = vec![5];
        assert_eq!(expected, res);
    }
}

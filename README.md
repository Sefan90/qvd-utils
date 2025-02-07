# Find string in Qlik Sense .qvd files 🛠

A python library for finding a specific string in a Qlik Sense .qvd file, written in Rust. 
Based on the library https://github.com/SBentley/qvd-utils 

<!--
## Install

Install from PyPi https://pypi.org/project/qvd/

```sh
pip install qvd
```
-->

## Usage

```python
from qvd import qvd_reader

# qvd_reader.read([File list],'find this string',contains string)
df = qvd_reader.read(['test1.qvd','test2.qvd'],'find_this_string',True)

# qvd_reader.read_to_dict([File list],'find this string',contains string)
dict = qvd_reader.read_to_dict(['test1.qvd','test2.qvd'],'find_this_string',True)
print(dict)

# Output:
# {'test1.qvd':['Header1','Header2'],'test2.qvd':['Header3','Header4']}
```

### Developing

Create a virtual env https://docs.python-guide.org/dev/virtualenvs/ and activate it.

```sh
python3 -m venv venv
```

Then install dev dependencies:

```sh
pip install pandas maturin
```

Afterwards, run 

```sh
maturin develop --release
```

to install the generated python lib to the virtual env.

## Test

To run the tests, you can use these commands:

```sh
cargo test  # runs all Rust unit tests
pytest test_qvd_reader.py  # runs all Python tests
```

## QVD File Structure

A QVD file is split into 3 parts; XML Metdata, Symbols table and the bit
stuffed binary indexes.

### XML Metadata

This section is at the top of the file and is in human readable XML. This
section contains metadata about the file in gneneral such as table name, number
of records, size of records as well as data about individual fields including
field name, length offset in symbol table.

### Symbol table

Directly after the xml section is the symbol table. This is a table of every
unique value contained within each column. The columns are in the order
described in the metadata fields section. In the metadata we can find the byte
offset from the start of the symbols section for each column. Symbol types
cannot be determined from the metadata and are instead determined by a flag
byte preceding each symbol. These types are:

* 1 - 4 byte signed int (u32) - little endiand
* 2 - 8 byte signed float (f64) - little endian
* 4 - null terminated string
* 5 - 4 bytes of junk follwed by a null terminated string representing an integer
* 6 - 8 bytes of junk followed by a null terminated string representing a float

### Binary Indexes

After the symbol table are the binary indexes that map to the symbols for each
row. They are bit stuffed and reversed binary numbers that point to the index
of the symbol in the symbols table for each field.

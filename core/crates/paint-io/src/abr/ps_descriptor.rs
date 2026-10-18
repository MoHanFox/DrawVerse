//! Typed, bounded Photoshop descriptor data. No property interpretation or file access.
use crate::{check, Error, Result};
use paint_task::CancellationToken;
use std::collections::BTreeSet;
pub const MAX_DESCRIPTOR_BYTES: usize = 16 * 1024 * 1024;
#[derive(Debug, PartialEq)]
pub struct Class {
    pub name: String,
    pub id: Vec<u8>,
}
#[derive(Debug, PartialEq)]
pub struct Descriptor {
    pub class: Class,
    pub entries: Vec<(Vec<u8>, Value)>,
}
impl Descriptor {
    pub fn get(&self, key: &[u8]) -> Option<&Value> {
        self.entries.iter().find(|(k, _)| k == key).map(|(_, v)| v)
    }
}
#[derive(Debug, PartialEq)]
pub enum Reference {
    Property {
        class: Class,
        key: Vec<u8>,
    },
    Class(Class),
    Enumerated {
        class: Class,
        type_id: Vec<u8>,
        value: Vec<u8>,
    },
    Offset {
        class: Class,
        value: i32,
    },
    Identifier(i32),
    Index(i32),
    Name {
        class: Class,
        value: String,
    },
}
#[derive(Debug, PartialEq)]
pub enum Value {
    Object {
        global: bool,
        descriptor: Box<Descriptor>,
    },
    List(Vec<Value>),
    Double(f64),
    UnitDouble {
        unit: [u8; 4],
        value: f64,
    },
    UnitFloat {
        unit: [u8; 4],
        value: f32,
    },
    Text(String),
    Enumerated {
        type_id: Vec<u8>,
        value: Vec<u8>,
    },
    Integer(i32),
    LargeInteger(i64),
    Boolean(bool),
    Class {
        global: bool,
        class: Class,
    },
    Alias(Box<[u8]>),
    Raw(Box<[u8]>),
    Path(Box<[u8]>),
    Reference(Vec<Reference>),
}
struct Reader<'a> {
    bytes: &'a [u8],
    offset: usize,
    nodes: usize,
    token: &'a CancellationToken,
}
impl Reader<'_> {
    fn take(&mut self, count: usize) -> Result<&[u8]> {
        let end = self
            .offset
            .checked_add(count)
            .ok_or(Error::Limit("descriptor offset"))?;
        let bytes = self
            .bytes
            .get(self.offset..end)
            .ok_or(Error::Invalid("truncated descriptor"))?;
        self.offset = end;
        Ok(bytes)
    }
    fn u32(&mut self) -> Result<u32> {
        Ok(u32::from_be_bytes(
            self.take(4)?.try_into().expect("four bytes"),
        ))
    }
    fn tag(&mut self) -> Result<[u8; 4]> {
        Ok(self.take(4)?.try_into().expect("four bytes"))
    }
    fn text(&mut self) -> Result<String> {
        let count = self.u32()? as usize;
        if count > 8192 {
            return Err(Error::Limit("descriptor Unicode length"));
        }
        let mut units = Vec::with_capacity(count);
        for pair in self.take(count * 2)?.chunks_exact(2) {
            units.push(u16::from_be_bytes([pair[0], pair[1]]));
        }
        String::from_utf16(&units).map_err(|_| Error::Invalid("descriptor Unicode"))
    }
    fn id(&mut self) -> Result<Vec<u8>> {
        let count = self.u32()? as usize;
        let count = if count == 0 { 4 } else { count };
        if count > 1024 {
            return Err(Error::Limit("descriptor ID bytes"));
        }
        Ok(self.take(count)?.to_vec())
    }
    fn class(&mut self) -> Result<Class> {
        Ok(Class {
            name: self.text()?,
            id: self.id()?,
        })
    }
    fn count(&mut self) -> Result<usize> {
        let count = self.u32()? as usize;
        if count > 4096 {
            return Err(Error::Limit("descriptor container count"));
        }
        Ok(count)
    }
    fn node(&mut self, depth: usize) -> Result<()> {
        check(self.token)?;
        if depth > 32 || self.nodes == 65536 {
            return Err(Error::Limit("descriptor depth/node budget"));
        }
        self.nodes += 1;
        Ok(())
    }
    fn double(&mut self) -> Result<f64> {
        let value = f64::from_be_bytes(self.take(8)?.try_into().expect("eight bytes"));
        if !value.is_finite() {
            return Err(Error::Invalid("descriptor non-finite double"));
        }
        Ok(value)
    }
    fn blob(&mut self) -> Result<Box<[u8]>> {
        let count = self.u32()? as usize;
        if count > MAX_DESCRIPTOR_BYTES {
            return Err(Error::Limit("descriptor blob bytes"));
        }
        Ok(self.take(count)?.into())
    }
    fn descriptor(&mut self, depth: usize) -> Result<Descriptor> {
        self.node(depth)?;
        let class = self.class()?;
        let count = self.count()?;
        let mut keys = BTreeSet::new();
        let mut entries = Vec::with_capacity(count);
        for _ in 0..count {
            let key = self.id()?;
            if !keys.insert(key.clone()) {
                return Err(Error::Invalid("duplicate descriptor key"));
            }
            entries.push((key, self.value(depth + 1)?));
        }
        Ok(Descriptor { class, entries })
    }
    fn references(&mut self, depth: usize) -> Result<Vec<Reference>> {
        let count = self.count()?;
        let mut values = Vec::with_capacity(count);
        for _ in 0..count {
            self.node(depth)?;
            let tag = self.tag()?;
            values.push(match &tag {
                b"prop" => Reference::Property {
                    class: self.class()?,
                    key: self.id()?,
                },
                b"Clss" => Reference::Class(self.class()?),
                b"Enmr" => Reference::Enumerated {
                    class: self.class()?,
                    type_id: self.id()?,
                    value: self.id()?,
                },
                b"rele" => Reference::Offset {
                    class: self.class()?,
                    value: self.u32()? as i32,
                },
                b"Idnt" => Reference::Identifier(self.u32()? as i32),
                b"indx" => Reference::Index(self.u32()? as i32),
                b"name" => Reference::Name {
                    class: self.class()?,
                    value: self.text()?,
                },
                _ => {
                    return Err(Error::Unsupported(format!(
                        "descriptor reference tag {tag:?}"
                    )))
                }
            });
        }
        Ok(values)
    }
    fn value(&mut self, depth: usize) -> Result<Value> {
        self.node(depth)?;
        let tag = self.tag()?;
        Ok(match &tag {
            b"Objc" | b"GlbO" => Value::Object {
                global: &tag == b"GlbO",
                descriptor: Box::new(self.descriptor(depth)?),
            },
            b"VlLs" => {
                let count = self.count()?;
                let mut values = Vec::with_capacity(count);
                for _ in 0..count {
                    values.push(self.value(depth + 1)?);
                }
                Value::List(values)
            }
            b"doub" => Value::Double(self.double()?),
            b"UntF" => Value::UnitDouble {
                unit: self.tag()?,
                value: self.double()?,
            },
            b"UnFl" => {
                let unit = self.tag()?;
                let value = f32::from_be_bytes(self.take(4)?.try_into().expect("four bytes"));
                if !value.is_finite() {
                    return Err(Error::Invalid("descriptor non-finite float"));
                }
                Value::UnitFloat { unit, value }
            }
            b"TEXT" => Value::Text(self.text()?),
            b"enum" => Value::Enumerated {
                type_id: self.id()?,
                value: self.id()?,
            },
            b"long" => Value::Integer(self.u32()? as i32),
            b"comp" => Value::LargeInteger(i64::from_be_bytes(
                self.take(8)?.try_into().expect("eight bytes"),
            )),
            b"bool" => {
                let byte = self.take(1)?[0];
                if byte > 1 {
                    return Err(Error::Invalid("descriptor boolean"));
                }
                Value::Boolean(byte == 1)
            }
            b"type" | b"GlbC" => Value::Class {
                global: &tag == b"GlbC",
                class: self.class()?,
            },
            b"alis" => Value::Alias(self.blob()?),
            b"tdta" => Value::Raw(self.blob()?),
            b"Pth " => Value::Path(self.blob()?),
            b"obj " => Value::Reference(self.references(depth + 1)?),
            _ => return Err(Error::Unsupported(format!("descriptor value tag {tag:?}"))),
        })
    }
}
pub fn decode(bytes: &[u8], token: &CancellationToken) -> Result<Descriptor> {
    check(token)?;
    if bytes.len() > MAX_DESCRIPTOR_BYTES {
        return Err(Error::Limit("descriptor input bytes"));
    }
    let mut reader = Reader {
        bytes,
        offset: 0,
        nodes: 0,
        token,
    };
    if reader.u32()? != 16 {
        return Err(Error::Unsupported("descriptor version".into()));
    }
    let descriptor = reader.descriptor(0)?;
    if reader.offset != bytes.len() {
        return Err(Error::Invalid("descriptor trailing bytes"));
    }
    check(token)?;
    Ok(descriptor)
}

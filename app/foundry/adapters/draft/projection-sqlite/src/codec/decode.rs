use std::collections::BTreeMap;

use data_boundary_kernel::Classified;
use data_ontology_kernel::{
    CalendarDate, FiniteDouble, ObjectEntity, ObjectProperty, PropertyValue,
};
use foundry_projection_draft::ProjectedObject;

use super::{CODEC_VERSION, CodecError, class_from_tag, refuse, tier_from_tag};

pub(crate) fn decode_object(bytes: &[u8]) -> Result<ProjectedObject, CodecError> {
    let mut cursor = Cursor { bytes, at: 0 };
    if cursor.u8()? != CODEC_VERSION {
        return refuse("unknown codec version");
    }
    let tenant_id = cursor.string()?;
    let id = cursor.string()?;
    let entity_type_value = cursor.string()?;
    let entity_type_class = class_from_tag(cursor.u8()?)?;
    let count = cursor.u32()?;
    let mut properties = BTreeMap::new();
    for _ in 0..count {
        let name = cursor.string()?;
        let tier = tier_from_tag(cursor.u8()?)?;
        let data_class = class_from_tag(cursor.u8()?)?;
        let decoded = cursor.value()?;
        properties.insert(
            name.clone(),
            ObjectProperty {
                name,
                value: Classified::new(decoded, data_class),
                tier,
            },
        );
    }
    let schema_revision = cursor.u32()?;
    let last_ordinal = cursor.u64()?;
    let last_actor = cursor.string()?;
    if cursor.at != bytes.len() {
        return refuse("trailing bytes");
    }
    Ok(ProjectedObject {
        entity: ObjectEntity {
            tenant_id,
            id,
            entity_type: Classified::new(entity_type_value, entity_type_class),
            properties,
        },
        schema_revision,
        last_ordinal,
        last_actor,
    })
}

struct Cursor<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl Cursor<'_> {
    fn take(&mut self, n: usize) -> Result<&[u8], CodecError> {
        let end = self.at.checked_add(n).ok_or(CodecError {
            detail: "length overflow",
        })?;
        if end > self.bytes.len() {
            return refuse("truncated buffer");
        }
        let slice = &self.bytes[self.at..end];
        self.at = end;
        Ok(slice)
    }

    fn u8(&mut self) -> Result<u8, CodecError> {
        Ok(self.take(1)?[0])
    }

    fn u32(&mut self) -> Result<u32, CodecError> {
        Ok(u32::from_le_bytes(self.take(4)?.try_into().map_err(
            |_| CodecError {
                detail: "u32 shape",
            },
        )?))
    }

    fn u64(&mut self) -> Result<u64, CodecError> {
        Ok(u64::from_le_bytes(self.take(8)?.try_into().map_err(
            |_| CodecError {
                detail: "u64 shape",
            },
        )?))
    }

    fn i64(&mut self) -> Result<i64, CodecError> {
        Ok(i64::from_le_bytes(self.take(8)?.try_into().map_err(
            |_| CodecError {
                detail: "i64 shape",
            },
        )?))
    }

    fn i32(&mut self) -> Result<i32, CodecError> {
        Ok(i32::from_le_bytes(self.take(4)?.try_into().map_err(
            |_| CodecError {
                detail: "i32 shape",
            },
        )?))
    }

    fn string(&mut self) -> Result<String, CodecError> {
        let len = self.u32()? as usize;
        let bytes = self.take(len)?;
        String::from_utf8(bytes.to_vec()).map_err(|_| CodecError {
            detail: "non-utf8 string",
        })
    }

    fn value(&mut self) -> Result<PropertyValue, CodecError> {
        Ok(match self.u8()? {
            1 => PropertyValue::String(self.string()?),
            2 => PropertyValue::Integer(self.i64()?),
            3 => {
                let bits = self.u64()?;
                PropertyValue::Double(FiniteDouble::new(f64::from_bits(bits)).map_err(|_| {
                    CodecError {
                        detail: "non-finite double",
                    }
                })?)
            }
            4 => PropertyValue::Boolean(match self.u8()? {
                0 => false,
                1 => true,
                _ => return refuse("boolean shape"),
            }),
            5 => {
                let year = self.i32()?;
                let month = self.u8()?;
                let day = self.u8()?;
                PropertyValue::Date(CalendarDate::new(year, month, day).map_err(|_| {
                    CodecError {
                        detail: "invalid date",
                    }
                })?)
            }
            6 => PropertyValue::Timestamp {
                epoch_millis: self.i64()?,
            },
            7 => {
                let count = self.u32()?;
                let mut items = Vec::new();
                for _ in 0..count {
                    items.push(self.value()?);
                }
                PropertyValue::Array(items)
            }
            8 => {
                let count = self.u32()?;
                let mut entries = BTreeMap::new();
                for _ in 0..count {
                    let key = self.string()?;
                    entries.insert(key, self.value()?);
                }
                PropertyValue::Struct(entries)
            }
            _ => return refuse("unknown value tag"),
        })
    }
}

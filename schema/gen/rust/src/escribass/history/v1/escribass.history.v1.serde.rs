// @generated
impl serde::Serialize for Op {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if true {
            len += 1;
        }
        if true {
            len += 1;
        }
        if true {
            len += 1;
        }
        if true {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("escribass.history.v1.Op", len)?;
        if true {
            struct_ser.serialize_field("op", &self.op)?;
        }
        if true {
            struct_ser.serialize_field("path", &self.path)?;
        }
        if let Some(v) = self.from.as_ref() {
            struct_ser.serialize_field("from", v)?;
        }
        if let Some(v) = self.value.as_ref() {
            struct_ser.serialize_field("value", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for Op {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "op",
            "path",
            "from",
            "value",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Op,
            Path,
            From,
            Value,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "op" => Ok(GeneratedField::Op),
                            "path" => Ok(GeneratedField::Path),
                            "from" => Ok(GeneratedField::From),
                            "value" => Ok(GeneratedField::Value),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = Op;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct escribass.history.v1.Op")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<Op, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut op__ = None;
                let mut path__ = None;
                let mut from__ = None;
                let mut value__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Op => {
                            if op__.is_some() {
                                return Err(serde::de::Error::duplicate_field("op"));
                            }
                            op__ = Some(map_.next_value()?);
                        }
                        GeneratedField::Path => {
                            if path__.is_some() {
                                return Err(serde::de::Error::duplicate_field("path"));
                            }
                            path__ = Some(map_.next_value()?);
                        }
                        GeneratedField::From => {
                            if from__.is_some() {
                                return Err(serde::de::Error::duplicate_field("from"));
                            }
                            from__ = map_.next_value()?;
                        }
                        GeneratedField::Value => {
                            if value__.is_some() {
                                return Err(serde::de::Error::duplicate_field("value"));
                            }
                            value__ = map_.next_value()?;
                        }
                    }
                }
                Ok(Op {
                    op: op__.unwrap_or_default(),
                    path: path__.unwrap_or_default(),
                    from: from__,
                    value: value__,
                })
            }
        }
        deserializer.deserialize_struct("escribass.history.v1.Op", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for PatchEntry {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if true {
            len += 1;
        }
        if true {
            len += 1;
        }
        if true {
            len += 1;
        }
        if true {
            len += 1;
        }
        if true {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("escribass.history.v1.PatchEntry", len)?;
        if true {
            struct_ser.serialize_field("id", &self.id)?;
        }
        if true {
            struct_ser.serialize_field("parents", &self.parents)?;
        }
        if true {
            struct_ser.serialize_field("tool", &self.tool)?;
        }
        if true {
            struct_ser.serialize_field("ops", &self.ops)?;
        }
        if let Some(v) = self.provenance.as_ref() {
            struct_ser.serialize_field("provenance", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for PatchEntry {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "id",
            "parents",
            "tool",
            "ops",
            "provenance",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Id,
            Parents,
            Tool,
            Ops,
            Provenance,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "id" => Ok(GeneratedField::Id),
                            "parents" => Ok(GeneratedField::Parents),
                            "tool" => Ok(GeneratedField::Tool),
                            "ops" => Ok(GeneratedField::Ops),
                            "provenance" => Ok(GeneratedField::Provenance),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = PatchEntry;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct escribass.history.v1.PatchEntry")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<PatchEntry, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut id__ = None;
                let mut parents__ = None;
                let mut tool__ = None;
                let mut ops__ = None;
                let mut provenance__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Id => {
                            if id__.is_some() {
                                return Err(serde::de::Error::duplicate_field("id"));
                            }
                            id__ = Some(map_.next_value()?);
                        }
                        GeneratedField::Parents => {
                            if parents__.is_some() {
                                return Err(serde::de::Error::duplicate_field("parents"));
                            }
                            parents__ = Some(map_.next_value()?);
                        }
                        GeneratedField::Tool => {
                            if tool__.is_some() {
                                return Err(serde::de::Error::duplicate_field("tool"));
                            }
                            tool__ = Some(map_.next_value()?);
                        }
                        GeneratedField::Ops => {
                            if ops__.is_some() {
                                return Err(serde::de::Error::duplicate_field("ops"));
                            }
                            ops__ = Some(map_.next_value()?);
                        }
                        GeneratedField::Provenance => {
                            if provenance__.is_some() {
                                return Err(serde::de::Error::duplicate_field("provenance"));
                            }
                            provenance__ = map_.next_value()?;
                        }
                    }
                }
                Ok(PatchEntry {
                    id: id__.unwrap_or_default(),
                    parents: parents__.unwrap_or_default(),
                    tool: tool__.unwrap_or_default(),
                    ops: ops__.unwrap_or_default(),
                    provenance: provenance__,
                })
            }
        }
        deserializer.deserialize_struct("escribass.history.v1.PatchEntry", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for Refs {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if true {
            len += 1;
        }
        if true {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("escribass.history.v1.Refs", len)?;
        if true {
            struct_ser.serialize_field("head", &self.head)?;
        }
        if true {
            struct_ser.serialize_field("refs", &self.refs)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for Refs {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "head",
            "refs",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Head,
            Refs,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "head" => Ok(GeneratedField::Head),
                            "refs" => Ok(GeneratedField::Refs),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = Refs;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct escribass.history.v1.Refs")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<Refs, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut head__ = None;
                let mut refs__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Head => {
                            if head__.is_some() {
                                return Err(serde::de::Error::duplicate_field("head"));
                            }
                            head__ = Some(map_.next_value()?);
                        }
                        GeneratedField::Refs => {
                            if refs__.is_some() {
                                return Err(serde::de::Error::duplicate_field("refs"));
                            }
                            refs__ = Some(
                                map_.next_value::<std::collections::BTreeMap<_, _>>()?
                            );
                        }
                    }
                }
                Ok(Refs {
                    head: head__.unwrap_or_default(),
                    refs: refs__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("escribass.history.v1.Refs", FIELDS, GeneratedVisitor)
    }
}

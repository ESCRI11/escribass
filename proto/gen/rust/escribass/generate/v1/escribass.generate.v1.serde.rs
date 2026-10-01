// @generated
impl serde::Serialize for CompileRequest {
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
        let mut struct_ser = serializer.serialize_struct("escribass.generate.v1.CompileRequest", len)?;
        if true {
            let v = ::escribass_schema::song::GeneratorKind::try_from(self.kind)
                .map_err(|_| serde::ser::Error::custom(format!("Invalid variant {}", self.kind)))?;
            struct_ser.serialize_field("kind", &v)?;
        }
        if true {
            struct_ser.serialize_field("source", &self.source)?;
        }
        if true {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("seed", ToString::to_string(&self.seed).as_str())?;
        }
        if true {
            struct_ser.serialize_field("params", &self.params)?;
        }
        if true {
            struct_ser.serialize_field("tempo", &self.tempo)?;
        }
        if true {
            struct_ser.serialize_field("signature", &self.signature)?;
        }
        if true {
            struct_ser.serialize_field("sections", &self.sections)?;
        }
        if true {
            struct_ser.serialize_field("clip_start_tick", &self.clip_start_tick)?;
        }
        if true {
            struct_ser.serialize_field("clip_length_ticks", &self.clip_length_ticks)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for CompileRequest {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "kind",
            "source",
            "seed",
            "params",
            "tempo",
            "signature",
            "sections",
            "clip_start_tick",
            "clipStartTick",
            "clip_length_ticks",
            "clipLengthTicks",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Kind,
            Source,
            Seed,
            Params,
            Tempo,
            Signature,
            Sections,
            ClipStartTick,
            ClipLengthTicks,
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
                            "kind" => Ok(GeneratedField::Kind),
                            "source" => Ok(GeneratedField::Source),
                            "seed" => Ok(GeneratedField::Seed),
                            "params" => Ok(GeneratedField::Params),
                            "tempo" => Ok(GeneratedField::Tempo),
                            "signature" => Ok(GeneratedField::Signature),
                            "sections" => Ok(GeneratedField::Sections),
                            "clipStartTick" | "clip_start_tick" => Ok(GeneratedField::ClipStartTick),
                            "clipLengthTicks" | "clip_length_ticks" => Ok(GeneratedField::ClipLengthTicks),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = CompileRequest;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct escribass.generate.v1.CompileRequest")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<CompileRequest, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut kind__ = None;
                let mut source__ = None;
                let mut seed__ = None;
                let mut params__ = None;
                let mut tempo__ = None;
                let mut signature__ = None;
                let mut sections__ = None;
                let mut clip_start_tick__ = None;
                let mut clip_length_ticks__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Kind => {
                            if kind__.is_some() {
                                return Err(serde::de::Error::duplicate_field("kind"));
                            }
                            kind__ = Some(map_.next_value::<::escribass_schema::song::GeneratorKind>()? as i32);
                        }
                        GeneratedField::Source => {
                            if source__.is_some() {
                                return Err(serde::de::Error::duplicate_field("source"));
                            }
                            source__ = Some(map_.next_value()?);
                        }
                        GeneratedField::Seed => {
                            if seed__.is_some() {
                                return Err(serde::de::Error::duplicate_field("seed"));
                            }
                            seed__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::Params => {
                            if params__.is_some() {
                                return Err(serde::de::Error::duplicate_field("params"));
                            }
                            params__ = Some(
                                map_.next_value::<std::collections::BTreeMap<_, _>>()?
                            );
                        }
                        GeneratedField::Tempo => {
                            if tempo__.is_some() {
                                return Err(serde::de::Error::duplicate_field("tempo"));
                            }
                            tempo__ = Some(map_.next_value()?);
                        }
                        GeneratedField::Signature => {
                            if signature__.is_some() {
                                return Err(serde::de::Error::duplicate_field("signature"));
                            }
                            signature__ = Some(map_.next_value()?);
                        }
                        GeneratedField::Sections => {
                            if sections__.is_some() {
                                return Err(serde::de::Error::duplicate_field("sections"));
                            }
                            sections__ = Some(map_.next_value()?);
                        }
                        GeneratedField::ClipStartTick => {
                            if clip_start_tick__.is_some() {
                                return Err(serde::de::Error::duplicate_field("clipStartTick"));
                            }
                            clip_start_tick__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::ClipLengthTicks => {
                            if clip_length_ticks__.is_some() {
                                return Err(serde::de::Error::duplicate_field("clipLengthTicks"));
                            }
                            clip_length_ticks__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                    }
                }
                Ok(CompileRequest {
                    kind: kind__.unwrap_or_default(),
                    source: source__.unwrap_or_default(),
                    seed: seed__.unwrap_or_default(),
                    params: params__.unwrap_or_default(),
                    tempo: tempo__.unwrap_or_default(),
                    signature: signature__.unwrap_or_default(),
                    sections: sections__.unwrap_or_default(),
                    clip_start_tick: clip_start_tick__.unwrap_or_default(),
                    clip_length_ticks: clip_length_ticks__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("escribass.generate.v1.CompileRequest", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for CompileResponse {
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
        if self.result.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("escribass.generate.v1.CompileResponse", len)?;
        if true {
            struct_ser.serialize_field("dsl_version", &self.dsl_version)?;
        }
        if true {
            struct_ser.serialize_field("python_version", &self.python_version)?;
        }
        if let Some(v) = self.result.as_ref() {
            match v {
                compile_response::Result::Notes(v) => {
                    struct_ser.serialize_field("notes", v)?;
                }
                compile_response::Result::Diagnostic(v) => {
                    struct_ser.serialize_field("diagnostic", v)?;
                }
            }
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for CompileResponse {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "dsl_version",
            "dslVersion",
            "python_version",
            "pythonVersion",
            "notes",
            "diagnostic",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            DslVersion,
            PythonVersion,
            Notes,
            Diagnostic,
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
                            "dslVersion" | "dsl_version" => Ok(GeneratedField::DslVersion),
                            "pythonVersion" | "python_version" => Ok(GeneratedField::PythonVersion),
                            "notes" => Ok(GeneratedField::Notes),
                            "diagnostic" => Ok(GeneratedField::Diagnostic),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = CompileResponse;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct escribass.generate.v1.CompileResponse")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<CompileResponse, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut dsl_version__ = None;
                let mut python_version__ = None;
                let mut result__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::DslVersion => {
                            if dsl_version__.is_some() {
                                return Err(serde::de::Error::duplicate_field("dslVersion"));
                            }
                            dsl_version__ = Some(map_.next_value()?);
                        }
                        GeneratedField::PythonVersion => {
                            if python_version__.is_some() {
                                return Err(serde::de::Error::duplicate_field("pythonVersion"));
                            }
                            python_version__ = Some(map_.next_value()?);
                        }
                        GeneratedField::Notes => {
                            if result__.is_some() {
                                return Err(serde::de::Error::duplicate_field("notes"));
                            }
                            result__ = map_.next_value::<::std::option::Option<_>>()?.map(compile_response::Result::Notes)
;
                        }
                        GeneratedField::Diagnostic => {
                            if result__.is_some() {
                                return Err(serde::de::Error::duplicate_field("diagnostic"));
                            }
                            result__ = map_.next_value::<::std::option::Option<_>>()?.map(compile_response::Result::Diagnostic)
;
                        }
                    }
                }
                Ok(CompileResponse {
                    dsl_version: dsl_version__.unwrap_or_default(),
                    python_version: python_version__.unwrap_or_default(),
                    result: result__,
                })
            }
        }
        deserializer.deserialize_struct("escribass.generate.v1.CompileResponse", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for Diagnostic {
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
        let mut struct_ser = serializer.serialize_struct("escribass.generate.v1.Diagnostic", len)?;
        if true {
            struct_ser.serialize_field("line", &self.line)?;
        }
        if true {
            struct_ser.serialize_field("column", &self.column)?;
        }
        if true {
            struct_ser.serialize_field("message", &self.message)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for Diagnostic {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "line",
            "column",
            "message",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Line,
            Column,
            Message,
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
                            "line" => Ok(GeneratedField::Line),
                            "column" => Ok(GeneratedField::Column),
                            "message" => Ok(GeneratedField::Message),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = Diagnostic;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct escribass.generate.v1.Diagnostic")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<Diagnostic, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut line__ = None;
                let mut column__ = None;
                let mut message__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Line => {
                            if line__.is_some() {
                                return Err(serde::de::Error::duplicate_field("line"));
                            }
                            line__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::Column => {
                            if column__.is_some() {
                                return Err(serde::de::Error::duplicate_field("column"));
                            }
                            column__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::Message => {
                            if message__.is_some() {
                                return Err(serde::de::Error::duplicate_field("message"));
                            }
                            message__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(Diagnostic {
                    line: line__.unwrap_or_default(),
                    column: column__.unwrap_or_default(),
                    message: message__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("escribass.generate.v1.Diagnostic", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for Notes {
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
        let mut struct_ser = serializer.serialize_struct("escribass.generate.v1.Notes", len)?;
        if true {
            struct_ser.serialize_field("notes", &self.notes)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for Notes {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "notes",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Notes,
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
                            "notes" => Ok(GeneratedField::Notes),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = Notes;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct escribass.generate.v1.Notes")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<Notes, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut notes__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Notes => {
                            if notes__.is_some() {
                                return Err(serde::de::Error::duplicate_field("notes"));
                            }
                            notes__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(Notes {
                    notes: notes__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("escribass.generate.v1.Notes", FIELDS, GeneratedVisitor)
    }
}

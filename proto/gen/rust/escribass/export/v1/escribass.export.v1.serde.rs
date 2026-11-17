// @generated
impl serde::Serialize for Bundle {
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
        let mut struct_ser = serializer.serialize_struct("escribass.export.v1.Bundle", len)?;
        if true {
            let v: std::collections::HashMap<_, _> = self.files.iter()
                .map(|(k, v)| (k, pbjson::private::base64::encode(v))).collect();
            struct_ser.serialize_field("files", &v)?;
        }
        if true {
            struct_ser.serialize_field("endpoints", &self.endpoints)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for Bundle {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "files",
            "endpoints",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Files,
            Endpoints,
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
                            "files" => Ok(GeneratedField::Files),
                            "endpoints" => Ok(GeneratedField::Endpoints),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = Bundle;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct escribass.export.v1.Bundle")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<Bundle, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut files__ = None;
                let mut endpoints__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Files => {
                            if files__.is_some() {
                                return Err(serde::de::Error::duplicate_field("files"));
                            }
                            files__ = Some(
                                map_.next_value::<std::collections::BTreeMap<_, ::pbjson::private::BytesDeserialize<_>>>()?
                                    .into_iter().map(|(k,v)| (k, v.0)).collect()
                            );
                        }
                        GeneratedField::Endpoints => {
                            if endpoints__.is_some() {
                                return Err(serde::de::Error::duplicate_field("endpoints"));
                            }
                            endpoints__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(Bundle {
                    files: files__.unwrap_or_default(),
                    endpoints: endpoints__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("escribass.export.v1.Bundle", FIELDS, GeneratedVisitor)
    }
}
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
        if self.input.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("escribass.export.v1.CompileRequest", len)?;
        if true {
            let v = DeviceKind::try_from(self.kind)
                .map_err(|_| serde::ser::Error::custom(format!("Invalid variant {}", self.kind)))?;
            struct_ser.serialize_field("kind", &v)?;
        }
        if let Some(v) = self.expected.as_ref() {
            struct_ser.serialize_field("expected", v)?;
        }
        if let Some(v) = self.input.as_ref() {
            match v {
                compile_request::Input::CmajorSource(v) => {
                    struct_ser.serialize_field("cmajor_source", v)?;
                }
                compile_request::Input::OnnxModel(v) => {
                    #[allow(clippy::needless_borrow)]
                    #[allow(clippy::needless_borrows_for_generic_args)]
                    struct_ser.serialize_field("onnx_model", pbjson::private::base64::encode(&v).as_str())?;
                }
            }
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
            "expected",
            "cmajor_source",
            "cmajorSource",
            "onnx_model",
            "onnxModel",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Kind,
            Expected,
            CmajorSource,
            OnnxModel,
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
                            "expected" => Ok(GeneratedField::Expected),
                            "cmajorSource" | "cmajor_source" => Ok(GeneratedField::CmajorSource),
                            "onnxModel" | "onnx_model" => Ok(GeneratedField::OnnxModel),
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
                formatter.write_str("struct escribass.export.v1.CompileRequest")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<CompileRequest, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut kind__ = None;
                let mut expected__ = None;
                let mut input__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Kind => {
                            if kind__.is_some() {
                                return Err(serde::de::Error::duplicate_field("kind"));
                            }
                            kind__ = Some(map_.next_value::<DeviceKind>()? as i32);
                        }
                        GeneratedField::Expected => {
                            if expected__.is_some() {
                                return Err(serde::de::Error::duplicate_field("expected"));
                            }
                            expected__ = map_.next_value()?;
                        }
                        GeneratedField::CmajorSource => {
                            if input__.is_some() {
                                return Err(serde::de::Error::duplicate_field("cmajorSource"));
                            }
                            input__ = map_.next_value::<::std::option::Option<_>>()?.map(compile_request::Input::CmajorSource);
                        }
                        GeneratedField::OnnxModel => {
                            if input__.is_some() {
                                return Err(serde::de::Error::duplicate_field("onnxModel"));
                            }
                            input__ = map_.next_value::<::std::option::Option<::pbjson::private::BytesDeserialize<_>>>()?.map(|x| compile_request::Input::OnnxModel(x.0));
                        }
                    }
                }
                Ok(CompileRequest {
                    kind: kind__.unwrap_or_default(),
                    expected: expected__,
                    input: input__,
                })
            }
        }
        deserializer.deserialize_struct("escribass.export.v1.CompileRequest", FIELDS, GeneratedVisitor)
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
        if self.result.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("escribass.export.v1.CompileResponse", len)?;
        if let Some(v) = self.actual.as_ref() {
            struct_ser.serialize_field("actual", v)?;
        }
        if let Some(v) = self.result.as_ref() {
            match v {
                compile_response::Result::Bundle(v) => {
                    struct_ser.serialize_field("bundle", v)?;
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
            "actual",
            "bundle",
            "diagnostic",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Actual,
            Bundle,
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
                            "actual" => Ok(GeneratedField::Actual),
                            "bundle" => Ok(GeneratedField::Bundle),
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
                formatter.write_str("struct escribass.export.v1.CompileResponse")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<CompileResponse, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut actual__ = None;
                let mut result__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Actual => {
                            if actual__.is_some() {
                                return Err(serde::de::Error::duplicate_field("actual"));
                            }
                            actual__ = map_.next_value()?;
                        }
                        GeneratedField::Bundle => {
                            if result__.is_some() {
                                return Err(serde::de::Error::duplicate_field("bundle"));
                            }
                            result__ = map_.next_value::<::std::option::Option<_>>()?.map(compile_response::Result::Bundle)
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
                    actual: actual__,
                    result: result__,
                })
            }
        }
        deserializer.deserialize_struct("escribass.export.v1.CompileResponse", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for ConvertedModel {
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
        let mut struct_ser = serializer.serialize_struct("escribass.export.v1.ConvertedModel", len)?;
        if true {
            struct_ser.serialize_field("model_hash", &self.model_hash)?;
        }
        if true {
            struct_ser.serialize_field("conversion_hash", &self.conversion_hash)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for ConvertedModel {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "model_hash",
            "modelHash",
            "conversion_hash",
            "conversionHash",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            ModelHash,
            ConversionHash,
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
                            "modelHash" | "model_hash" => Ok(GeneratedField::ModelHash),
                            "conversionHash" | "conversion_hash" => Ok(GeneratedField::ConversionHash),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = ConvertedModel;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct escribass.export.v1.ConvertedModel")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<ConvertedModel, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut model_hash__ = None;
                let mut conversion_hash__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::ModelHash => {
                            if model_hash__.is_some() {
                                return Err(serde::de::Error::duplicate_field("modelHash"));
                            }
                            model_hash__ = Some(map_.next_value()?);
                        }
                        GeneratedField::ConversionHash => {
                            if conversion_hash__.is_some() {
                                return Err(serde::de::Error::duplicate_field("conversionHash"));
                            }
                            conversion_hash__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(ConvertedModel {
                    model_hash: model_hash__.unwrap_or_default(),
                    conversion_hash: conversion_hash__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("escribass.export.v1.ConvertedModel", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for DeviceKind {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let variant = match self {
            Self::Unspecified => "DEVICE_KIND_UNSPECIFIED",
            Self::Instrument => "DEVICE_KIND_INSTRUMENT",
            Self::Effect => "DEVICE_KIND_EFFECT",
        };
        serializer.serialize_str(variant)
    }
}
impl<'de> serde::Deserialize<'de> for DeviceKind {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "DEVICE_KIND_UNSPECIFIED",
            "DEVICE_KIND_INSTRUMENT",
            "DEVICE_KIND_EFFECT",
        ];

        struct GeneratedVisitor;

        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = DeviceKind;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(formatter, "expected one of: {:?}", &FIELDS)
            }

            fn visit_i64<E>(self, v: i64) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                i32::try_from(v)
                    .ok()
                    .and_then(|x| x.try_into().ok())
                    .ok_or_else(|| {
                        serde::de::Error::invalid_value(serde::de::Unexpected::Signed(v), &self)
                    })
            }

            fn visit_u64<E>(self, v: u64) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                i32::try_from(v)
                    .ok()
                    .and_then(|x| x.try_into().ok())
                    .ok_or_else(|| {
                        serde::de::Error::invalid_value(serde::de::Unexpected::Unsigned(v), &self)
                    })
            }

            fn visit_str<E>(self, value: &str) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                match value {
                    "DEVICE_KIND_UNSPECIFIED" => Ok(DeviceKind::Unspecified),
                    "DEVICE_KIND_INSTRUMENT" => Ok(DeviceKind::Instrument),
                    "DEVICE_KIND_EFFECT" => Ok(DeviceKind::Effect),
                    _ => Err(serde::de::Error::unknown_variant(value, FIELDS)),
                }
            }
        }
        deserializer.deserialize_any(GeneratedVisitor)
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
        if true {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("escribass.export.v1.Diagnostic", len)?;
        if true {
            struct_ser.serialize_field("stage", &self.stage)?;
        }
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
            "stage",
            "line",
            "column",
            "message",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Stage,
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
                            "stage" => Ok(GeneratedField::Stage),
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
                formatter.write_str("struct escribass.export.v1.Diagnostic")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<Diagnostic, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut stage__ = None;
                let mut line__ = None;
                let mut column__ = None;
                let mut message__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Stage => {
                            if stage__.is_some() {
                                return Err(serde::de::Error::duplicate_field("stage"));
                            }
                            stage__ = Some(map_.next_value()?);
                        }
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
                    stage: stage__.unwrap_or_default(),
                    line: line__.unwrap_or_default(),
                    column: column__.unwrap_or_default(),
                    message: message__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("escribass.export.v1.Diagnostic", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for Endpoint {
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
        let mut struct_ser = serializer.serialize_struct("escribass.export.v1.Endpoint", len)?;
        if true {
            struct_ser.serialize_field("identifier", &self.identifier)?;
        }
        if true {
            struct_ser.serialize_field("handle", &self.handle)?;
        }
        if true {
            struct_ser.serialize_field("annotation", &self.annotation)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for Endpoint {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "identifier",
            "handle",
            "annotation",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Identifier,
            Handle,
            Annotation,
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
                            "identifier" => Ok(GeneratedField::Identifier),
                            "handle" => Ok(GeneratedField::Handle),
                            "annotation" => Ok(GeneratedField::Annotation),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = Endpoint;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct escribass.export.v1.Endpoint")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<Endpoint, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut identifier__ = None;
                let mut handle__ = None;
                let mut annotation__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Identifier => {
                            if identifier__.is_some() {
                                return Err(serde::de::Error::duplicate_field("identifier"));
                            }
                            identifier__ = Some(map_.next_value()?);
                        }
                        GeneratedField::Handle => {
                            if handle__.is_some() {
                                return Err(serde::de::Error::duplicate_field("handle"));
                            }
                            handle__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::Annotation => {
                            if annotation__.is_some() {
                                return Err(serde::de::Error::duplicate_field("annotation"));
                            }
                            annotation__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(Endpoint {
                    identifier: identifier__.unwrap_or_default(),
                    handle: handle__.unwrap_or_default(),
                    annotation: annotation__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("escribass.export.v1.Endpoint", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for ExportInput {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.built_from.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("escribass.export.v1.ExportInput", len)?;
        if let Some(v) = self.built_from.as_ref() {
            match v {
                export_input::BuiltFrom::SourceHash(v) => {
                    struct_ser.serialize_field("source_hash", v)?;
                }
                export_input::BuiltFrom::Model(v) => {
                    struct_ser.serialize_field("model", v)?;
                }
            }
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for ExportInput {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "source_hash",
            "sourceHash",
            "model",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            SourceHash,
            Model,
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
                            "sourceHash" | "source_hash" => Ok(GeneratedField::SourceHash),
                            "model" => Ok(GeneratedField::Model),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = ExportInput;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct escribass.export.v1.ExportInput")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<ExportInput, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut built_from__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::SourceHash => {
                            if built_from__.is_some() {
                                return Err(serde::de::Error::duplicate_field("sourceHash"));
                            }
                            built_from__ = map_.next_value::<::std::option::Option<_>>()?.map(export_input::BuiltFrom::SourceHash);
                        }
                        GeneratedField::Model => {
                            if built_from__.is_some() {
                                return Err(serde::de::Error::duplicate_field("model"));
                            }
                            built_from__ = map_.next_value::<::std::option::Option<_>>()?.map(export_input::BuiltFrom::Model)
;
                        }
                    }
                }
                Ok(ExportInput {
                    built_from: built_from__,
                })
            }
        }
        deserializer.deserialize_struct("escribass.export.v1.ExportInput", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for ExportRecord {
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
        let mut struct_ser = serializer.serialize_struct("escribass.export.v1.ExportRecord", len)?;
        if let Some(v) = self.input.as_ref() {
            struct_ser.serialize_field("input", v)?;
        }
        if true {
            struct_ser.serialize_field("target", &self.target)?;
        }
        if let Some(v) = self.recipe.as_ref() {
            struct_ser.serialize_field("recipe", v)?;
        }
        if true {
            struct_ser.serialize_field("members", &self.members)?;
        }
        if let Some(v) = self.class.as_ref() {
            struct_ser.serialize_field("class", v)?;
        }
        if let Some(v) = self.ports.as_ref() {
            struct_ser.serialize_field("ports", v)?;
        }
        if true {
            struct_ser.serialize_field("params", &self.params)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for ExportRecord {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "input",
            "target",
            "recipe",
            "members",
            "class",
            "ports",
            "params",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Input,
            Target,
            Recipe,
            Members,
            Class,
            Ports,
            Params,
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
                            "input" => Ok(GeneratedField::Input),
                            "target" => Ok(GeneratedField::Target),
                            "recipe" => Ok(GeneratedField::Recipe),
                            "members" => Ok(GeneratedField::Members),
                            "class" => Ok(GeneratedField::Class),
                            "ports" => Ok(GeneratedField::Ports),
                            "params" => Ok(GeneratedField::Params),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = ExportRecord;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct escribass.export.v1.ExportRecord")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<ExportRecord, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut input__ = None;
                let mut target__ = None;
                let mut recipe__ = None;
                let mut members__ = None;
                let mut class__ = None;
                let mut ports__ = None;
                let mut params__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Input => {
                            if input__.is_some() {
                                return Err(serde::de::Error::duplicate_field("input"));
                            }
                            input__ = map_.next_value()?;
                        }
                        GeneratedField::Target => {
                            if target__.is_some() {
                                return Err(serde::de::Error::duplicate_field("target"));
                            }
                            target__ = Some(map_.next_value()?);
                        }
                        GeneratedField::Recipe => {
                            if recipe__.is_some() {
                                return Err(serde::de::Error::duplicate_field("recipe"));
                            }
                            recipe__ = map_.next_value()?;
                        }
                        GeneratedField::Members => {
                            if members__.is_some() {
                                return Err(serde::de::Error::duplicate_field("members"));
                            }
                            members__ = Some(
                                map_.next_value::<std::collections::BTreeMap<_, _>>()?
                            );
                        }
                        GeneratedField::Class => {
                            if class__.is_some() {
                                return Err(serde::de::Error::duplicate_field("class"));
                            }
                            class__ = map_.next_value()?;
                        }
                        GeneratedField::Ports => {
                            if ports__.is_some() {
                                return Err(serde::de::Error::duplicate_field("ports"));
                            }
                            ports__ = map_.next_value()?;
                        }
                        GeneratedField::Params => {
                            if params__.is_some() {
                                return Err(serde::de::Error::duplicate_field("params"));
                            }
                            params__ = Some(
                                map_.next_value::<std::collections::BTreeMap<_, _>>()?
                            );
                        }
                    }
                }
                Ok(ExportRecord {
                    input: input__,
                    target: target__.unwrap_or_default(),
                    recipe: recipe__,
                    members: members__.unwrap_or_default(),
                    class: class__,
                    ports: ports__,
                    params: params__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("escribass.export.v1.ExportRecord", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for Param {
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
        let mut struct_ser = serializer.serialize_struct("escribass.export.v1.Param", len)?;
        if true {
            struct_ser.serialize_field("param_id", &self.param_id)?;
        }
        if true {
            struct_ser.serialize_field("title", &self.title)?;
        }
        if true {
            struct_ser.serialize_field("min", &self.min)?;
        }
        if true {
            struct_ser.serialize_field("max", &self.max)?;
        }
        if true {
            struct_ser.serialize_field("init", &self.init)?;
        }
        if let Some(v) = self.step.as_ref() {
            struct_ser.serialize_field("step", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for Param {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "param_id",
            "paramId",
            "title",
            "min",
            "max",
            "init",
            "step",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            ParamId,
            Title,
            Min,
            Max,
            Init,
            Step,
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
                            "paramId" | "param_id" => Ok(GeneratedField::ParamId),
                            "title" => Ok(GeneratedField::Title),
                            "min" => Ok(GeneratedField::Min),
                            "max" => Ok(GeneratedField::Max),
                            "init" => Ok(GeneratedField::Init),
                            "step" => Ok(GeneratedField::Step),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = Param;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct escribass.export.v1.Param")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<Param, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut param_id__ = None;
                let mut title__ = None;
                let mut min__ = None;
                let mut max__ = None;
                let mut init__ = None;
                let mut step__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::ParamId => {
                            if param_id__.is_some() {
                                return Err(serde::de::Error::duplicate_field("paramId"));
                            }
                            param_id__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::Title => {
                            if title__.is_some() {
                                return Err(serde::de::Error::duplicate_field("title"));
                            }
                            title__ = Some(map_.next_value()?);
                        }
                        GeneratedField::Min => {
                            if min__.is_some() {
                                return Err(serde::de::Error::duplicate_field("min"));
                            }
                            min__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::Max => {
                            if max__.is_some() {
                                return Err(serde::de::Error::duplicate_field("max"));
                            }
                            max__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::Init => {
                            if init__.is_some() {
                                return Err(serde::de::Error::duplicate_field("init"));
                            }
                            init__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::Step => {
                            if step__.is_some() {
                                return Err(serde::de::Error::duplicate_field("step"));
                            }
                            step__ = 
                                map_.next_value::<::std::option::Option<::pbjson::private::NumberDeserialize<_>>>()?.map(|x| x.0)
                            ;
                        }
                    }
                }
                Ok(Param {
                    param_id: param_id__.unwrap_or_default(),
                    title: title__.unwrap_or_default(),
                    min: min__.unwrap_or_default(),
                    max: max__.unwrap_or_default(),
                    init: init__.unwrap_or_default(),
                    step: step__,
                })
            }
        }
        deserializer.deserialize_struct("escribass.export.v1.Param", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for PluginClass {
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
        let mut struct_ser = serializer.serialize_struct("escribass.export.v1.PluginClass", len)?;
        if true {
            struct_ser.serialize_field("vendor", &self.vendor)?;
        }
        if true {
            struct_ser.serialize_field("name", &self.name)?;
        }
        if true {
            struct_ser.serialize_field("vst3_class_id", &self.vst3_class_id)?;
        }
        if true {
            struct_ser.serialize_field("clap_id", &self.clap_id)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for PluginClass {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "vendor",
            "name",
            "vst3_class_id",
            "vst3ClassId",
            "clap_id",
            "clapId",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Vendor,
            Name,
            Vst3ClassId,
            ClapId,
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
                            "vendor" => Ok(GeneratedField::Vendor),
                            "name" => Ok(GeneratedField::Name),
                            "vst3ClassId" | "vst3_class_id" => Ok(GeneratedField::Vst3ClassId),
                            "clapId" | "clap_id" => Ok(GeneratedField::ClapId),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = PluginClass;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct escribass.export.v1.PluginClass")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<PluginClass, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut vendor__ = None;
                let mut name__ = None;
                let mut vst3_class_id__ = None;
                let mut clap_id__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Vendor => {
                            if vendor__.is_some() {
                                return Err(serde::de::Error::duplicate_field("vendor"));
                            }
                            vendor__ = Some(map_.next_value()?);
                        }
                        GeneratedField::Name => {
                            if name__.is_some() {
                                return Err(serde::de::Error::duplicate_field("name"));
                            }
                            name__ = Some(map_.next_value()?);
                        }
                        GeneratedField::Vst3ClassId => {
                            if vst3_class_id__.is_some() {
                                return Err(serde::de::Error::duplicate_field("vst3ClassId"));
                            }
                            vst3_class_id__ = Some(map_.next_value()?);
                        }
                        GeneratedField::ClapId => {
                            if clap_id__.is_some() {
                                return Err(serde::de::Error::duplicate_field("clapId"));
                            }
                            clap_id__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(PluginClass {
                    vendor: vendor__.unwrap_or_default(),
                    name: name__.unwrap_or_default(),
                    vst3_class_id: vst3_class_id__.unwrap_or_default(),
                    clap_id: clap_id__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("escribass.export.v1.PluginClass", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for Ports {
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
        let mut struct_ser = serializer.serialize_struct("escribass.export.v1.Ports", len)?;
        if true {
            struct_ser.serialize_field("audio_input_channels", &self.audio_input_channels)?;
        }
        if true {
            struct_ser.serialize_field("audio_output_channels", &self.audio_output_channels)?;
        }
        if true {
            struct_ser.serialize_field("midi_input", &self.midi_input)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for Ports {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "audio_input_channels",
            "audioInputChannels",
            "audio_output_channels",
            "audioOutputChannels",
            "midi_input",
            "midiInput",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            AudioInputChannels,
            AudioOutputChannels,
            MidiInput,
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
                            "audioInputChannels" | "audio_input_channels" => Ok(GeneratedField::AudioInputChannels),
                            "audioOutputChannels" | "audio_output_channels" => Ok(GeneratedField::AudioOutputChannels),
                            "midiInput" | "midi_input" => Ok(GeneratedField::MidiInput),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = Ports;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct escribass.export.v1.Ports")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<Ports, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut audio_input_channels__ = None;
                let mut audio_output_channels__ = None;
                let mut midi_input__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::AudioInputChannels => {
                            if audio_input_channels__.is_some() {
                                return Err(serde::de::Error::duplicate_field("audioInputChannels"));
                            }
                            audio_input_channels__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::AudioOutputChannels => {
                            if audio_output_channels__.is_some() {
                                return Err(serde::de::Error::duplicate_field("audioOutputChannels"));
                            }
                            audio_output_channels__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::MidiInput => {
                            if midi_input__.is_some() {
                                return Err(serde::de::Error::duplicate_field("midiInput"));
                            }
                            midi_input__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(Ports {
                    audio_input_channels: audio_input_channels__.unwrap_or_default(),
                    audio_output_channels: audio_output_channels__.unwrap_or_default(),
                    midi_input: midi_input__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("escribass.export.v1.Ports", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for Recipe {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let len = 0;
        let struct_ser = serializer.serialize_struct("escribass.export.v1.Recipe", len)?;
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for Recipe {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
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
                            Err(serde::de::Error::unknown_field(value, FIELDS))
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = Recipe;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct escribass.export.v1.Recipe")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<Recipe, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                while map_.next_key::<GeneratedField>()?.is_some() {
                    let _ = map_.next_value::<serde::de::IgnoredAny>()?;
                }
                Ok(Recipe {
                })
            }
        }
        deserializer.deserialize_struct("escribass.export.v1.Recipe", FIELDS, GeneratedVisitor)
    }
}

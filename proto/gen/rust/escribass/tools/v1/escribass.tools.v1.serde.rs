// @generated
impl serde::Serialize for AddAssetRequest {
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
        let mut struct_ser = serializer.serialize_struct("escribass.tools.v1.AddAssetRequest", len)?;
        if true {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("content", pbjson::private::base64::encode(&self.content).as_str())?;
        }
        if true {
            struct_ser.serialize_field("dry_run", &self.dry_run)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for AddAssetRequest {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "content",
            "dry_run",
            "dryRun",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Content,
            DryRun,
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
                            "content" => Ok(GeneratedField::Content),
                            "dryRun" | "dry_run" => Ok(GeneratedField::DryRun),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = AddAssetRequest;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct escribass.tools.v1.AddAssetRequest")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<AddAssetRequest, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut content__ = None;
                let mut dry_run__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Content => {
                            if content__.is_some() {
                                return Err(serde::de::Error::duplicate_field("content"));
                            }
                            content__ = 
                                Some(map_.next_value::<::pbjson::private::BytesDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::DryRun => {
                            if dry_run__.is_some() {
                                return Err(serde::de::Error::duplicate_field("dryRun"));
                            }
                            dry_run__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(AddAssetRequest {
                    content: content__.unwrap_or_default(),
                    dry_run: dry_run__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("escribass.tools.v1.AddAssetRequest", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for AddAutomationRequest {
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
        let mut struct_ser = serializer.serialize_struct("escribass.tools.v1.AddAutomationRequest", len)?;
        if let Some(v) = self.target.as_ref() {
            struct_ser.serialize_field("target", v)?;
        }
        if true {
            struct_ser.serialize_field("points", &self.points)?;
        }
        if true {
            struct_ser.serialize_field("dry_run", &self.dry_run)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for AddAutomationRequest {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "target",
            "points",
            "dry_run",
            "dryRun",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Target,
            Points,
            DryRun,
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
                            "target" => Ok(GeneratedField::Target),
                            "points" => Ok(GeneratedField::Points),
                            "dryRun" | "dry_run" => Ok(GeneratedField::DryRun),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = AddAutomationRequest;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct escribass.tools.v1.AddAutomationRequest")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<AddAutomationRequest, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut target__ = None;
                let mut points__ = None;
                let mut dry_run__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Target => {
                            if target__.is_some() {
                                return Err(serde::de::Error::duplicate_field("target"));
                            }
                            target__ = map_.next_value()?;
                        }
                        GeneratedField::Points => {
                            if points__.is_some() {
                                return Err(serde::de::Error::duplicate_field("points"));
                            }
                            points__ = Some(
                                map_.next_value::<std::collections::BTreeMap<_, _>>()?
                            );
                        }
                        GeneratedField::DryRun => {
                            if dry_run__.is_some() {
                                return Err(serde::de::Error::duplicate_field("dryRun"));
                            }
                            dry_run__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(AddAutomationRequest {
                    target: target__,
                    points: points__.unwrap_or_default(),
                    dry_run: dry_run__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("escribass.tools.v1.AddAutomationRequest", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for AddClipRequest {
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
        if self.content.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("escribass.tools.v1.AddClipRequest", len)?;
        if true {
            struct_ser.serialize_field("track_id", &self.track_id)?;
        }
        if true {
            struct_ser.serialize_field("start_tick", &self.start_tick)?;
        }
        if true {
            struct_ser.serialize_field("length_ticks", &self.length_ticks)?;
        }
        if true {
            struct_ser.serialize_field("dry_run", &self.dry_run)?;
        }
        if let Some(v) = self.content.as_ref() {
            match v {
                add_clip_request::Content::NoteClip(v) => {
                    struct_ser.serialize_field("note_clip", v)?;
                }
                add_clip_request::Content::AudioClip(v) => {
                    struct_ser.serialize_field("audio_clip", v)?;
                }
            }
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for AddClipRequest {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "track_id",
            "trackId",
            "start_tick",
            "startTick",
            "length_ticks",
            "lengthTicks",
            "dry_run",
            "dryRun",
            "note_clip",
            "noteClip",
            "audio_clip",
            "audioClip",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            TrackId,
            StartTick,
            LengthTicks,
            DryRun,
            NoteClip,
            AudioClip,
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
                            "trackId" | "track_id" => Ok(GeneratedField::TrackId),
                            "startTick" | "start_tick" => Ok(GeneratedField::StartTick),
                            "lengthTicks" | "length_ticks" => Ok(GeneratedField::LengthTicks),
                            "dryRun" | "dry_run" => Ok(GeneratedField::DryRun),
                            "noteClip" | "note_clip" => Ok(GeneratedField::NoteClip),
                            "audioClip" | "audio_clip" => Ok(GeneratedField::AudioClip),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = AddClipRequest;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct escribass.tools.v1.AddClipRequest")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<AddClipRequest, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut track_id__ = None;
                let mut start_tick__ = None;
                let mut length_ticks__ = None;
                let mut dry_run__ = None;
                let mut content__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::TrackId => {
                            if track_id__.is_some() {
                                return Err(serde::de::Error::duplicate_field("trackId"));
                            }
                            track_id__ = Some(map_.next_value()?);
                        }
                        GeneratedField::StartTick => {
                            if start_tick__.is_some() {
                                return Err(serde::de::Error::duplicate_field("startTick"));
                            }
                            start_tick__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::LengthTicks => {
                            if length_ticks__.is_some() {
                                return Err(serde::de::Error::duplicate_field("lengthTicks"));
                            }
                            length_ticks__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::DryRun => {
                            if dry_run__.is_some() {
                                return Err(serde::de::Error::duplicate_field("dryRun"));
                            }
                            dry_run__ = Some(map_.next_value()?);
                        }
                        GeneratedField::NoteClip => {
                            if content__.is_some() {
                                return Err(serde::de::Error::duplicate_field("noteClip"));
                            }
                            content__ = map_.next_value::<::std::option::Option<_>>()?.map(add_clip_request::Content::NoteClip)
;
                        }
                        GeneratedField::AudioClip => {
                            if content__.is_some() {
                                return Err(serde::de::Error::duplicate_field("audioClip"));
                            }
                            content__ = map_.next_value::<::std::option::Option<_>>()?.map(add_clip_request::Content::AudioClip)
;
                        }
                    }
                }
                Ok(AddClipRequest {
                    track_id: track_id__.unwrap_or_default(),
                    start_tick: start_tick__.unwrap_or_default(),
                    length_ticks: length_ticks__.unwrap_or_default(),
                    dry_run: dry_run__.unwrap_or_default(),
                    content: content__,
                })
            }
        }
        deserializer.deserialize_struct("escribass.tools.v1.AddClipRequest", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for AddEffectRequest {
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
        let mut struct_ser = serializer.serialize_struct("escribass.tools.v1.AddEffectRequest", len)?;
        if true {
            struct_ser.serialize_field("track_id", &self.track_id)?;
        }
        if let Some(v) = self.r#ref.as_ref() {
            struct_ser.serialize_field("ref", v)?;
        }
        if let Some(v) = self.index.as_ref() {
            struct_ser.serialize_field("index", v)?;
        }
        if true {
            struct_ser.serialize_field("dry_run", &self.dry_run)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for AddEffectRequest {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "track_id",
            "trackId",
            "ref",
            "index",
            "dry_run",
            "dryRun",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            TrackId,
            Ref,
            Index,
            DryRun,
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
                            "trackId" | "track_id" => Ok(GeneratedField::TrackId),
                            "ref" => Ok(GeneratedField::Ref),
                            "index" => Ok(GeneratedField::Index),
                            "dryRun" | "dry_run" => Ok(GeneratedField::DryRun),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = AddEffectRequest;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct escribass.tools.v1.AddEffectRequest")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<AddEffectRequest, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut track_id__ = None;
                let mut r#ref__ = None;
                let mut index__ = None;
                let mut dry_run__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::TrackId => {
                            if track_id__.is_some() {
                                return Err(serde::de::Error::duplicate_field("trackId"));
                            }
                            track_id__ = Some(map_.next_value()?);
                        }
                        GeneratedField::Ref => {
                            if r#ref__.is_some() {
                                return Err(serde::de::Error::duplicate_field("ref"));
                            }
                            r#ref__ = map_.next_value()?;
                        }
                        GeneratedField::Index => {
                            if index__.is_some() {
                                return Err(serde::de::Error::duplicate_field("index"));
                            }
                            index__ = 
                                map_.next_value::<::std::option::Option<::pbjson::private::NumberDeserialize<_>>>()?.map(|x| x.0)
                            ;
                        }
                        GeneratedField::DryRun => {
                            if dry_run__.is_some() {
                                return Err(serde::de::Error::duplicate_field("dryRun"));
                            }
                            dry_run__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(AddEffectRequest {
                    track_id: track_id__.unwrap_or_default(),
                    r#ref: r#ref__,
                    index: index__,
                    dry_run: dry_run__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("escribass.tools.v1.AddEffectRequest", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for AddSectionRequest {
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
        let mut struct_ser = serializer.serialize_struct("escribass.tools.v1.AddSectionRequest", len)?;
        if true {
            struct_ser.serialize_field("name", &self.name)?;
        }
        if true {
            struct_ser.serialize_field("start_tick", &self.start_tick)?;
        }
        if true {
            struct_ser.serialize_field("end_tick", &self.end_tick)?;
        }
        if true {
            struct_ser.serialize_field("dry_run", &self.dry_run)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for AddSectionRequest {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "name",
            "start_tick",
            "startTick",
            "end_tick",
            "endTick",
            "dry_run",
            "dryRun",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Name,
            StartTick,
            EndTick,
            DryRun,
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
                            "name" => Ok(GeneratedField::Name),
                            "startTick" | "start_tick" => Ok(GeneratedField::StartTick),
                            "endTick" | "end_tick" => Ok(GeneratedField::EndTick),
                            "dryRun" | "dry_run" => Ok(GeneratedField::DryRun),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = AddSectionRequest;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct escribass.tools.v1.AddSectionRequest")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<AddSectionRequest, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut name__ = None;
                let mut start_tick__ = None;
                let mut end_tick__ = None;
                let mut dry_run__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Name => {
                            if name__.is_some() {
                                return Err(serde::de::Error::duplicate_field("name"));
                            }
                            name__ = Some(map_.next_value()?);
                        }
                        GeneratedField::StartTick => {
                            if start_tick__.is_some() {
                                return Err(serde::de::Error::duplicate_field("startTick"));
                            }
                            start_tick__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::EndTick => {
                            if end_tick__.is_some() {
                                return Err(serde::de::Error::duplicate_field("endTick"));
                            }
                            end_tick__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::DryRun => {
                            if dry_run__.is_some() {
                                return Err(serde::de::Error::duplicate_field("dryRun"));
                            }
                            dry_run__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(AddSectionRequest {
                    name: name__.unwrap_or_default(),
                    start_tick: start_tick__.unwrap_or_default(),
                    end_tick: end_tick__.unwrap_or_default(),
                    dry_run: dry_run__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("escribass.tools.v1.AddSectionRequest", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for AddTrackRequest {
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
        let mut struct_ser = serializer.serialize_struct("escribass.tools.v1.AddTrackRequest", len)?;
        if true {
            struct_ser.serialize_field("name", &self.name)?;
        }
        if true {
            let v = ::escribass_schema::song::TrackKind::try_from(self.kind)
                .map_err(|_| serde::ser::Error::custom(format!("Invalid variant {}", self.kind)))?;
            struct_ser.serialize_field("kind", &v)?;
        }
        if let Some(v) = self.r#ref.as_ref() {
            struct_ser.serialize_field("ref", v)?;
        }
        if true {
            struct_ser.serialize_field("dry_run", &self.dry_run)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for AddTrackRequest {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "name",
            "kind",
            "ref",
            "dry_run",
            "dryRun",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Name,
            Kind,
            Ref,
            DryRun,
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
                            "name" => Ok(GeneratedField::Name),
                            "kind" => Ok(GeneratedField::Kind),
                            "ref" => Ok(GeneratedField::Ref),
                            "dryRun" | "dry_run" => Ok(GeneratedField::DryRun),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = AddTrackRequest;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct escribass.tools.v1.AddTrackRequest")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<AddTrackRequest, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut name__ = None;
                let mut kind__ = None;
                let mut r#ref__ = None;
                let mut dry_run__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Name => {
                            if name__.is_some() {
                                return Err(serde::de::Error::duplicate_field("name"));
                            }
                            name__ = Some(map_.next_value()?);
                        }
                        GeneratedField::Kind => {
                            if kind__.is_some() {
                                return Err(serde::de::Error::duplicate_field("kind"));
                            }
                            kind__ = Some(map_.next_value::<::escribass_schema::song::TrackKind>()? as i32);
                        }
                        GeneratedField::Ref => {
                            if r#ref__.is_some() {
                                return Err(serde::de::Error::duplicate_field("ref"));
                            }
                            r#ref__ = map_.next_value()?;
                        }
                        GeneratedField::DryRun => {
                            if dry_run__.is_some() {
                                return Err(serde::de::Error::duplicate_field("dryRun"));
                            }
                            dry_run__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(AddTrackRequest {
                    name: name__.unwrap_or_default(),
                    kind: kind__.unwrap_or_default(),
                    r#ref: r#ref__,
                    dry_run: dry_run__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("escribass.tools.v1.AddTrackRequest", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for ApplyPatchRequest {
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
        let mut struct_ser = serializer.serialize_struct("escribass.tools.v1.ApplyPatchRequest", len)?;
        if true {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("patch", pbjson::private::base64::encode(&self.patch).as_str())?;
        }
        if true {
            struct_ser.serialize_field("dry_run", &self.dry_run)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for ApplyPatchRequest {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "patch",
            "dry_run",
            "dryRun",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Patch,
            DryRun,
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
                            "patch" => Ok(GeneratedField::Patch),
                            "dryRun" | "dry_run" => Ok(GeneratedField::DryRun),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = ApplyPatchRequest;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct escribass.tools.v1.ApplyPatchRequest")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<ApplyPatchRequest, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut patch__ = None;
                let mut dry_run__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Patch => {
                            if patch__.is_some() {
                                return Err(serde::de::Error::duplicate_field("patch"));
                            }
                            patch__ = 
                                Some(map_.next_value::<::pbjson::private::BytesDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::DryRun => {
                            if dry_run__.is_some() {
                                return Err(serde::de::Error::duplicate_field("dryRun"));
                            }
                            dry_run__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(ApplyPatchRequest {
                    patch: patch__.unwrap_or_default(),
                    dry_run: dry_run__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("escribass.tools.v1.ApplyPatchRequest", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for AssetResponse {
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
        let mut struct_ser = serializer.serialize_struct("escribass.tools.v1.AssetResponse", len)?;
        if true {
            struct_ser.serialize_field("asset_hash", &self.asset_hash)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for AssetResponse {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "asset_hash",
            "assetHash",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            AssetHash,
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
                            "assetHash" | "asset_hash" => Ok(GeneratedField::AssetHash),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = AssetResponse;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct escribass.tools.v1.AssetResponse")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<AssetResponse, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut asset_hash__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::AssetHash => {
                            if asset_hash__.is_some() {
                                return Err(serde::de::Error::duplicate_field("assetHash"));
                            }
                            asset_hash__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(AssetResponse {
                    asset_hash: asset_hash__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("escribass.tools.v1.AssetResponse", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for CompileGeneratorRequest {
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
        let mut struct_ser = serializer.serialize_struct("escribass.tools.v1.CompileGeneratorRequest", len)?;
        if true {
            struct_ser.serialize_field("generator_id", &self.generator_id)?;
        }
        if true {
            struct_ser.serialize_field("dry_run", &self.dry_run)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for CompileGeneratorRequest {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "generator_id",
            "generatorId",
            "dry_run",
            "dryRun",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            GeneratorId,
            DryRun,
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
                            "generatorId" | "generator_id" => Ok(GeneratedField::GeneratorId),
                            "dryRun" | "dry_run" => Ok(GeneratedField::DryRun),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = CompileGeneratorRequest;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct escribass.tools.v1.CompileGeneratorRequest")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<CompileGeneratorRequest, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut generator_id__ = None;
                let mut dry_run__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::GeneratorId => {
                            if generator_id__.is_some() {
                                return Err(serde::de::Error::duplicate_field("generatorId"));
                            }
                            generator_id__ = Some(map_.next_value()?);
                        }
                        GeneratedField::DryRun => {
                            if dry_run__.is_some() {
                                return Err(serde::de::Error::duplicate_field("dryRun"));
                            }
                            dry_run__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(CompileGeneratorRequest {
                    generator_id: generator_id__.unwrap_or_default(),
                    dry_run: dry_run__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("escribass.tools.v1.CompileGeneratorRequest", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for CreateBranchRequest {
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
        let mut struct_ser = serializer.serialize_struct("escribass.tools.v1.CreateBranchRequest", len)?;
        if true {
            struct_ser.serialize_field("name", &self.name)?;
        }
        if true {
            struct_ser.serialize_field("at_entry_id", &self.at_entry_id)?;
        }
        if true {
            struct_ser.serialize_field("dry_run", &self.dry_run)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for CreateBranchRequest {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "name",
            "at_entry_id",
            "atEntryId",
            "dry_run",
            "dryRun",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Name,
            AtEntryId,
            DryRun,
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
                            "name" => Ok(GeneratedField::Name),
                            "atEntryId" | "at_entry_id" => Ok(GeneratedField::AtEntryId),
                            "dryRun" | "dry_run" => Ok(GeneratedField::DryRun),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = CreateBranchRequest;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct escribass.tools.v1.CreateBranchRequest")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<CreateBranchRequest, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut name__ = None;
                let mut at_entry_id__ = None;
                let mut dry_run__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Name => {
                            if name__.is_some() {
                                return Err(serde::de::Error::duplicate_field("name"));
                            }
                            name__ = Some(map_.next_value()?);
                        }
                        GeneratedField::AtEntryId => {
                            if at_entry_id__.is_some() {
                                return Err(serde::de::Error::duplicate_field("atEntryId"));
                            }
                            at_entry_id__ = Some(map_.next_value()?);
                        }
                        GeneratedField::DryRun => {
                            if dry_run__.is_some() {
                                return Err(serde::de::Error::duplicate_field("dryRun"));
                            }
                            dry_run__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(CreateBranchRequest {
                    name: name__.unwrap_or_default(),
                    at_entry_id: at_entry_id__.unwrap_or_default(),
                    dry_run: dry_run__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("escribass.tools.v1.CreateBranchRequest", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for DefineGeneratorRequest {
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
        if self.target.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("escribass.tools.v1.DefineGeneratorRequest", len)?;
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
            struct_ser.serialize_field("dry_run", &self.dry_run)?;
        }
        if let Some(v) = self.target.as_ref() {
            match v {
                define_generator_request::Target::TrackId(v) => {
                    struct_ser.serialize_field("track_id", v)?;
                }
                define_generator_request::Target::ClipId(v) => {
                    struct_ser.serialize_field("clip_id", v)?;
                }
            }
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for DefineGeneratorRequest {
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
            "dry_run",
            "dryRun",
            "track_id",
            "trackId",
            "clip_id",
            "clipId",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Kind,
            Source,
            Seed,
            Params,
            DryRun,
            TrackId,
            ClipId,
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
                            "dryRun" | "dry_run" => Ok(GeneratedField::DryRun),
                            "trackId" | "track_id" => Ok(GeneratedField::TrackId),
                            "clipId" | "clip_id" => Ok(GeneratedField::ClipId),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = DefineGeneratorRequest;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct escribass.tools.v1.DefineGeneratorRequest")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<DefineGeneratorRequest, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut kind__ = None;
                let mut source__ = None;
                let mut seed__ = None;
                let mut params__ = None;
                let mut dry_run__ = None;
                let mut target__ = None;
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
                        GeneratedField::DryRun => {
                            if dry_run__.is_some() {
                                return Err(serde::de::Error::duplicate_field("dryRun"));
                            }
                            dry_run__ = Some(map_.next_value()?);
                        }
                        GeneratedField::TrackId => {
                            if target__.is_some() {
                                return Err(serde::de::Error::duplicate_field("trackId"));
                            }
                            target__ = map_.next_value::<::std::option::Option<_>>()?.map(define_generator_request::Target::TrackId);
                        }
                        GeneratedField::ClipId => {
                            if target__.is_some() {
                                return Err(serde::de::Error::duplicate_field("clipId"));
                            }
                            target__ = map_.next_value::<::std::option::Option<_>>()?.map(define_generator_request::Target::ClipId);
                        }
                    }
                }
                Ok(DefineGeneratorRequest {
                    kind: kind__.unwrap_or_default(),
                    source: source__.unwrap_or_default(),
                    seed: seed__.unwrap_or_default(),
                    params: params__.unwrap_or_default(),
                    dry_run: dry_run__.unwrap_or_default(),
                    target: target__,
                })
            }
        }
        deserializer.deserialize_struct("escribass.tools.v1.DefineGeneratorRequest", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for DeleteBranchRequest {
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
        let mut struct_ser = serializer.serialize_struct("escribass.tools.v1.DeleteBranchRequest", len)?;
        if true {
            struct_ser.serialize_field("name", &self.name)?;
        }
        if true {
            struct_ser.serialize_field("dry_run", &self.dry_run)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for DeleteBranchRequest {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "name",
            "dry_run",
            "dryRun",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Name,
            DryRun,
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
                            "name" => Ok(GeneratedField::Name),
                            "dryRun" | "dry_run" => Ok(GeneratedField::DryRun),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = DeleteBranchRequest;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct escribass.tools.v1.DeleteBranchRequest")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<DeleteBranchRequest, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut name__ = None;
                let mut dry_run__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Name => {
                            if name__.is_some() {
                                return Err(serde::de::Error::duplicate_field("name"));
                            }
                            name__ = Some(map_.next_value()?);
                        }
                        GeneratedField::DryRun => {
                            if dry_run__.is_some() {
                                return Err(serde::de::Error::duplicate_field("dryRun"));
                            }
                            dry_run__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(DeleteBranchRequest {
                    name: name__.unwrap_or_default(),
                    dry_run: dry_run__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("escribass.tools.v1.DeleteBranchRequest", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for GetHistoryRequest {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let len = 0;
        let struct_ser = serializer.serialize_struct("escribass.tools.v1.GetHistoryRequest", len)?;
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for GetHistoryRequest {
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
            type Value = GetHistoryRequest;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct escribass.tools.v1.GetHistoryRequest")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<GetHistoryRequest, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                while map_.next_key::<GeneratedField>()?.is_some() {
                    let _ = map_.next_value::<serde::de::IgnoredAny>()?;
                }
                Ok(GetHistoryRequest {
                })
            }
        }
        deserializer.deserialize_struct("escribass.tools.v1.GetHistoryRequest", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for GetSongAtRequest {
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
        let mut struct_ser = serializer.serialize_struct("escribass.tools.v1.GetSongAtRequest", len)?;
        if true {
            struct_ser.serialize_field("entry_id", &self.entry_id)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for GetSongAtRequest {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "entry_id",
            "entryId",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            EntryId,
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
                            "entryId" | "entry_id" => Ok(GeneratedField::EntryId),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = GetSongAtRequest;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct escribass.tools.v1.GetSongAtRequest")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<GetSongAtRequest, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut entry_id__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::EntryId => {
                            if entry_id__.is_some() {
                                return Err(serde::de::Error::duplicate_field("entryId"));
                            }
                            entry_id__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(GetSongAtRequest {
                    entry_id: entry_id__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("escribass.tools.v1.GetSongAtRequest", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for GetSongRequest {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let len = 0;
        let struct_ser = serializer.serialize_struct("escribass.tools.v1.GetSongRequest", len)?;
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for GetSongRequest {
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
            type Value = GetSongRequest;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct escribass.tools.v1.GetSongRequest")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<GetSongRequest, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                while map_.next_key::<GeneratedField>()?.is_some() {
                    let _ = map_.next_value::<serde::de::IgnoredAny>()?;
                }
                Ok(GetSongRequest {
                })
            }
        }
        deserializer.deserialize_struct("escribass.tools.v1.GetSongRequest", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for HistoryResponse {
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
        let mut struct_ser = serializer.serialize_struct("escribass.tools.v1.HistoryResponse", len)?;
        if true {
            struct_ser.serialize_field("entries", &self.entries)?;
        }
        if let Some(v) = self.refs.as_ref() {
            struct_ser.serialize_field("refs", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for HistoryResponse {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "entries",
            "refs",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Entries,
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
                            "entries" => Ok(GeneratedField::Entries),
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
            type Value = HistoryResponse;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct escribass.tools.v1.HistoryResponse")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<HistoryResponse, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut entries__ = None;
                let mut refs__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Entries => {
                            if entries__.is_some() {
                                return Err(serde::de::Error::duplicate_field("entries"));
                            }
                            entries__ = Some(
                                map_.next_value::<std::collections::BTreeMap<_, _>>()?
                            );
                        }
                        GeneratedField::Refs => {
                            if refs__.is_some() {
                                return Err(serde::de::Error::duplicate_field("refs"));
                            }
                            refs__ = map_.next_value()?;
                        }
                    }
                }
                Ok(HistoryResponse {
                    entries: entries__.unwrap_or_default(),
                    refs: refs__,
                })
            }
        }
        deserializer.deserialize_struct("escribass.tools.v1.HistoryResponse", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for MergeBranchRequest {
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
        let mut struct_ser = serializer.serialize_struct("escribass.tools.v1.MergeBranchRequest", len)?;
        if true {
            struct_ser.serialize_field("name", &self.name)?;
        }
        if true {
            struct_ser.serialize_field("dry_run", &self.dry_run)?;
        }
        if true {
            let v: std::collections::HashMap<_, _> = self.resolve.iter()
                .map(|(k, v)| {
                    let v = MergeSide::try_from(*v)
                        .map_err(|_| serde::ser::Error::custom(format!("Invalid variant {}", *v)))?;
                    Ok((k, v))
                }).collect::<std::result::Result<_,_>>()?;
            struct_ser.serialize_field("resolve", &v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for MergeBranchRequest {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "name",
            "dry_run",
            "dryRun",
            "resolve",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Name,
            DryRun,
            Resolve,
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
                            "name" => Ok(GeneratedField::Name),
                            "dryRun" | "dry_run" => Ok(GeneratedField::DryRun),
                            "resolve" => Ok(GeneratedField::Resolve),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = MergeBranchRequest;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct escribass.tools.v1.MergeBranchRequest")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<MergeBranchRequest, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut name__ = None;
                let mut dry_run__ = None;
                let mut resolve__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Name => {
                            if name__.is_some() {
                                return Err(serde::de::Error::duplicate_field("name"));
                            }
                            name__ = Some(map_.next_value()?);
                        }
                        GeneratedField::DryRun => {
                            if dry_run__.is_some() {
                                return Err(serde::de::Error::duplicate_field("dryRun"));
                            }
                            dry_run__ = Some(map_.next_value()?);
                        }
                        GeneratedField::Resolve => {
                            if resolve__.is_some() {
                                return Err(serde::de::Error::duplicate_field("resolve"));
                            }
                            resolve__ = Some(
                                map_.next_value::<std::collections::BTreeMap<_, MergeSide>>()?
                                    .into_iter().map(|(k,v)| (k, v as i32)).collect()
                            );
                        }
                    }
                }
                Ok(MergeBranchRequest {
                    name: name__.unwrap_or_default(),
                    dry_run: dry_run__.unwrap_or_default(),
                    resolve: resolve__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("escribass.tools.v1.MergeBranchRequest", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for MergeSide {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let variant = match self {
            Self::Unspecified => "MERGE_SIDE_UNSPECIFIED",
            Self::Ours => "MERGE_SIDE_OURS",
            Self::Theirs => "MERGE_SIDE_THEIRS",
        };
        serializer.serialize_str(variant)
    }
}
impl<'de> serde::Deserialize<'de> for MergeSide {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "MERGE_SIDE_UNSPECIFIED",
            "MERGE_SIDE_OURS",
            "MERGE_SIDE_THEIRS",
        ];

        struct GeneratedVisitor;

        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = MergeSide;

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
                    "MERGE_SIDE_UNSPECIFIED" => Ok(MergeSide::Unspecified),
                    "MERGE_SIDE_OURS" => Ok(MergeSide::Ours),
                    "MERGE_SIDE_THEIRS" => Ok(MergeSide::Theirs),
                    _ => Err(serde::de::Error::unknown_variant(value, FIELDS)),
                }
            }
        }
        deserializer.deserialize_any(GeneratedVisitor)
    }
}
impl serde::Serialize for MoveSectionRequest {
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
        let mut struct_ser = serializer.serialize_struct("escribass.tools.v1.MoveSectionRequest", len)?;
        if true {
            struct_ser.serialize_field("section_id", &self.section_id)?;
        }
        if true {
            struct_ser.serialize_field("start_tick", &self.start_tick)?;
        }
        if true {
            struct_ser.serialize_field("end_tick", &self.end_tick)?;
        }
        if true {
            struct_ser.serialize_field("dry_run", &self.dry_run)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for MoveSectionRequest {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "section_id",
            "sectionId",
            "start_tick",
            "startTick",
            "end_tick",
            "endTick",
            "dry_run",
            "dryRun",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            SectionId,
            StartTick,
            EndTick,
            DryRun,
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
                            "sectionId" | "section_id" => Ok(GeneratedField::SectionId),
                            "startTick" | "start_tick" => Ok(GeneratedField::StartTick),
                            "endTick" | "end_tick" => Ok(GeneratedField::EndTick),
                            "dryRun" | "dry_run" => Ok(GeneratedField::DryRun),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = MoveSectionRequest;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct escribass.tools.v1.MoveSectionRequest")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<MoveSectionRequest, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut section_id__ = None;
                let mut start_tick__ = None;
                let mut end_tick__ = None;
                let mut dry_run__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::SectionId => {
                            if section_id__.is_some() {
                                return Err(serde::de::Error::duplicate_field("sectionId"));
                            }
                            section_id__ = Some(map_.next_value()?);
                        }
                        GeneratedField::StartTick => {
                            if start_tick__.is_some() {
                                return Err(serde::de::Error::duplicate_field("startTick"));
                            }
                            start_tick__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::EndTick => {
                            if end_tick__.is_some() {
                                return Err(serde::de::Error::duplicate_field("endTick"));
                            }
                            end_tick__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::DryRun => {
                            if dry_run__.is_some() {
                                return Err(serde::de::Error::duplicate_field("dryRun"));
                            }
                            dry_run__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(MoveSectionRequest {
                    section_id: section_id__.unwrap_or_default(),
                    start_tick: start_tick__.unwrap_or_default(),
                    end_tick: end_tick__.unwrap_or_default(),
                    dry_run: dry_run__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("escribass.tools.v1.MoveSectionRequest", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for PreviewFrom {
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
        let mut struct_ser = serializer.serialize_struct("escribass.tools.v1.PreviewFrom", len)?;
        if let Some(v) = self.start_tick.as_ref() {
            struct_ser.serialize_field("start_tick", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for PreviewFrom {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "start_tick",
            "startTick",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            StartTick,
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
                            "startTick" | "start_tick" => Ok(GeneratedField::StartTick),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = PreviewFrom;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct escribass.tools.v1.PreviewFrom")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<PreviewFrom, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut start_tick__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::StartTick => {
                            if start_tick__.is_some() {
                                return Err(serde::de::Error::duplicate_field("startTick"));
                            }
                            start_tick__ = 
                                map_.next_value::<::std::option::Option<::pbjson::private::NumberDeserialize<_>>>()?.map(|x| x.0)
                            ;
                        }
                    }
                }
                Ok(PreviewFrom {
                    start_tick: start_tick__,
                })
            }
        }
        deserializer.deserialize_struct("escribass.tools.v1.PreviewFrom", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for PreviewResponse {
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
        let mut struct_ser = serializer.serialize_struct("escribass.tools.v1.PreviewResponse", len)?;
        if true {
            struct_ser.serialize_field("valid", &self.valid)?;
        }
        if true {
            struct_ser.serialize_field("errors", &self.errors)?;
        }
        if true {
            struct_ser.serialize_field("summary", &self.summary)?;
        }
        if let Some(v) = self.event.as_ref() {
            struct_ser.serialize_field("event", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for PreviewResponse {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "valid",
            "errors",
            "summary",
            "event",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Valid,
            Errors,
            Summary,
            Event,
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
                            "valid" => Ok(GeneratedField::Valid),
                            "errors" => Ok(GeneratedField::Errors),
                            "summary" => Ok(GeneratedField::Summary),
                            "event" => Ok(GeneratedField::Event),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = PreviewResponse;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct escribass.tools.v1.PreviewResponse")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<PreviewResponse, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut valid__ = None;
                let mut errors__ = None;
                let mut summary__ = None;
                let mut event__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Valid => {
                            if valid__.is_some() {
                                return Err(serde::de::Error::duplicate_field("valid"));
                            }
                            valid__ = Some(map_.next_value()?);
                        }
                        GeneratedField::Errors => {
                            if errors__.is_some() {
                                return Err(serde::de::Error::duplicate_field("errors"));
                            }
                            errors__ = Some(map_.next_value()?);
                        }
                        GeneratedField::Summary => {
                            if summary__.is_some() {
                                return Err(serde::de::Error::duplicate_field("summary"));
                            }
                            summary__ = Some(map_.next_value()?);
                        }
                        GeneratedField::Event => {
                            if event__.is_some() {
                                return Err(serde::de::Error::duplicate_field("event"));
                            }
                            event__ = map_.next_value()?;
                        }
                    }
                }
                Ok(PreviewResponse {
                    valid: valid__.unwrap_or_default(),
                    errors: errors__.unwrap_or_default(),
                    summary: summary__.unwrap_or_default(),
                    event: event__,
                })
            }
        }
        deserializer.deserialize_struct("escribass.tools.v1.PreviewResponse", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for QuantizeRequest {
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
        let mut struct_ser = serializer.serialize_struct("escribass.tools.v1.QuantizeRequest", len)?;
        if true {
            struct_ser.serialize_field("clip_id", &self.clip_id)?;
        }
        if true {
            struct_ser.serialize_field("grid_ticks", &self.grid_ticks)?;
        }
        if true {
            struct_ser.serialize_field("note_ids", &self.note_ids)?;
        }
        if true {
            struct_ser.serialize_field("dry_run", &self.dry_run)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for QuantizeRequest {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "clip_id",
            "clipId",
            "grid_ticks",
            "gridTicks",
            "note_ids",
            "noteIds",
            "dry_run",
            "dryRun",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            ClipId,
            GridTicks,
            NoteIds,
            DryRun,
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
                            "clipId" | "clip_id" => Ok(GeneratedField::ClipId),
                            "gridTicks" | "grid_ticks" => Ok(GeneratedField::GridTicks),
                            "noteIds" | "note_ids" => Ok(GeneratedField::NoteIds),
                            "dryRun" | "dry_run" => Ok(GeneratedField::DryRun),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = QuantizeRequest;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct escribass.tools.v1.QuantizeRequest")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<QuantizeRequest, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut clip_id__ = None;
                let mut grid_ticks__ = None;
                let mut note_ids__ = None;
                let mut dry_run__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::ClipId => {
                            if clip_id__.is_some() {
                                return Err(serde::de::Error::duplicate_field("clipId"));
                            }
                            clip_id__ = Some(map_.next_value()?);
                        }
                        GeneratedField::GridTicks => {
                            if grid_ticks__.is_some() {
                                return Err(serde::de::Error::duplicate_field("gridTicks"));
                            }
                            grid_ticks__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::NoteIds => {
                            if note_ids__.is_some() {
                                return Err(serde::de::Error::duplicate_field("noteIds"));
                            }
                            note_ids__ = Some(map_.next_value()?);
                        }
                        GeneratedField::DryRun => {
                            if dry_run__.is_some() {
                                return Err(serde::de::Error::duplicate_field("dryRun"));
                            }
                            dry_run__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(QuantizeRequest {
                    clip_id: clip_id__.unwrap_or_default(),
                    grid_ticks: grid_ticks__.unwrap_or_default(),
                    note_ids: note_ids__.unwrap_or_default(),
                    dry_run: dry_run__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("escribass.tools.v1.QuantizeRequest", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for RedoRequest {
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
        let mut struct_ser = serializer.serialize_struct("escribass.tools.v1.RedoRequest", len)?;
        if true {
            struct_ser.serialize_field("dry_run", &self.dry_run)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for RedoRequest {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "dry_run",
            "dryRun",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            DryRun,
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
                            "dryRun" | "dry_run" => Ok(GeneratedField::DryRun),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = RedoRequest;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct escribass.tools.v1.RedoRequest")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<RedoRequest, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut dry_run__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::DryRun => {
                            if dry_run__.is_some() {
                                return Err(serde::de::Error::duplicate_field("dryRun"));
                            }
                            dry_run__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(RedoRequest {
                    dry_run: dry_run__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("escribass.tools.v1.RedoRequest", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for RenderExportRequest {
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
        let mut struct_ser = serializer.serialize_struct("escribass.tools.v1.RenderExportRequest", len)?;
        if true {
            struct_ser.serialize_field("output_path", &self.output_path)?;
        }
        if true {
            struct_ser.serialize_field("dry_run", &self.dry_run)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for RenderExportRequest {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "output_path",
            "outputPath",
            "dry_run",
            "dryRun",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            OutputPath,
            DryRun,
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
                            "outputPath" | "output_path" => Ok(GeneratedField::OutputPath),
                            "dryRun" | "dry_run" => Ok(GeneratedField::DryRun),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = RenderExportRequest;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct escribass.tools.v1.RenderExportRequest")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<RenderExportRequest, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut output_path__ = None;
                let mut dry_run__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::OutputPath => {
                            if output_path__.is_some() {
                                return Err(serde::de::Error::duplicate_field("outputPath"));
                            }
                            output_path__ = Some(map_.next_value()?);
                        }
                        GeneratedField::DryRun => {
                            if dry_run__.is_some() {
                                return Err(serde::de::Error::duplicate_field("dryRun"));
                            }
                            dry_run__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(RenderExportRequest {
                    output_path: output_path__.unwrap_or_default(),
                    dry_run: dry_run__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("escribass.tools.v1.RenderExportRequest", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for RenderPreviewRequest {
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
        if self.command.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("escribass.tools.v1.RenderPreviewRequest", len)?;
        if true {
            struct_ser.serialize_field("dry_run", &self.dry_run)?;
        }
        if let Some(v) = self.command.as_ref() {
            match v {
                render_preview_request::Command::Play(v) => {
                    struct_ser.serialize_field("play", v)?;
                }
                render_preview_request::Command::Seek(v) => {
                    struct_ser.serialize_field("seek", v)?;
                }
                render_preview_request::Command::Loop(v) => {
                    struct_ser.serialize_field("loop", v)?;
                }
                render_preview_request::Command::Stop(v) => {
                    struct_ser.serialize_field("stop", v)?;
                }
            }
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for RenderPreviewRequest {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "dry_run",
            "dryRun",
            "play",
            "seek",
            "loop",
            "stop",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            DryRun,
            Play,
            Seek,
            Loop,
            Stop,
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
                            "dryRun" | "dry_run" => Ok(GeneratedField::DryRun),
                            "play" => Ok(GeneratedField::Play),
                            "seek" => Ok(GeneratedField::Seek),
                            "loop" => Ok(GeneratedField::Loop),
                            "stop" => Ok(GeneratedField::Stop),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = RenderPreviewRequest;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct escribass.tools.v1.RenderPreviewRequest")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<RenderPreviewRequest, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut dry_run__ = None;
                let mut command__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::DryRun => {
                            if dry_run__.is_some() {
                                return Err(serde::de::Error::duplicate_field("dryRun"));
                            }
                            dry_run__ = Some(map_.next_value()?);
                        }
                        GeneratedField::Play => {
                            if command__.is_some() {
                                return Err(serde::de::Error::duplicate_field("play"));
                            }
                            command__ = map_.next_value::<::std::option::Option<_>>()?.map(render_preview_request::Command::Play)
;
                        }
                        GeneratedField::Seek => {
                            if command__.is_some() {
                                return Err(serde::de::Error::duplicate_field("seek"));
                            }
                            command__ = map_.next_value::<::std::option::Option<_>>()?.map(render_preview_request::Command::Seek)
;
                        }
                        GeneratedField::Loop => {
                            if command__.is_some() {
                                return Err(serde::de::Error::duplicate_field("loop"));
                            }
                            command__ = map_.next_value::<::std::option::Option<_>>()?.map(render_preview_request::Command::Loop)
;
                        }
                        GeneratedField::Stop => {
                            if command__.is_some() {
                                return Err(serde::de::Error::duplicate_field("stop"));
                            }
                            command__ = map_.next_value::<::std::option::Option<_>>()?.map(render_preview_request::Command::Stop)
;
                        }
                    }
                }
                Ok(RenderPreviewRequest {
                    dry_run: dry_run__.unwrap_or_default(),
                    command: command__,
                })
            }
        }
        deserializer.deserialize_struct("escribass.tools.v1.RenderPreviewRequest", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for RenderResponse {
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
        let mut struct_ser = serializer.serialize_struct("escribass.tools.v1.RenderResponse", len)?;
        if true {
            struct_ser.serialize_field("valid", &self.valid)?;
        }
        if true {
            struct_ser.serialize_field("errors", &self.errors)?;
        }
        if true {
            struct_ser.serialize_field("summary", &self.summary)?;
        }
        if let Some(v) = self.result.as_ref() {
            struct_ser.serialize_field("result", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for RenderResponse {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "valid",
            "errors",
            "summary",
            "result",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Valid,
            Errors,
            Summary,
            Result,
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
                            "valid" => Ok(GeneratedField::Valid),
                            "errors" => Ok(GeneratedField::Errors),
                            "summary" => Ok(GeneratedField::Summary),
                            "result" => Ok(GeneratedField::Result),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = RenderResponse;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct escribass.tools.v1.RenderResponse")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<RenderResponse, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut valid__ = None;
                let mut errors__ = None;
                let mut summary__ = None;
                let mut result__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Valid => {
                            if valid__.is_some() {
                                return Err(serde::de::Error::duplicate_field("valid"));
                            }
                            valid__ = Some(map_.next_value()?);
                        }
                        GeneratedField::Errors => {
                            if errors__.is_some() {
                                return Err(serde::de::Error::duplicate_field("errors"));
                            }
                            errors__ = Some(map_.next_value()?);
                        }
                        GeneratedField::Summary => {
                            if summary__.is_some() {
                                return Err(serde::de::Error::duplicate_field("summary"));
                            }
                            summary__ = Some(map_.next_value()?);
                        }
                        GeneratedField::Result => {
                            if result__.is_some() {
                                return Err(serde::de::Error::duplicate_field("result"));
                            }
                            result__ = map_.next_value()?;
                        }
                    }
                }
                Ok(RenderResponse {
                    valid: valid__.unwrap_or_default(),
                    errors: errors__.unwrap_or_default(),
                    summary: summary__.unwrap_or_default(),
                    result: result__,
                })
            }
        }
        deserializer.deserialize_struct("escribass.tools.v1.RenderResponse", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for SetNotesRequest {
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
        let mut struct_ser = serializer.serialize_struct("escribass.tools.v1.SetNotesRequest", len)?;
        if true {
            struct_ser.serialize_field("clip_id", &self.clip_id)?;
        }
        if true {
            struct_ser.serialize_field("notes", &self.notes)?;
        }
        if true {
            struct_ser.serialize_field("dry_run", &self.dry_run)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for SetNotesRequest {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "clip_id",
            "clipId",
            "notes",
            "dry_run",
            "dryRun",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            ClipId,
            Notes,
            DryRun,
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
                            "clipId" | "clip_id" => Ok(GeneratedField::ClipId),
                            "notes" => Ok(GeneratedField::Notes),
                            "dryRun" | "dry_run" => Ok(GeneratedField::DryRun),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = SetNotesRequest;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct escribass.tools.v1.SetNotesRequest")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<SetNotesRequest, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut clip_id__ = None;
                let mut notes__ = None;
                let mut dry_run__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::ClipId => {
                            if clip_id__.is_some() {
                                return Err(serde::de::Error::duplicate_field("clipId"));
                            }
                            clip_id__ = Some(map_.next_value()?);
                        }
                        GeneratedField::Notes => {
                            if notes__.is_some() {
                                return Err(serde::de::Error::duplicate_field("notes"));
                            }
                            notes__ = Some(
                                map_.next_value::<std::collections::BTreeMap<_, _>>()?
                            );
                        }
                        GeneratedField::DryRun => {
                            if dry_run__.is_some() {
                                return Err(serde::de::Error::duplicate_field("dryRun"));
                            }
                            dry_run__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(SetNotesRequest {
                    clip_id: clip_id__.unwrap_or_default(),
                    notes: notes__.unwrap_or_default(),
                    dry_run: dry_run__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("escribass.tools.v1.SetNotesRequest", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for SetParamRequest {
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
        let mut struct_ser = serializer.serialize_struct("escribass.tools.v1.SetParamRequest", len)?;
        if true {
            struct_ser.serialize_field("device_id", &self.device_id)?;
        }
        if true {
            struct_ser.serialize_field("param", &self.param)?;
        }
        if true {
            struct_ser.serialize_field("value", &self.value)?;
        }
        if true {
            struct_ser.serialize_field("dry_run", &self.dry_run)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for SetParamRequest {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "device_id",
            "deviceId",
            "param",
            "value",
            "dry_run",
            "dryRun",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            DeviceId,
            Param,
            Value,
            DryRun,
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
                            "deviceId" | "device_id" => Ok(GeneratedField::DeviceId),
                            "param" => Ok(GeneratedField::Param),
                            "value" => Ok(GeneratedField::Value),
                            "dryRun" | "dry_run" => Ok(GeneratedField::DryRun),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = SetParamRequest;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct escribass.tools.v1.SetParamRequest")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<SetParamRequest, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut device_id__ = None;
                let mut param__ = None;
                let mut value__ = None;
                let mut dry_run__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::DeviceId => {
                            if device_id__.is_some() {
                                return Err(serde::de::Error::duplicate_field("deviceId"));
                            }
                            device_id__ = Some(map_.next_value()?);
                        }
                        GeneratedField::Param => {
                            if param__.is_some() {
                                return Err(serde::de::Error::duplicate_field("param"));
                            }
                            param__ = Some(map_.next_value()?);
                        }
                        GeneratedField::Value => {
                            if value__.is_some() {
                                return Err(serde::de::Error::duplicate_field("value"));
                            }
                            value__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::DryRun => {
                            if dry_run__.is_some() {
                                return Err(serde::de::Error::duplicate_field("dryRun"));
                            }
                            dry_run__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(SetParamRequest {
                    device_id: device_id__.unwrap_or_default(),
                    param: param__.unwrap_or_default(),
                    value: value__.unwrap_or_default(),
                    dry_run: dry_run__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("escribass.tools.v1.SetParamRequest", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for SetTempoRequest {
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
        let mut struct_ser = serializer.serialize_struct("escribass.tools.v1.SetTempoRequest", len)?;
        if true {
            struct_ser.serialize_field("bpm", &self.bpm)?;
        }
        if true {
            struct_ser.serialize_field("tick", &self.tick)?;
        }
        if true {
            struct_ser.serialize_field("dry_run", &self.dry_run)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for SetTempoRequest {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "bpm",
            "tick",
            "dry_run",
            "dryRun",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Bpm,
            Tick,
            DryRun,
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
                            "bpm" => Ok(GeneratedField::Bpm),
                            "tick" => Ok(GeneratedField::Tick),
                            "dryRun" | "dry_run" => Ok(GeneratedField::DryRun),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = SetTempoRequest;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct escribass.tools.v1.SetTempoRequest")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<SetTempoRequest, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut bpm__ = None;
                let mut tick__ = None;
                let mut dry_run__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Bpm => {
                            if bpm__.is_some() {
                                return Err(serde::de::Error::duplicate_field("bpm"));
                            }
                            bpm__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::Tick => {
                            if tick__.is_some() {
                                return Err(serde::de::Error::duplicate_field("tick"));
                            }
                            tick__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::DryRun => {
                            if dry_run__.is_some() {
                                return Err(serde::de::Error::duplicate_field("dryRun"));
                            }
                            dry_run__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(SetTempoRequest {
                    bpm: bpm__.unwrap_or_default(),
                    tick: tick__.unwrap_or_default(),
                    dry_run: dry_run__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("escribass.tools.v1.SetTempoRequest", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for SetTrackInstrumentRequest {
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
        let mut struct_ser = serializer.serialize_struct("escribass.tools.v1.SetTrackInstrumentRequest", len)?;
        if true {
            struct_ser.serialize_field("track_id", &self.track_id)?;
        }
        if let Some(v) = self.r#ref.as_ref() {
            struct_ser.serialize_field("ref", v)?;
        }
        if true {
            struct_ser.serialize_field("dry_run", &self.dry_run)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for SetTrackInstrumentRequest {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "track_id",
            "trackId",
            "ref",
            "dry_run",
            "dryRun",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            TrackId,
            Ref,
            DryRun,
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
                            "trackId" | "track_id" => Ok(GeneratedField::TrackId),
                            "ref" => Ok(GeneratedField::Ref),
                            "dryRun" | "dry_run" => Ok(GeneratedField::DryRun),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = SetTrackInstrumentRequest;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct escribass.tools.v1.SetTrackInstrumentRequest")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<SetTrackInstrumentRequest, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut track_id__ = None;
                let mut r#ref__ = None;
                let mut dry_run__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::TrackId => {
                            if track_id__.is_some() {
                                return Err(serde::de::Error::duplicate_field("trackId"));
                            }
                            track_id__ = Some(map_.next_value()?);
                        }
                        GeneratedField::Ref => {
                            if r#ref__.is_some() {
                                return Err(serde::de::Error::duplicate_field("ref"));
                            }
                            r#ref__ = map_.next_value()?;
                        }
                        GeneratedField::DryRun => {
                            if dry_run__.is_some() {
                                return Err(serde::de::Error::duplicate_field("dryRun"));
                            }
                            dry_run__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(SetTrackInstrumentRequest {
                    track_id: track_id__.unwrap_or_default(),
                    r#ref: r#ref__,
                    dry_run: dry_run__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("escribass.tools.v1.SetTrackInstrumentRequest", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for SongResponse {
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
        let mut struct_ser = serializer.serialize_struct("escribass.tools.v1.SongResponse", len)?;
        if let Some(v) = self.song.as_ref() {
            struct_ser.serialize_field("song", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for SongResponse {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "song",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Song,
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
                            "song" => Ok(GeneratedField::Song),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = SongResponse;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct escribass.tools.v1.SongResponse")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<SongResponse, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut song__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Song => {
                            if song__.is_some() {
                                return Err(serde::de::Error::duplicate_field("song"));
                            }
                            song__ = map_.next_value()?;
                        }
                    }
                }
                Ok(SongResponse {
                    song: song__,
                })
            }
        }
        deserializer.deserialize_struct("escribass.tools.v1.SongResponse", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for SwitchBranchRequest {
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
        let mut struct_ser = serializer.serialize_struct("escribass.tools.v1.SwitchBranchRequest", len)?;
        if true {
            struct_ser.serialize_field("name", &self.name)?;
        }
        if true {
            struct_ser.serialize_field("dry_run", &self.dry_run)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for SwitchBranchRequest {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "name",
            "dry_run",
            "dryRun",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Name,
            DryRun,
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
                            "name" => Ok(GeneratedField::Name),
                            "dryRun" | "dry_run" => Ok(GeneratedField::DryRun),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = SwitchBranchRequest;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct escribass.tools.v1.SwitchBranchRequest")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<SwitchBranchRequest, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut name__ = None;
                let mut dry_run__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Name => {
                            if name__.is_some() {
                                return Err(serde::de::Error::duplicate_field("name"));
                            }
                            name__ = Some(map_.next_value()?);
                        }
                        GeneratedField::DryRun => {
                            if dry_run__.is_some() {
                                return Err(serde::de::Error::duplicate_field("dryRun"));
                            }
                            dry_run__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(SwitchBranchRequest {
                    name: name__.unwrap_or_default(),
                    dry_run: dry_run__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("escribass.tools.v1.SwitchBranchRequest", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for ToolResult {
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
        let mut struct_ser = serializer.serialize_struct("escribass.tools.v1.ToolResult", len)?;
        if true {
            struct_ser.serialize_field("valid", &self.valid)?;
        }
        if true {
            struct_ser.serialize_field("errors", &self.errors)?;
        }
        if true {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("patch", pbjson::private::base64::encode(&self.patch).as_str())?;
        }
        if true {
            struct_ser.serialize_field("summary", &self.summary)?;
        }
        if true {
            struct_ser.serialize_field("entry_id", &self.entry_id)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for ToolResult {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "valid",
            "errors",
            "patch",
            "summary",
            "entry_id",
            "entryId",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Valid,
            Errors,
            Patch,
            Summary,
            EntryId,
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
                            "valid" => Ok(GeneratedField::Valid),
                            "errors" => Ok(GeneratedField::Errors),
                            "patch" => Ok(GeneratedField::Patch),
                            "summary" => Ok(GeneratedField::Summary),
                            "entryId" | "entry_id" => Ok(GeneratedField::EntryId),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = ToolResult;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct escribass.tools.v1.ToolResult")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<ToolResult, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut valid__ = None;
                let mut errors__ = None;
                let mut patch__ = None;
                let mut summary__ = None;
                let mut entry_id__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Valid => {
                            if valid__.is_some() {
                                return Err(serde::de::Error::duplicate_field("valid"));
                            }
                            valid__ = Some(map_.next_value()?);
                        }
                        GeneratedField::Errors => {
                            if errors__.is_some() {
                                return Err(serde::de::Error::duplicate_field("errors"));
                            }
                            errors__ = Some(map_.next_value()?);
                        }
                        GeneratedField::Patch => {
                            if patch__.is_some() {
                                return Err(serde::de::Error::duplicate_field("patch"));
                            }
                            patch__ = 
                                Some(map_.next_value::<::pbjson::private::BytesDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::Summary => {
                            if summary__.is_some() {
                                return Err(serde::de::Error::duplicate_field("summary"));
                            }
                            summary__ = Some(map_.next_value()?);
                        }
                        GeneratedField::EntryId => {
                            if entry_id__.is_some() {
                                return Err(serde::de::Error::duplicate_field("entryId"));
                            }
                            entry_id__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(ToolResult {
                    valid: valid__.unwrap_or_default(),
                    errors: errors__.unwrap_or_default(),
                    patch: patch__.unwrap_or_default(),
                    summary: summary__.unwrap_or_default(),
                    entry_id: entry_id__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("escribass.tools.v1.ToolResult", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for TransposeRequest {
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
        let mut struct_ser = serializer.serialize_struct("escribass.tools.v1.TransposeRequest", len)?;
        if true {
            struct_ser.serialize_field("clip_id", &self.clip_id)?;
        }
        if true {
            struct_ser.serialize_field("semitones", &self.semitones)?;
        }
        if true {
            struct_ser.serialize_field("note_ids", &self.note_ids)?;
        }
        if true {
            struct_ser.serialize_field("dry_run", &self.dry_run)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for TransposeRequest {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "clip_id",
            "clipId",
            "semitones",
            "note_ids",
            "noteIds",
            "dry_run",
            "dryRun",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            ClipId,
            Semitones,
            NoteIds,
            DryRun,
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
                            "clipId" | "clip_id" => Ok(GeneratedField::ClipId),
                            "semitones" => Ok(GeneratedField::Semitones),
                            "noteIds" | "note_ids" => Ok(GeneratedField::NoteIds),
                            "dryRun" | "dry_run" => Ok(GeneratedField::DryRun),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = TransposeRequest;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct escribass.tools.v1.TransposeRequest")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<TransposeRequest, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut clip_id__ = None;
                let mut semitones__ = None;
                let mut note_ids__ = None;
                let mut dry_run__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::ClipId => {
                            if clip_id__.is_some() {
                                return Err(serde::de::Error::duplicate_field("clipId"));
                            }
                            clip_id__ = Some(map_.next_value()?);
                        }
                        GeneratedField::Semitones => {
                            if semitones__.is_some() {
                                return Err(serde::de::Error::duplicate_field("semitones"));
                            }
                            semitones__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::NoteIds => {
                            if note_ids__.is_some() {
                                return Err(serde::de::Error::duplicate_field("noteIds"));
                            }
                            note_ids__ = Some(map_.next_value()?);
                        }
                        GeneratedField::DryRun => {
                            if dry_run__.is_some() {
                                return Err(serde::de::Error::duplicate_field("dryRun"));
                            }
                            dry_run__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(TransposeRequest {
                    clip_id: clip_id__.unwrap_or_default(),
                    semitones: semitones__.unwrap_or_default(),
                    note_ids: note_ids__.unwrap_or_default(),
                    dry_run: dry_run__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("escribass.tools.v1.TransposeRequest", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for UndoRequest {
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
        let mut struct_ser = serializer.serialize_struct("escribass.tools.v1.UndoRequest", len)?;
        if true {
            struct_ser.serialize_field("dry_run", &self.dry_run)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for UndoRequest {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "dry_run",
            "dryRun",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            DryRun,
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
                            "dryRun" | "dry_run" => Ok(GeneratedField::DryRun),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = UndoRequest;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct escribass.tools.v1.UndoRequest")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<UndoRequest, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut dry_run__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::DryRun => {
                            if dry_run__.is_some() {
                                return Err(serde::de::Error::duplicate_field("dryRun"));
                            }
                            dry_run__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(UndoRequest {
                    dry_run: dry_run__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("escribass.tools.v1.UndoRequest", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for Violation {
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
        let mut struct_ser = serializer.serialize_struct("escribass.tools.v1.Violation", len)?;
        if true {
            struct_ser.serialize_field("path", &self.path)?;
        }
        if true {
            struct_ser.serialize_field("rule", &self.rule)?;
        }
        if true {
            struct_ser.serialize_field("message", &self.message)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for Violation {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "path",
            "rule",
            "message",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Path,
            Rule,
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
                            "path" => Ok(GeneratedField::Path),
                            "rule" => Ok(GeneratedField::Rule),
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
            type Value = Violation;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct escribass.tools.v1.Violation")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<Violation, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut path__ = None;
                let mut rule__ = None;
                let mut message__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Path => {
                            if path__.is_some() {
                                return Err(serde::de::Error::duplicate_field("path"));
                            }
                            path__ = Some(map_.next_value()?);
                        }
                        GeneratedField::Rule => {
                            if rule__.is_some() {
                                return Err(serde::de::Error::duplicate_field("rule"));
                            }
                            rule__ = Some(map_.next_value()?);
                        }
                        GeneratedField::Message => {
                            if message__.is_some() {
                                return Err(serde::de::Error::duplicate_field("message"));
                            }
                            message__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(Violation {
                    path: path__.unwrap_or_default(),
                    rule: rule__.unwrap_or_default(),
                    message: message__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("escribass.tools.v1.Violation", FIELDS, GeneratedVisitor)
    }
}

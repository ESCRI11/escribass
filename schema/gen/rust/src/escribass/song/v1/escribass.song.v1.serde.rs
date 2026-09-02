// @generated
impl serde::Serialize for AudioClip {
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
        let mut struct_ser = serializer.serialize_struct("escribass.song.v1.AudioClip", len)?;
        if true {
            struct_ser.serialize_field("asset_hash", &self.asset_hash)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for AudioClip {
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
            type Value = AudioClip;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct escribass.song.v1.AudioClip")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<AudioClip, V::Error>
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
                Ok(AudioClip {
                    asset_hash: asset_hash__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("escribass.song.v1.AudioClip", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for Author {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let variant = match self {
            Self::Unspecified => "AUTHOR_UNSPECIFIED",
            Self::Human => "AUTHOR_HUMAN",
            Self::Model => "AUTHOR_MODEL",
        };
        serializer.serialize_str(variant)
    }
}
impl<'de> serde::Deserialize<'de> for Author {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "AUTHOR_UNSPECIFIED",
            "AUTHOR_HUMAN",
            "AUTHOR_MODEL",
        ];

        struct GeneratedVisitor;

        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = Author;

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
                    "AUTHOR_UNSPECIFIED" => Ok(Author::Unspecified),
                    "AUTHOR_HUMAN" => Ok(Author::Human),
                    "AUTHOR_MODEL" => Ok(Author::Model),
                    _ => Err(serde::de::Error::unknown_variant(value, FIELDS)),
                }
            }
        }
        deserializer.deserialize_any(GeneratedVisitor)
    }
}
impl serde::Serialize for Automation {
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
        let mut struct_ser = serializer.serialize_struct("escribass.song.v1.Automation", len)?;
        if true {
            struct_ser.serialize_field("id", &self.id)?;
        }
        if let Some(v) = self.provenance.as_ref() {
            struct_ser.serialize_field("provenance", v)?;
        }
        if true {
            struct_ser.serialize_field("version", &self.version)?;
        }
        if let Some(v) = self.target.as_ref() {
            struct_ser.serialize_field("target", v)?;
        }
        if true {
            struct_ser.serialize_field("points", &self.points)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for Automation {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "id",
            "provenance",
            "version",
            "target",
            "points",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Id,
            Provenance,
            Version,
            Target,
            Points,
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
                            "provenance" => Ok(GeneratedField::Provenance),
                            "version" => Ok(GeneratedField::Version),
                            "target" => Ok(GeneratedField::Target),
                            "points" => Ok(GeneratedField::Points),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = Automation;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct escribass.song.v1.Automation")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<Automation, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut id__ = None;
                let mut provenance__ = None;
                let mut version__ = None;
                let mut target__ = None;
                let mut points__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Id => {
                            if id__.is_some() {
                                return Err(serde::de::Error::duplicate_field("id"));
                            }
                            id__ = Some(map_.next_value()?);
                        }
                        GeneratedField::Provenance => {
                            if provenance__.is_some() {
                                return Err(serde::de::Error::duplicate_field("provenance"));
                            }
                            provenance__ = map_.next_value()?;
                        }
                        GeneratedField::Version => {
                            if version__.is_some() {
                                return Err(serde::de::Error::duplicate_field("version"));
                            }
                            version__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
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
                    }
                }
                Ok(Automation {
                    id: id__.unwrap_or_default(),
                    provenance: provenance__,
                    version: version__.unwrap_or_default(),
                    target: target__,
                    points: points__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("escribass.song.v1.Automation", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for AutomationPoint {
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
        let mut struct_ser = serializer.serialize_struct("escribass.song.v1.AutomationPoint", len)?;
        if true {
            struct_ser.serialize_field("id", &self.id)?;
        }
        if true {
            struct_ser.serialize_field("tick", &self.tick)?;
        }
        if true {
            struct_ser.serialize_field("value", &self.value)?;
        }
        if true {
            let v = Curve::try_from(self.curve)
                .map_err(|_| serde::ser::Error::custom(format!("Invalid variant {}", self.curve)))?;
            struct_ser.serialize_field("curve", &v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for AutomationPoint {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "id",
            "tick",
            "value",
            "curve",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Id,
            Tick,
            Value,
            Curve,
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
                            "tick" => Ok(GeneratedField::Tick),
                            "value" => Ok(GeneratedField::Value),
                            "curve" => Ok(GeneratedField::Curve),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = AutomationPoint;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct escribass.song.v1.AutomationPoint")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<AutomationPoint, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut id__ = None;
                let mut tick__ = None;
                let mut value__ = None;
                let mut curve__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Id => {
                            if id__.is_some() {
                                return Err(serde::de::Error::duplicate_field("id"));
                            }
                            id__ = Some(map_.next_value()?);
                        }
                        GeneratedField::Tick => {
                            if tick__.is_some() {
                                return Err(serde::de::Error::duplicate_field("tick"));
                            }
                            tick__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::Value => {
                            if value__.is_some() {
                                return Err(serde::de::Error::duplicate_field("value"));
                            }
                            value__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::Curve => {
                            if curve__.is_some() {
                                return Err(serde::de::Error::duplicate_field("curve"));
                            }
                            curve__ = Some(map_.next_value::<Curve>()? as i32);
                        }
                    }
                }
                Ok(AutomationPoint {
                    id: id__.unwrap_or_default(),
                    tick: tick__.unwrap_or_default(),
                    value: value__.unwrap_or_default(),
                    curve: curve__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("escribass.song.v1.AutomationPoint", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for Clip {
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
        if self.content.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("escribass.song.v1.Clip", len)?;
        if true {
            struct_ser.serialize_field("id", &self.id)?;
        }
        if let Some(v) = self.provenance.as_ref() {
            struct_ser.serialize_field("provenance", v)?;
        }
        if true {
            struct_ser.serialize_field("version", &self.version)?;
        }
        if true {
            struct_ser.serialize_field("track_id", &self.track_id)?;
        }
        if true {
            struct_ser.serialize_field("start_tick", &self.start_tick)?;
        }
        if true {
            struct_ser.serialize_field("length_ticks", &self.length_ticks)?;
        }
        if let Some(v) = self.loop_length_ticks.as_ref() {
            struct_ser.serialize_field("loop_length_ticks", v)?;
        }
        if let Some(v) = self.content.as_ref() {
            match v {
                clip::Content::NoteClip(v) => {
                    struct_ser.serialize_field("note_clip", v)?;
                }
                clip::Content::AudioClip(v) => {
                    struct_ser.serialize_field("audio_clip", v)?;
                }
            }
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for Clip {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "id",
            "provenance",
            "version",
            "track_id",
            "trackId",
            "start_tick",
            "startTick",
            "length_ticks",
            "lengthTicks",
            "loop_length_ticks",
            "loopLengthTicks",
            "note_clip",
            "noteClip",
            "audio_clip",
            "audioClip",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Id,
            Provenance,
            Version,
            TrackId,
            StartTick,
            LengthTicks,
            LoopLengthTicks,
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
                            "id" => Ok(GeneratedField::Id),
                            "provenance" => Ok(GeneratedField::Provenance),
                            "version" => Ok(GeneratedField::Version),
                            "trackId" | "track_id" => Ok(GeneratedField::TrackId),
                            "startTick" | "start_tick" => Ok(GeneratedField::StartTick),
                            "lengthTicks" | "length_ticks" => Ok(GeneratedField::LengthTicks),
                            "loopLengthTicks" | "loop_length_ticks" => Ok(GeneratedField::LoopLengthTicks),
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
            type Value = Clip;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct escribass.song.v1.Clip")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<Clip, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut id__ = None;
                let mut provenance__ = None;
                let mut version__ = None;
                let mut track_id__ = None;
                let mut start_tick__ = None;
                let mut length_ticks__ = None;
                let mut loop_length_ticks__ = None;
                let mut content__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Id => {
                            if id__.is_some() {
                                return Err(serde::de::Error::duplicate_field("id"));
                            }
                            id__ = Some(map_.next_value()?);
                        }
                        GeneratedField::Provenance => {
                            if provenance__.is_some() {
                                return Err(serde::de::Error::duplicate_field("provenance"));
                            }
                            provenance__ = map_.next_value()?;
                        }
                        GeneratedField::Version => {
                            if version__.is_some() {
                                return Err(serde::de::Error::duplicate_field("version"));
                            }
                            version__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
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
                        GeneratedField::LoopLengthTicks => {
                            if loop_length_ticks__.is_some() {
                                return Err(serde::de::Error::duplicate_field("loopLengthTicks"));
                            }
                            loop_length_ticks__ = 
                                map_.next_value::<::std::option::Option<::pbjson::private::NumberDeserialize<_>>>()?.map(|x| x.0)
                            ;
                        }
                        GeneratedField::NoteClip => {
                            if content__.is_some() {
                                return Err(serde::de::Error::duplicate_field("noteClip"));
                            }
                            content__ = map_.next_value::<::std::option::Option<_>>()?.map(clip::Content::NoteClip)
;
                        }
                        GeneratedField::AudioClip => {
                            if content__.is_some() {
                                return Err(serde::de::Error::duplicate_field("audioClip"));
                            }
                            content__ = map_.next_value::<::std::option::Option<_>>()?.map(clip::Content::AudioClip)
;
                        }
                    }
                }
                Ok(Clip {
                    id: id__.unwrap_or_default(),
                    provenance: provenance__,
                    version: version__.unwrap_or_default(),
                    track_id: track_id__.unwrap_or_default(),
                    start_tick: start_tick__.unwrap_or_default(),
                    length_ticks: length_ticks__.unwrap_or_default(),
                    loop_length_ticks: loop_length_ticks__,
                    content: content__,
                })
            }
        }
        deserializer.deserialize_struct("escribass.song.v1.Clip", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for Curve {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let variant = match self {
            Self::Unspecified => "CURVE_UNSPECIFIED",
            Self::Linear => "CURVE_LINEAR",
            Self::Hold => "CURVE_HOLD",
        };
        serializer.serialize_str(variant)
    }
}
impl<'de> serde::Deserialize<'de> for Curve {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "CURVE_UNSPECIFIED",
            "CURVE_LINEAR",
            "CURVE_HOLD",
        ];

        struct GeneratedVisitor;

        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = Curve;

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
                    "CURVE_UNSPECIFIED" => Ok(Curve::Unspecified),
                    "CURVE_LINEAR" => Ok(Curve::Linear),
                    "CURVE_HOLD" => Ok(Curve::Hold),
                    _ => Err(serde::de::Error::unknown_variant(value, FIELDS)),
                }
            }
        }
        deserializer.deserialize_any(GeneratedVisitor)
    }
}
impl serde::Serialize for DeviceRef {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.kind.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("escribass.song.v1.DeviceRef", len)?;
        if let Some(v) = self.kind.as_ref() {
            match v {
                device_ref::Kind::Plugin(v) => {
                    struct_ser.serialize_field("plugin", v)?;
                }
                device_ref::Kind::Cmajor(v) => {
                    struct_ser.serialize_field("cmajor", v)?;
                }
                device_ref::Kind::Faust(v) => {
                    struct_ser.serialize_field("faust", v)?;
                }
                device_ref::Kind::Neural(v) => {
                    struct_ser.serialize_field("neural", v)?;
                }
                device_ref::Kind::Sampler(v) => {
                    struct_ser.serialize_field("sampler", v)?;
                }
            }
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for DeviceRef {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "plugin",
            "cmajor",
            "faust",
            "neural",
            "sampler",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Plugin,
            Cmajor,
            Faust,
            Neural,
            Sampler,
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
                            "plugin" => Ok(GeneratedField::Plugin),
                            "cmajor" => Ok(GeneratedField::Cmajor),
                            "faust" => Ok(GeneratedField::Faust),
                            "neural" => Ok(GeneratedField::Neural),
                            "sampler" => Ok(GeneratedField::Sampler),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = DeviceRef;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct escribass.song.v1.DeviceRef")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<DeviceRef, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut kind__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Plugin => {
                            if kind__.is_some() {
                                return Err(serde::de::Error::duplicate_field("plugin"));
                            }
                            kind__ = map_.next_value::<::std::option::Option<_>>()?.map(device_ref::Kind::Plugin)
;
                        }
                        GeneratedField::Cmajor => {
                            if kind__.is_some() {
                                return Err(serde::de::Error::duplicate_field("cmajor"));
                            }
                            kind__ = map_.next_value::<::std::option::Option<_>>()?.map(device_ref::Kind::Cmajor)
;
                        }
                        GeneratedField::Faust => {
                            if kind__.is_some() {
                                return Err(serde::de::Error::duplicate_field("faust"));
                            }
                            kind__ = map_.next_value::<::std::option::Option<_>>()?.map(device_ref::Kind::Faust)
;
                        }
                        GeneratedField::Neural => {
                            if kind__.is_some() {
                                return Err(serde::de::Error::duplicate_field("neural"));
                            }
                            kind__ = map_.next_value::<::std::option::Option<_>>()?.map(device_ref::Kind::Neural)
;
                        }
                        GeneratedField::Sampler => {
                            if kind__.is_some() {
                                return Err(serde::de::Error::duplicate_field("sampler"));
                            }
                            kind__ = map_.next_value::<::std::option::Option<_>>()?.map(device_ref::Kind::Sampler)
;
                        }
                    }
                }
                Ok(DeviceRef {
                    kind: kind__,
                })
            }
        }
        deserializer.deserialize_struct("escribass.song.v1.DeviceRef", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for Effect {
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
        let mut struct_ser = serializer.serialize_struct("escribass.song.v1.Effect", len)?;
        if true {
            struct_ser.serialize_field("id", &self.id)?;
        }
        if let Some(v) = self.provenance.as_ref() {
            struct_ser.serialize_field("provenance", v)?;
        }
        if true {
            struct_ser.serialize_field("version", &self.version)?;
        }
        if let Some(v) = self.r#ref.as_ref() {
            struct_ser.serialize_field("ref", v)?;
        }
        if true {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("state", pbjson::private::base64::encode(&self.state).as_str())?;
        }
        if true {
            struct_ser.serialize_field("params", &self.params)?;
        }
        if true {
            struct_ser.serialize_field("index", &self.index)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for Effect {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "id",
            "provenance",
            "version",
            "ref",
            "state",
            "params",
            "index",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Id,
            Provenance,
            Version,
            Ref,
            State,
            Params,
            Index,
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
                            "provenance" => Ok(GeneratedField::Provenance),
                            "version" => Ok(GeneratedField::Version),
                            "ref" => Ok(GeneratedField::Ref),
                            "state" => Ok(GeneratedField::State),
                            "params" => Ok(GeneratedField::Params),
                            "index" => Ok(GeneratedField::Index),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = Effect;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct escribass.song.v1.Effect")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<Effect, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut id__ = None;
                let mut provenance__ = None;
                let mut version__ = None;
                let mut r#ref__ = None;
                let mut state__ = None;
                let mut params__ = None;
                let mut index__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Id => {
                            if id__.is_some() {
                                return Err(serde::de::Error::duplicate_field("id"));
                            }
                            id__ = Some(map_.next_value()?);
                        }
                        GeneratedField::Provenance => {
                            if provenance__.is_some() {
                                return Err(serde::de::Error::duplicate_field("provenance"));
                            }
                            provenance__ = map_.next_value()?;
                        }
                        GeneratedField::Version => {
                            if version__.is_some() {
                                return Err(serde::de::Error::duplicate_field("version"));
                            }
                            version__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::Ref => {
                            if r#ref__.is_some() {
                                return Err(serde::de::Error::duplicate_field("ref"));
                            }
                            r#ref__ = map_.next_value()?;
                        }
                        GeneratedField::State => {
                            if state__.is_some() {
                                return Err(serde::de::Error::duplicate_field("state"));
                            }
                            state__ = 
                                Some(map_.next_value::<::pbjson::private::BytesDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::Params => {
                            if params__.is_some() {
                                return Err(serde::de::Error::duplicate_field("params"));
                            }
                            params__ = Some(
                                map_.next_value::<std::collections::BTreeMap<_, ::pbjson::private::NumberDeserialize<f64>>>()?
                                    .into_iter().map(|(k,v)| (k, v.0)).collect()
                            );
                        }
                        GeneratedField::Index => {
                            if index__.is_some() {
                                return Err(serde::de::Error::duplicate_field("index"));
                            }
                            index__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                    }
                }
                Ok(Effect {
                    id: id__.unwrap_or_default(),
                    provenance: provenance__,
                    version: version__.unwrap_or_default(),
                    r#ref: r#ref__,
                    state: state__.unwrap_or_default(),
                    params: params__.unwrap_or_default(),
                    index: index__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("escribass.song.v1.Effect", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for Generator {
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
        if self.target.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("escribass.song.v1.Generator", len)?;
        if true {
            struct_ser.serialize_field("id", &self.id)?;
        }
        if let Some(v) = self.provenance.as_ref() {
            struct_ser.serialize_field("provenance", v)?;
        }
        if true {
            struct_ser.serialize_field("version", &self.version)?;
        }
        if true {
            let v = GeneratorKind::try_from(self.kind)
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
            struct_ser.serialize_field("toolchain_version", &self.toolchain_version)?;
        }
        if true {
            struct_ser.serialize_field("params", &self.params)?;
        }
        if let Some(v) = self.target.as_ref() {
            match v {
                generator::Target::TrackId(v) => {
                    struct_ser.serialize_field("track_id", v)?;
                }
                generator::Target::ClipId(v) => {
                    struct_ser.serialize_field("clip_id", v)?;
                }
            }
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for Generator {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "id",
            "provenance",
            "version",
            "kind",
            "source",
            "seed",
            "toolchain_version",
            "toolchainVersion",
            "params",
            "track_id",
            "trackId",
            "clip_id",
            "clipId",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Id,
            Provenance,
            Version,
            Kind,
            Source,
            Seed,
            ToolchainVersion,
            Params,
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
                            "id" => Ok(GeneratedField::Id),
                            "provenance" => Ok(GeneratedField::Provenance),
                            "version" => Ok(GeneratedField::Version),
                            "kind" => Ok(GeneratedField::Kind),
                            "source" => Ok(GeneratedField::Source),
                            "seed" => Ok(GeneratedField::Seed),
                            "toolchainVersion" | "toolchain_version" => Ok(GeneratedField::ToolchainVersion),
                            "params" => Ok(GeneratedField::Params),
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
            type Value = Generator;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct escribass.song.v1.Generator")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<Generator, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut id__ = None;
                let mut provenance__ = None;
                let mut version__ = None;
                let mut kind__ = None;
                let mut source__ = None;
                let mut seed__ = None;
                let mut toolchain_version__ = None;
                let mut params__ = None;
                let mut target__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Id => {
                            if id__.is_some() {
                                return Err(serde::de::Error::duplicate_field("id"));
                            }
                            id__ = Some(map_.next_value()?);
                        }
                        GeneratedField::Provenance => {
                            if provenance__.is_some() {
                                return Err(serde::de::Error::duplicate_field("provenance"));
                            }
                            provenance__ = map_.next_value()?;
                        }
                        GeneratedField::Version => {
                            if version__.is_some() {
                                return Err(serde::de::Error::duplicate_field("version"));
                            }
                            version__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::Kind => {
                            if kind__.is_some() {
                                return Err(serde::de::Error::duplicate_field("kind"));
                            }
                            kind__ = Some(map_.next_value::<GeneratorKind>()? as i32);
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
                        GeneratedField::ToolchainVersion => {
                            if toolchain_version__.is_some() {
                                return Err(serde::de::Error::duplicate_field("toolchainVersion"));
                            }
                            toolchain_version__ = Some(map_.next_value()?);
                        }
                        GeneratedField::Params => {
                            if params__.is_some() {
                                return Err(serde::de::Error::duplicate_field("params"));
                            }
                            params__ = Some(
                                map_.next_value::<std::collections::BTreeMap<_, _>>()?
                            );
                        }
                        GeneratedField::TrackId => {
                            if target__.is_some() {
                                return Err(serde::de::Error::duplicate_field("trackId"));
                            }
                            target__ = map_.next_value::<::std::option::Option<_>>()?.map(generator::Target::TrackId);
                        }
                        GeneratedField::ClipId => {
                            if target__.is_some() {
                                return Err(serde::de::Error::duplicate_field("clipId"));
                            }
                            target__ = map_.next_value::<::std::option::Option<_>>()?.map(generator::Target::ClipId);
                        }
                    }
                }
                Ok(Generator {
                    id: id__.unwrap_or_default(),
                    provenance: provenance__,
                    version: version__.unwrap_or_default(),
                    kind: kind__.unwrap_or_default(),
                    source: source__.unwrap_or_default(),
                    seed: seed__.unwrap_or_default(),
                    toolchain_version: toolchain_version__.unwrap_or_default(),
                    params: params__.unwrap_or_default(),
                    target: target__,
                })
            }
        }
        deserializer.deserialize_struct("escribass.song.v1.Generator", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for GeneratorKind {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let variant = match self {
            Self::Unspecified => "GENERATOR_KIND_UNSPECIFIED",
            Self::Python => "GENERATOR_KIND_PYTHON",
        };
        serializer.serialize_str(variant)
    }
}
impl<'de> serde::Deserialize<'de> for GeneratorKind {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "GENERATOR_KIND_UNSPECIFIED",
            "GENERATOR_KIND_PYTHON",
        ];

        struct GeneratedVisitor;

        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = GeneratorKind;

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
                    "GENERATOR_KIND_UNSPECIFIED" => Ok(GeneratorKind::Unspecified),
                    "GENERATOR_KIND_PYTHON" => Ok(GeneratorKind::Python),
                    _ => Err(serde::de::Error::unknown_variant(value, FIELDS)),
                }
            }
        }
        deserializer.deserialize_any(GeneratedVisitor)
    }
}
impl serde::Serialize for Instrument {
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
        let mut struct_ser = serializer.serialize_struct("escribass.song.v1.Instrument", len)?;
        if true {
            struct_ser.serialize_field("id", &self.id)?;
        }
        if let Some(v) = self.provenance.as_ref() {
            struct_ser.serialize_field("provenance", v)?;
        }
        if true {
            struct_ser.serialize_field("version", &self.version)?;
        }
        if let Some(v) = self.r#ref.as_ref() {
            struct_ser.serialize_field("ref", v)?;
        }
        if true {
            #[allow(clippy::needless_borrow)]
            #[allow(clippy::needless_borrows_for_generic_args)]
            struct_ser.serialize_field("state", pbjson::private::base64::encode(&self.state).as_str())?;
        }
        if true {
            struct_ser.serialize_field("params", &self.params)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for Instrument {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "id",
            "provenance",
            "version",
            "ref",
            "state",
            "params",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Id,
            Provenance,
            Version,
            Ref,
            State,
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
                            "id" => Ok(GeneratedField::Id),
                            "provenance" => Ok(GeneratedField::Provenance),
                            "version" => Ok(GeneratedField::Version),
                            "ref" => Ok(GeneratedField::Ref),
                            "state" => Ok(GeneratedField::State),
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
            type Value = Instrument;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct escribass.song.v1.Instrument")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<Instrument, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut id__ = None;
                let mut provenance__ = None;
                let mut version__ = None;
                let mut r#ref__ = None;
                let mut state__ = None;
                let mut params__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Id => {
                            if id__.is_some() {
                                return Err(serde::de::Error::duplicate_field("id"));
                            }
                            id__ = Some(map_.next_value()?);
                        }
                        GeneratedField::Provenance => {
                            if provenance__.is_some() {
                                return Err(serde::de::Error::duplicate_field("provenance"));
                            }
                            provenance__ = map_.next_value()?;
                        }
                        GeneratedField::Version => {
                            if version__.is_some() {
                                return Err(serde::de::Error::duplicate_field("version"));
                            }
                            version__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::Ref => {
                            if r#ref__.is_some() {
                                return Err(serde::de::Error::duplicate_field("ref"));
                            }
                            r#ref__ = map_.next_value()?;
                        }
                        GeneratedField::State => {
                            if state__.is_some() {
                                return Err(serde::de::Error::duplicate_field("state"));
                            }
                            state__ = 
                                Some(map_.next_value::<::pbjson::private::BytesDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::Params => {
                            if params__.is_some() {
                                return Err(serde::de::Error::duplicate_field("params"));
                            }
                            params__ = Some(
                                map_.next_value::<std::collections::BTreeMap<_, ::pbjson::private::NumberDeserialize<f64>>>()?
                                    .into_iter().map(|(k,v)| (k, v.0)).collect()
                            );
                        }
                    }
                }
                Ok(Instrument {
                    id: id__.unwrap_or_default(),
                    provenance: provenance__,
                    version: version__.unwrap_or_default(),
                    r#ref: r#ref__,
                    state: state__.unwrap_or_default(),
                    params: params__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("escribass.song.v1.Instrument", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for Marker {
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
        let mut struct_ser = serializer.serialize_struct("escribass.song.v1.Marker", len)?;
        if true {
            struct_ser.serialize_field("id", &self.id)?;
        }
        if let Some(v) = self.provenance.as_ref() {
            struct_ser.serialize_field("provenance", v)?;
        }
        if true {
            struct_ser.serialize_field("version", &self.version)?;
        }
        if true {
            struct_ser.serialize_field("name", &self.name)?;
        }
        if true {
            struct_ser.serialize_field("tick", &self.tick)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for Marker {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "id",
            "provenance",
            "version",
            "name",
            "tick",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Id,
            Provenance,
            Version,
            Name,
            Tick,
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
                            "provenance" => Ok(GeneratedField::Provenance),
                            "version" => Ok(GeneratedField::Version),
                            "name" => Ok(GeneratedField::Name),
                            "tick" => Ok(GeneratedField::Tick),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = Marker;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct escribass.song.v1.Marker")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<Marker, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut id__ = None;
                let mut provenance__ = None;
                let mut version__ = None;
                let mut name__ = None;
                let mut tick__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Id => {
                            if id__.is_some() {
                                return Err(serde::de::Error::duplicate_field("id"));
                            }
                            id__ = Some(map_.next_value()?);
                        }
                        GeneratedField::Provenance => {
                            if provenance__.is_some() {
                                return Err(serde::de::Error::duplicate_field("provenance"));
                            }
                            provenance__ = map_.next_value()?;
                        }
                        GeneratedField::Version => {
                            if version__.is_some() {
                                return Err(serde::de::Error::duplicate_field("version"));
                            }
                            version__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::Name => {
                            if name__.is_some() {
                                return Err(serde::de::Error::duplicate_field("name"));
                            }
                            name__ = Some(map_.next_value()?);
                        }
                        GeneratedField::Tick => {
                            if tick__.is_some() {
                                return Err(serde::de::Error::duplicate_field("tick"));
                            }
                            tick__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                    }
                }
                Ok(Marker {
                    id: id__.unwrap_or_default(),
                    provenance: provenance__,
                    version: version__.unwrap_or_default(),
                    name: name__.unwrap_or_default(),
                    tick: tick__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("escribass.song.v1.Marker", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for Mix {
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
        let mut struct_ser = serializer.serialize_struct("escribass.song.v1.Mix", len)?;
        if true {
            struct_ser.serialize_field("gain_db", &self.gain_db)?;
        }
        if true {
            struct_ser.serialize_field("pan", &self.pan)?;
        }
        if true {
            struct_ser.serialize_field("mute", &self.mute)?;
        }
        if true {
            struct_ser.serialize_field("solo", &self.solo)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for Mix {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "gain_db",
            "gainDb",
            "pan",
            "mute",
            "solo",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            GainDb,
            Pan,
            Mute,
            Solo,
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
                            "gainDb" | "gain_db" => Ok(GeneratedField::GainDb),
                            "pan" => Ok(GeneratedField::Pan),
                            "mute" => Ok(GeneratedField::Mute),
                            "solo" => Ok(GeneratedField::Solo),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = Mix;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct escribass.song.v1.Mix")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<Mix, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut gain_db__ = None;
                let mut pan__ = None;
                let mut mute__ = None;
                let mut solo__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::GainDb => {
                            if gain_db__.is_some() {
                                return Err(serde::de::Error::duplicate_field("gainDb"));
                            }
                            gain_db__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::Pan => {
                            if pan__.is_some() {
                                return Err(serde::de::Error::duplicate_field("pan"));
                            }
                            pan__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::Mute => {
                            if mute__.is_some() {
                                return Err(serde::de::Error::duplicate_field("mute"));
                            }
                            mute__ = Some(map_.next_value()?);
                        }
                        GeneratedField::Solo => {
                            if solo__.is_some() {
                                return Err(serde::de::Error::duplicate_field("solo"));
                            }
                            solo__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(Mix {
                    gain_db: gain_db__.unwrap_or_default(),
                    pan: pan__.unwrap_or_default(),
                    mute: mute__.unwrap_or_default(),
                    solo: solo__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("escribass.song.v1.Mix", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for ModelRef {
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
        let mut struct_ser = serializer.serialize_struct("escribass.song.v1.ModelRef", len)?;
        if true {
            struct_ser.serialize_field("model_hash", &self.model_hash)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for ModelRef {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "model_hash",
            "modelHash",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            ModelHash,
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
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = ModelRef;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct escribass.song.v1.ModelRef")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<ModelRef, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut model_hash__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::ModelHash => {
                            if model_hash__.is_some() {
                                return Err(serde::de::Error::duplicate_field("modelHash"));
                            }
                            model_hash__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(ModelRef {
                    model_hash: model_hash__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("escribass.song.v1.ModelRef", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for Note {
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
        let mut struct_ser = serializer.serialize_struct("escribass.song.v1.Note", len)?;
        if true {
            struct_ser.serialize_field("id", &self.id)?;
        }
        if let Some(v) = self.provenance.as_ref() {
            struct_ser.serialize_field("provenance", v)?;
        }
        if true {
            struct_ser.serialize_field("version", &self.version)?;
        }
        if true {
            struct_ser.serialize_field("pitch", &self.pitch)?;
        }
        if true {
            struct_ser.serialize_field("microtonal_cents", &self.microtonal_cents)?;
        }
        if true {
            struct_ser.serialize_field("start_tick", &self.start_tick)?;
        }
        if true {
            struct_ser.serialize_field("length_ticks", &self.length_ticks)?;
        }
        if true {
            struct_ser.serialize_field("velocity", &self.velocity)?;
        }
        if true {
            struct_ser.serialize_field("expression", &self.expression)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for Note {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "id",
            "provenance",
            "version",
            "pitch",
            "microtonal_cents",
            "microtonalCents",
            "start_tick",
            "startTick",
            "length_ticks",
            "lengthTicks",
            "velocity",
            "expression",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Id,
            Provenance,
            Version,
            Pitch,
            MicrotonalCents,
            StartTick,
            LengthTicks,
            Velocity,
            Expression,
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
                            "provenance" => Ok(GeneratedField::Provenance),
                            "version" => Ok(GeneratedField::Version),
                            "pitch" => Ok(GeneratedField::Pitch),
                            "microtonalCents" | "microtonal_cents" => Ok(GeneratedField::MicrotonalCents),
                            "startTick" | "start_tick" => Ok(GeneratedField::StartTick),
                            "lengthTicks" | "length_ticks" => Ok(GeneratedField::LengthTicks),
                            "velocity" => Ok(GeneratedField::Velocity),
                            "expression" => Ok(GeneratedField::Expression),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = Note;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct escribass.song.v1.Note")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<Note, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut id__ = None;
                let mut provenance__ = None;
                let mut version__ = None;
                let mut pitch__ = None;
                let mut microtonal_cents__ = None;
                let mut start_tick__ = None;
                let mut length_ticks__ = None;
                let mut velocity__ = None;
                let mut expression__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Id => {
                            if id__.is_some() {
                                return Err(serde::de::Error::duplicate_field("id"));
                            }
                            id__ = Some(map_.next_value()?);
                        }
                        GeneratedField::Provenance => {
                            if provenance__.is_some() {
                                return Err(serde::de::Error::duplicate_field("provenance"));
                            }
                            provenance__ = map_.next_value()?;
                        }
                        GeneratedField::Version => {
                            if version__.is_some() {
                                return Err(serde::de::Error::duplicate_field("version"));
                            }
                            version__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::Pitch => {
                            if pitch__.is_some() {
                                return Err(serde::de::Error::duplicate_field("pitch"));
                            }
                            pitch__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::MicrotonalCents => {
                            if microtonal_cents__.is_some() {
                                return Err(serde::de::Error::duplicate_field("microtonalCents"));
                            }
                            microtonal_cents__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
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
                        GeneratedField::Velocity => {
                            if velocity__.is_some() {
                                return Err(serde::de::Error::duplicate_field("velocity"));
                            }
                            velocity__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::Expression => {
                            if expression__.is_some() {
                                return Err(serde::de::Error::duplicate_field("expression"));
                            }
                            expression__ = Some(
                                map_.next_value::<std::collections::BTreeMap<_, ::pbjson::private::NumberDeserialize<f64>>>()?
                                    .into_iter().map(|(k,v)| (k, v.0)).collect()
                            );
                        }
                    }
                }
                Ok(Note {
                    id: id__.unwrap_or_default(),
                    provenance: provenance__,
                    version: version__.unwrap_or_default(),
                    pitch: pitch__.unwrap_or_default(),
                    microtonal_cents: microtonal_cents__.unwrap_or_default(),
                    start_tick: start_tick__.unwrap_or_default(),
                    length_ticks: length_ticks__.unwrap_or_default(),
                    velocity: velocity__.unwrap_or_default(),
                    expression: expression__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("escribass.song.v1.Note", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for NoteClip {
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
        let mut struct_ser = serializer.serialize_struct("escribass.song.v1.NoteClip", len)?;
        if true {
            struct_ser.serialize_field("notes", &self.notes)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for NoteClip {
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
            type Value = NoteClip;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct escribass.song.v1.NoteClip")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<NoteClip, V::Error>
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
                            notes__ = Some(
                                map_.next_value::<std::collections::BTreeMap<_, _>>()?
                            );
                        }
                    }
                }
                Ok(NoteClip {
                    notes: notes__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("escribass.song.v1.NoteClip", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for ParamRef {
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
        let mut struct_ser = serializer.serialize_struct("escribass.song.v1.ParamRef", len)?;
        if true {
            struct_ser.serialize_field("device_id", &self.device_id)?;
        }
        if true {
            struct_ser.serialize_field("param", &self.param)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for ParamRef {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "device_id",
            "deviceId",
            "param",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            DeviceId,
            Param,
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
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = ParamRef;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct escribass.song.v1.ParamRef")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<ParamRef, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut device_id__ = None;
                let mut param__ = None;
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
                    }
                }
                Ok(ParamRef {
                    device_id: device_id__.unwrap_or_default(),
                    param: param__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("escribass.song.v1.ParamRef", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for PluginRef {
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
        let mut struct_ser = serializer.serialize_struct("escribass.song.v1.PluginRef", len)?;
        if true {
            struct_ser.serialize_field("plugin_id", &self.plugin_id)?;
        }
        if true {
            struct_ser.serialize_field("version", &self.version)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for PluginRef {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "plugin_id",
            "pluginId",
            "version",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            PluginId,
            Version,
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
                            "pluginId" | "plugin_id" => Ok(GeneratedField::PluginId),
                            "version" => Ok(GeneratedField::Version),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = PluginRef;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct escribass.song.v1.PluginRef")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<PluginRef, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut plugin_id__ = None;
                let mut version__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::PluginId => {
                            if plugin_id__.is_some() {
                                return Err(serde::de::Error::duplicate_field("pluginId"));
                            }
                            plugin_id__ = Some(map_.next_value()?);
                        }
                        GeneratedField::Version => {
                            if version__.is_some() {
                                return Err(serde::de::Error::duplicate_field("version"));
                            }
                            version__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(PluginRef {
                    plugin_id: plugin_id__.unwrap_or_default(),
                    version: version__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("escribass.song.v1.PluginRef", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for Provenance {
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
        let mut struct_ser = serializer.serialize_struct("escribass.song.v1.Provenance", len)?;
        if true {
            let v = Author::try_from(self.author)
                .map_err(|_| serde::ser::Error::custom(format!("Invalid variant {}", self.author)))?;
            struct_ser.serialize_field("author", &v)?;
        }
        if let Some(v) = self.model_id.as_ref() {
            struct_ser.serialize_field("model_id", v)?;
        }
        if let Some(v) = self.prompt_id.as_ref() {
            struct_ser.serialize_field("prompt_id", v)?;
        }
        if let Some(v) = self.tool_call_id.as_ref() {
            struct_ser.serialize_field("tool_call_id", v)?;
        }
        if let Some(v) = self.created_at.as_ref() {
            struct_ser.serialize_field("created_at", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for Provenance {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "author",
            "model_id",
            "modelId",
            "prompt_id",
            "promptId",
            "tool_call_id",
            "toolCallId",
            "created_at",
            "createdAt",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Author,
            ModelId,
            PromptId,
            ToolCallId,
            CreatedAt,
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
                            "author" => Ok(GeneratedField::Author),
                            "modelId" | "model_id" => Ok(GeneratedField::ModelId),
                            "promptId" | "prompt_id" => Ok(GeneratedField::PromptId),
                            "toolCallId" | "tool_call_id" => Ok(GeneratedField::ToolCallId),
                            "createdAt" | "created_at" => Ok(GeneratedField::CreatedAt),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = Provenance;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct escribass.song.v1.Provenance")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<Provenance, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut author__ = None;
                let mut model_id__ = None;
                let mut prompt_id__ = None;
                let mut tool_call_id__ = None;
                let mut created_at__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Author => {
                            if author__.is_some() {
                                return Err(serde::de::Error::duplicate_field("author"));
                            }
                            author__ = Some(map_.next_value::<Author>()? as i32);
                        }
                        GeneratedField::ModelId => {
                            if model_id__.is_some() {
                                return Err(serde::de::Error::duplicate_field("modelId"));
                            }
                            model_id__ = map_.next_value()?;
                        }
                        GeneratedField::PromptId => {
                            if prompt_id__.is_some() {
                                return Err(serde::de::Error::duplicate_field("promptId"));
                            }
                            prompt_id__ = map_.next_value()?;
                        }
                        GeneratedField::ToolCallId => {
                            if tool_call_id__.is_some() {
                                return Err(serde::de::Error::duplicate_field("toolCallId"));
                            }
                            tool_call_id__ = map_.next_value()?;
                        }
                        GeneratedField::CreatedAt => {
                            if created_at__.is_some() {
                                return Err(serde::de::Error::duplicate_field("createdAt"));
                            }
                            created_at__ = map_.next_value()?;
                        }
                    }
                }
                Ok(Provenance {
                    author: author__.unwrap_or_default(),
                    model_id: model_id__,
                    prompt_id: prompt_id__,
                    tool_call_id: tool_call_id__,
                    created_at: created_at__,
                })
            }
        }
        deserializer.deserialize_struct("escribass.song.v1.Provenance", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for RenderKind {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let variant = match self {
            Self::Unspecified => "RENDER_KIND_UNSPECIFIED",
            Self::Master => "RENDER_KIND_MASTER",
            Self::Stems => "RENDER_KIND_STEMS",
        };
        serializer.serialize_str(variant)
    }
}
impl<'de> serde::Deserialize<'de> for RenderKind {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "RENDER_KIND_UNSPECIFIED",
            "RENDER_KIND_MASTER",
            "RENDER_KIND_STEMS",
        ];

        struct GeneratedVisitor;

        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = RenderKind;

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
                    "RENDER_KIND_UNSPECIFIED" => Ok(RenderKind::Unspecified),
                    "RENDER_KIND_MASTER" => Ok(RenderKind::Master),
                    "RENDER_KIND_STEMS" => Ok(RenderKind::Stems),
                    _ => Err(serde::de::Error::unknown_variant(value, FIELDS)),
                }
            }
        }
        deserializer.deserialize_any(GeneratedVisitor)
    }
}
impl serde::Serialize for RenderTarget {
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
        let mut struct_ser = serializer.serialize_struct("escribass.song.v1.RenderTarget", len)?;
        if true {
            let v = RenderKind::try_from(self.kind)
                .map_err(|_| serde::ser::Error::custom(format!("Invalid variant {}", self.kind)))?;
            struct_ser.serialize_field("kind", &v)?;
        }
        if true {
            struct_ser.serialize_field("sample_rate", &self.sample_rate)?;
        }
        if true {
            struct_ser.serialize_field("bit_depth", &self.bit_depth)?;
        }
        if true {
            struct_ser.serialize_field("dither", &self.dither)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for RenderTarget {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "kind",
            "sample_rate",
            "sampleRate",
            "bit_depth",
            "bitDepth",
            "dither",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Kind,
            SampleRate,
            BitDepth,
            Dither,
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
                            "sampleRate" | "sample_rate" => Ok(GeneratedField::SampleRate),
                            "bitDepth" | "bit_depth" => Ok(GeneratedField::BitDepth),
                            "dither" => Ok(GeneratedField::Dither),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = RenderTarget;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct escribass.song.v1.RenderTarget")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<RenderTarget, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut kind__ = None;
                let mut sample_rate__ = None;
                let mut bit_depth__ = None;
                let mut dither__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Kind => {
                            if kind__.is_some() {
                                return Err(serde::de::Error::duplicate_field("kind"));
                            }
                            kind__ = Some(map_.next_value::<RenderKind>()? as i32);
                        }
                        GeneratedField::SampleRate => {
                            if sample_rate__.is_some() {
                                return Err(serde::de::Error::duplicate_field("sampleRate"));
                            }
                            sample_rate__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::BitDepth => {
                            if bit_depth__.is_some() {
                                return Err(serde::de::Error::duplicate_field("bitDepth"));
                            }
                            bit_depth__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::Dither => {
                            if dither__.is_some() {
                                return Err(serde::de::Error::duplicate_field("dither"));
                            }
                            dither__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(RenderTarget {
                    kind: kind__.unwrap_or_default(),
                    sample_rate: sample_rate__.unwrap_or_default(),
                    bit_depth: bit_depth__.unwrap_or_default(),
                    dither: dither__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("escribass.song.v1.RenderTarget", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for Routing {
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
        let mut struct_ser = serializer.serialize_struct("escribass.song.v1.Routing", len)?;
        if let Some(v) = self.output_track_id.as_ref() {
            struct_ser.serialize_field("output_track_id", v)?;
        }
        if true {
            struct_ser.serialize_field("sends", &self.sends)?;
        }
        if true {
            struct_ser.serialize_field("sidechains", &self.sidechains)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for Routing {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "output_track_id",
            "outputTrackId",
            "sends",
            "sidechains",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            OutputTrackId,
            Sends,
            Sidechains,
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
                            "outputTrackId" | "output_track_id" => Ok(GeneratedField::OutputTrackId),
                            "sends" => Ok(GeneratedField::Sends),
                            "sidechains" => Ok(GeneratedField::Sidechains),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = Routing;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct escribass.song.v1.Routing")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<Routing, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut output_track_id__ = None;
                let mut sends__ = None;
                let mut sidechains__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::OutputTrackId => {
                            if output_track_id__.is_some() {
                                return Err(serde::de::Error::duplicate_field("outputTrackId"));
                            }
                            output_track_id__ = map_.next_value()?;
                        }
                        GeneratedField::Sends => {
                            if sends__.is_some() {
                                return Err(serde::de::Error::duplicate_field("sends"));
                            }
                            sends__ = Some(
                                map_.next_value::<std::collections::BTreeMap<_, ::pbjson::private::NumberDeserialize<f64>>>()?
                                    .into_iter().map(|(k,v)| (k, v.0)).collect()
                            );
                        }
                        GeneratedField::Sidechains => {
                            if sidechains__.is_some() {
                                return Err(serde::de::Error::duplicate_field("sidechains"));
                            }
                            sidechains__ = Some(
                                map_.next_value::<std::collections::BTreeMap<_, _>>()?
                            );
                        }
                    }
                }
                Ok(Routing {
                    output_track_id: output_track_id__,
                    sends: sends__.unwrap_or_default(),
                    sidechains: sidechains__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("escribass.song.v1.Routing", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for SamplerRef {
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
        let mut struct_ser = serializer.serialize_struct("escribass.song.v1.SamplerRef", len)?;
        if true {
            struct_ser.serialize_field("sfz_hash", &self.sfz_hash)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for SamplerRef {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "sfz_hash",
            "sfzHash",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            SfzHash,
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
                            "sfzHash" | "sfz_hash" => Ok(GeneratedField::SfzHash),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = SamplerRef;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct escribass.song.v1.SamplerRef")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<SamplerRef, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut sfz_hash__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::SfzHash => {
                            if sfz_hash__.is_some() {
                                return Err(serde::de::Error::duplicate_field("sfzHash"));
                            }
                            sfz_hash__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(SamplerRef {
                    sfz_hash: sfz_hash__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("escribass.song.v1.SamplerRef", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for Section {
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
        let mut struct_ser = serializer.serialize_struct("escribass.song.v1.Section", len)?;
        if true {
            struct_ser.serialize_field("id", &self.id)?;
        }
        if let Some(v) = self.provenance.as_ref() {
            struct_ser.serialize_field("provenance", v)?;
        }
        if true {
            struct_ser.serialize_field("version", &self.version)?;
        }
        if true {
            struct_ser.serialize_field("name", &self.name)?;
        }
        if true {
            struct_ser.serialize_field("start_tick", &self.start_tick)?;
        }
        if true {
            struct_ser.serialize_field("end_tick", &self.end_tick)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for Section {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "id",
            "provenance",
            "version",
            "name",
            "start_tick",
            "startTick",
            "end_tick",
            "endTick",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Id,
            Provenance,
            Version,
            Name,
            StartTick,
            EndTick,
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
                            "provenance" => Ok(GeneratedField::Provenance),
                            "version" => Ok(GeneratedField::Version),
                            "name" => Ok(GeneratedField::Name),
                            "startTick" | "start_tick" => Ok(GeneratedField::StartTick),
                            "endTick" | "end_tick" => Ok(GeneratedField::EndTick),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = Section;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct escribass.song.v1.Section")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<Section, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut id__ = None;
                let mut provenance__ = None;
                let mut version__ = None;
                let mut name__ = None;
                let mut start_tick__ = None;
                let mut end_tick__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Id => {
                            if id__.is_some() {
                                return Err(serde::de::Error::duplicate_field("id"));
                            }
                            id__ = Some(map_.next_value()?);
                        }
                        GeneratedField::Provenance => {
                            if provenance__.is_some() {
                                return Err(serde::de::Error::duplicate_field("provenance"));
                            }
                            provenance__ = map_.next_value()?;
                        }
                        GeneratedField::Version => {
                            if version__.is_some() {
                                return Err(serde::de::Error::duplicate_field("version"));
                            }
                            version__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
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
                    }
                }
                Ok(Section {
                    id: id__.unwrap_or_default(),
                    provenance: provenance__,
                    version: version__.unwrap_or_default(),
                    name: name__.unwrap_or_default(),
                    start_tick: start_tick__.unwrap_or_default(),
                    end_tick: end_tick__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("escribass.song.v1.Section", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for Song {
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
        let mut struct_ser = serializer.serialize_struct("escribass.song.v1.Song", len)?;
        if true {
            struct_ser.serialize_field("id", &self.id)?;
        }
        if let Some(v) = self.provenance.as_ref() {
            struct_ser.serialize_field("provenance", v)?;
        }
        if true {
            struct_ser.serialize_field("version", &self.version)?;
        }
        if true {
            struct_ser.serialize_field("schema_version", &self.schema_version)?;
        }
        if let Some(v) = self.tempo_map.as_ref() {
            struct_ser.serialize_field("tempo_map", v)?;
        }
        if let Some(v) = self.time_signature_map.as_ref() {
            struct_ser.serialize_field("time_signature_map", v)?;
        }
        if true {
            struct_ser.serialize_field("sections", &self.sections)?;
        }
        if true {
            struct_ser.serialize_field("markers", &self.markers)?;
        }
        if true {
            struct_ser.serialize_field("tracks", &self.tracks)?;
        }
        if true {
            struct_ser.serialize_field("clips", &self.clips)?;
        }
        if true {
            struct_ser.serialize_field("automation", &self.automation)?;
        }
        if true {
            struct_ser.serialize_field("generators", &self.generators)?;
        }
        if let Some(v) = self.render_target.as_ref() {
            struct_ser.serialize_field("render_target", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for Song {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "id",
            "provenance",
            "version",
            "schema_version",
            "schemaVersion",
            "tempo_map",
            "tempoMap",
            "time_signature_map",
            "timeSignatureMap",
            "sections",
            "markers",
            "tracks",
            "clips",
            "automation",
            "generators",
            "render_target",
            "renderTarget",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Id,
            Provenance,
            Version,
            SchemaVersion,
            TempoMap,
            TimeSignatureMap,
            Sections,
            Markers,
            Tracks,
            Clips,
            Automation,
            Generators,
            RenderTarget,
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
                            "provenance" => Ok(GeneratedField::Provenance),
                            "version" => Ok(GeneratedField::Version),
                            "schemaVersion" | "schema_version" => Ok(GeneratedField::SchemaVersion),
                            "tempoMap" | "tempo_map" => Ok(GeneratedField::TempoMap),
                            "timeSignatureMap" | "time_signature_map" => Ok(GeneratedField::TimeSignatureMap),
                            "sections" => Ok(GeneratedField::Sections),
                            "markers" => Ok(GeneratedField::Markers),
                            "tracks" => Ok(GeneratedField::Tracks),
                            "clips" => Ok(GeneratedField::Clips),
                            "automation" => Ok(GeneratedField::Automation),
                            "generators" => Ok(GeneratedField::Generators),
                            "renderTarget" | "render_target" => Ok(GeneratedField::RenderTarget),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = Song;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct escribass.song.v1.Song")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<Song, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut id__ = None;
                let mut provenance__ = None;
                let mut version__ = None;
                let mut schema_version__ = None;
                let mut tempo_map__ = None;
                let mut time_signature_map__ = None;
                let mut sections__ = None;
                let mut markers__ = None;
                let mut tracks__ = None;
                let mut clips__ = None;
                let mut automation__ = None;
                let mut generators__ = None;
                let mut render_target__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Id => {
                            if id__.is_some() {
                                return Err(serde::de::Error::duplicate_field("id"));
                            }
                            id__ = Some(map_.next_value()?);
                        }
                        GeneratedField::Provenance => {
                            if provenance__.is_some() {
                                return Err(serde::de::Error::duplicate_field("provenance"));
                            }
                            provenance__ = map_.next_value()?;
                        }
                        GeneratedField::Version => {
                            if version__.is_some() {
                                return Err(serde::de::Error::duplicate_field("version"));
                            }
                            version__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::SchemaVersion => {
                            if schema_version__.is_some() {
                                return Err(serde::de::Error::duplicate_field("schemaVersion"));
                            }
                            schema_version__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::TempoMap => {
                            if tempo_map__.is_some() {
                                return Err(serde::de::Error::duplicate_field("tempoMap"));
                            }
                            tempo_map__ = map_.next_value()?;
                        }
                        GeneratedField::TimeSignatureMap => {
                            if time_signature_map__.is_some() {
                                return Err(serde::de::Error::duplicate_field("timeSignatureMap"));
                            }
                            time_signature_map__ = map_.next_value()?;
                        }
                        GeneratedField::Sections => {
                            if sections__.is_some() {
                                return Err(serde::de::Error::duplicate_field("sections"));
                            }
                            sections__ = Some(
                                map_.next_value::<std::collections::BTreeMap<_, _>>()?
                            );
                        }
                        GeneratedField::Markers => {
                            if markers__.is_some() {
                                return Err(serde::de::Error::duplicate_field("markers"));
                            }
                            markers__ = Some(
                                map_.next_value::<std::collections::BTreeMap<_, _>>()?
                            );
                        }
                        GeneratedField::Tracks => {
                            if tracks__.is_some() {
                                return Err(serde::de::Error::duplicate_field("tracks"));
                            }
                            tracks__ = Some(
                                map_.next_value::<std::collections::BTreeMap<_, _>>()?
                            );
                        }
                        GeneratedField::Clips => {
                            if clips__.is_some() {
                                return Err(serde::de::Error::duplicate_field("clips"));
                            }
                            clips__ = Some(
                                map_.next_value::<std::collections::BTreeMap<_, _>>()?
                            );
                        }
                        GeneratedField::Automation => {
                            if automation__.is_some() {
                                return Err(serde::de::Error::duplicate_field("automation"));
                            }
                            automation__ = Some(
                                map_.next_value::<std::collections::BTreeMap<_, _>>()?
                            );
                        }
                        GeneratedField::Generators => {
                            if generators__.is_some() {
                                return Err(serde::de::Error::duplicate_field("generators"));
                            }
                            generators__ = Some(
                                map_.next_value::<std::collections::BTreeMap<_, _>>()?
                            );
                        }
                        GeneratedField::RenderTarget => {
                            if render_target__.is_some() {
                                return Err(serde::de::Error::duplicate_field("renderTarget"));
                            }
                            render_target__ = map_.next_value()?;
                        }
                    }
                }
                Ok(Song {
                    id: id__.unwrap_or_default(),
                    provenance: provenance__,
                    version: version__.unwrap_or_default(),
                    schema_version: schema_version__.unwrap_or_default(),
                    tempo_map: tempo_map__,
                    time_signature_map: time_signature_map__,
                    sections: sections__.unwrap_or_default(),
                    markers: markers__.unwrap_or_default(),
                    tracks: tracks__.unwrap_or_default(),
                    clips: clips__.unwrap_or_default(),
                    automation: automation__.unwrap_or_default(),
                    generators: generators__.unwrap_or_default(),
                    render_target: render_target__,
                })
            }
        }
        deserializer.deserialize_struct("escribass.song.v1.Song", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for SourceRef {
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
        let mut struct_ser = serializer.serialize_struct("escribass.song.v1.SourceRef", len)?;
        if true {
            struct_ser.serialize_field("source_hash", &self.source_hash)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for SourceRef {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "source_hash",
            "sourceHash",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            SourceHash,
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
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = SourceRef;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct escribass.song.v1.SourceRef")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<SourceRef, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut source_hash__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::SourceHash => {
                            if source_hash__.is_some() {
                                return Err(serde::de::Error::duplicate_field("sourceHash"));
                            }
                            source_hash__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(SourceRef {
                    source_hash: source_hash__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("escribass.song.v1.SourceRef", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for TempoEvent {
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
        let mut struct_ser = serializer.serialize_struct("escribass.song.v1.TempoEvent", len)?;
        if true {
            struct_ser.serialize_field("id", &self.id)?;
        }
        if true {
            struct_ser.serialize_field("tick", &self.tick)?;
        }
        if true {
            struct_ser.serialize_field("bpm", &self.bpm)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for TempoEvent {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "id",
            "tick",
            "bpm",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Id,
            Tick,
            Bpm,
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
                            "tick" => Ok(GeneratedField::Tick),
                            "bpm" => Ok(GeneratedField::Bpm),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = TempoEvent;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct escribass.song.v1.TempoEvent")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<TempoEvent, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut id__ = None;
                let mut tick__ = None;
                let mut bpm__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Id => {
                            if id__.is_some() {
                                return Err(serde::de::Error::duplicate_field("id"));
                            }
                            id__ = Some(map_.next_value()?);
                        }
                        GeneratedField::Tick => {
                            if tick__.is_some() {
                                return Err(serde::de::Error::duplicate_field("tick"));
                            }
                            tick__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::Bpm => {
                            if bpm__.is_some() {
                                return Err(serde::de::Error::duplicate_field("bpm"));
                            }
                            bpm__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                    }
                }
                Ok(TempoEvent {
                    id: id__.unwrap_or_default(),
                    tick: tick__.unwrap_or_default(),
                    bpm: bpm__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("escribass.song.v1.TempoEvent", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for TempoMap {
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
        let mut struct_ser = serializer.serialize_struct("escribass.song.v1.TempoMap", len)?;
        if true {
            struct_ser.serialize_field("events", &self.events)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for TempoMap {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "events",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Events,
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
                            "events" => Ok(GeneratedField::Events),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = TempoMap;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct escribass.song.v1.TempoMap")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<TempoMap, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut events__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Events => {
                            if events__.is_some() {
                                return Err(serde::de::Error::duplicate_field("events"));
                            }
                            events__ = Some(
                                map_.next_value::<std::collections::BTreeMap<_, _>>()?
                            );
                        }
                    }
                }
                Ok(TempoMap {
                    events: events__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("escribass.song.v1.TempoMap", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for TimeSignatureEvent {
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
        let mut struct_ser = serializer.serialize_struct("escribass.song.v1.TimeSignatureEvent", len)?;
        if true {
            struct_ser.serialize_field("id", &self.id)?;
        }
        if true {
            struct_ser.serialize_field("tick", &self.tick)?;
        }
        if true {
            struct_ser.serialize_field("numerator", &self.numerator)?;
        }
        if true {
            struct_ser.serialize_field("denominator", &self.denominator)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for TimeSignatureEvent {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "id",
            "tick",
            "numerator",
            "denominator",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Id,
            Tick,
            Numerator,
            Denominator,
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
                            "tick" => Ok(GeneratedField::Tick),
                            "numerator" => Ok(GeneratedField::Numerator),
                            "denominator" => Ok(GeneratedField::Denominator),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = TimeSignatureEvent;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct escribass.song.v1.TimeSignatureEvent")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<TimeSignatureEvent, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut id__ = None;
                let mut tick__ = None;
                let mut numerator__ = None;
                let mut denominator__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Id => {
                            if id__.is_some() {
                                return Err(serde::de::Error::duplicate_field("id"));
                            }
                            id__ = Some(map_.next_value()?);
                        }
                        GeneratedField::Tick => {
                            if tick__.is_some() {
                                return Err(serde::de::Error::duplicate_field("tick"));
                            }
                            tick__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::Numerator => {
                            if numerator__.is_some() {
                                return Err(serde::de::Error::duplicate_field("numerator"));
                            }
                            numerator__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::Denominator => {
                            if denominator__.is_some() {
                                return Err(serde::de::Error::duplicate_field("denominator"));
                            }
                            denominator__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                    }
                }
                Ok(TimeSignatureEvent {
                    id: id__.unwrap_or_default(),
                    tick: tick__.unwrap_or_default(),
                    numerator: numerator__.unwrap_or_default(),
                    denominator: denominator__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("escribass.song.v1.TimeSignatureEvent", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for TimeSignatureMap {
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
        let mut struct_ser = serializer.serialize_struct("escribass.song.v1.TimeSignatureMap", len)?;
        if true {
            struct_ser.serialize_field("events", &self.events)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for TimeSignatureMap {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "events",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Events,
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
                            "events" => Ok(GeneratedField::Events),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = TimeSignatureMap;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct escribass.song.v1.TimeSignatureMap")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<TimeSignatureMap, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut events__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Events => {
                            if events__.is_some() {
                                return Err(serde::de::Error::duplicate_field("events"));
                            }
                            events__ = Some(
                                map_.next_value::<std::collections::BTreeMap<_, _>>()?
                            );
                        }
                    }
                }
                Ok(TimeSignatureMap {
                    events: events__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("escribass.song.v1.TimeSignatureMap", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for Track {
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
        if true {
            len += 1;
        }
        if true {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("escribass.song.v1.Track", len)?;
        if true {
            struct_ser.serialize_field("id", &self.id)?;
        }
        if let Some(v) = self.provenance.as_ref() {
            struct_ser.serialize_field("provenance", v)?;
        }
        if true {
            struct_ser.serialize_field("version", &self.version)?;
        }
        if true {
            struct_ser.serialize_field("name", &self.name)?;
        }
        if true {
            let v = TrackKind::try_from(self.kind)
                .map_err(|_| serde::ser::Error::custom(format!("Invalid variant {}", self.kind)))?;
            struct_ser.serialize_field("kind", &v)?;
        }
        if true {
            struct_ser.serialize_field("index", &self.index)?;
        }
        if let Some(v) = self.instrument.as_ref() {
            struct_ser.serialize_field("instrument", v)?;
        }
        if true {
            struct_ser.serialize_field("fx_chain", &self.fx_chain)?;
        }
        if let Some(v) = self.routing.as_ref() {
            struct_ser.serialize_field("routing", v)?;
        }
        if let Some(v) = self.mix.as_ref() {
            struct_ser.serialize_field("mix", v)?;
        }
        if true {
            struct_ser.serialize_field("allow_overlap", &self.allow_overlap)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for Track {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "id",
            "provenance",
            "version",
            "name",
            "kind",
            "index",
            "instrument",
            "fx_chain",
            "fxChain",
            "routing",
            "mix",
            "allow_overlap",
            "allowOverlap",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Id,
            Provenance,
            Version,
            Name,
            Kind,
            Index,
            Instrument,
            FxChain,
            Routing,
            Mix,
            AllowOverlap,
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
                            "provenance" => Ok(GeneratedField::Provenance),
                            "version" => Ok(GeneratedField::Version),
                            "name" => Ok(GeneratedField::Name),
                            "kind" => Ok(GeneratedField::Kind),
                            "index" => Ok(GeneratedField::Index),
                            "instrument" => Ok(GeneratedField::Instrument),
                            "fxChain" | "fx_chain" => Ok(GeneratedField::FxChain),
                            "routing" => Ok(GeneratedField::Routing),
                            "mix" => Ok(GeneratedField::Mix),
                            "allowOverlap" | "allow_overlap" => Ok(GeneratedField::AllowOverlap),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = Track;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct escribass.song.v1.Track")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<Track, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut id__ = None;
                let mut provenance__ = None;
                let mut version__ = None;
                let mut name__ = None;
                let mut kind__ = None;
                let mut index__ = None;
                let mut instrument__ = None;
                let mut fx_chain__ = None;
                let mut routing__ = None;
                let mut mix__ = None;
                let mut allow_overlap__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Id => {
                            if id__.is_some() {
                                return Err(serde::de::Error::duplicate_field("id"));
                            }
                            id__ = Some(map_.next_value()?);
                        }
                        GeneratedField::Provenance => {
                            if provenance__.is_some() {
                                return Err(serde::de::Error::duplicate_field("provenance"));
                            }
                            provenance__ = map_.next_value()?;
                        }
                        GeneratedField::Version => {
                            if version__.is_some() {
                                return Err(serde::de::Error::duplicate_field("version"));
                            }
                            version__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
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
                            kind__ = Some(map_.next_value::<TrackKind>()? as i32);
                        }
                        GeneratedField::Index => {
                            if index__.is_some() {
                                return Err(serde::de::Error::duplicate_field("index"));
                            }
                            index__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::Instrument => {
                            if instrument__.is_some() {
                                return Err(serde::de::Error::duplicate_field("instrument"));
                            }
                            instrument__ = map_.next_value()?;
                        }
                        GeneratedField::FxChain => {
                            if fx_chain__.is_some() {
                                return Err(serde::de::Error::duplicate_field("fxChain"));
                            }
                            fx_chain__ = Some(
                                map_.next_value::<std::collections::BTreeMap<_, _>>()?
                            );
                        }
                        GeneratedField::Routing => {
                            if routing__.is_some() {
                                return Err(serde::de::Error::duplicate_field("routing"));
                            }
                            routing__ = map_.next_value()?;
                        }
                        GeneratedField::Mix => {
                            if mix__.is_some() {
                                return Err(serde::de::Error::duplicate_field("mix"));
                            }
                            mix__ = map_.next_value()?;
                        }
                        GeneratedField::AllowOverlap => {
                            if allow_overlap__.is_some() {
                                return Err(serde::de::Error::duplicate_field("allowOverlap"));
                            }
                            allow_overlap__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(Track {
                    id: id__.unwrap_or_default(),
                    provenance: provenance__,
                    version: version__.unwrap_or_default(),
                    name: name__.unwrap_or_default(),
                    kind: kind__.unwrap_or_default(),
                    index: index__.unwrap_or_default(),
                    instrument: instrument__,
                    fx_chain: fx_chain__.unwrap_or_default(),
                    routing: routing__,
                    mix: mix__,
                    allow_overlap: allow_overlap__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("escribass.song.v1.Track", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for TrackKind {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let variant = match self {
            Self::Unspecified => "TRACK_KIND_UNSPECIFIED",
            Self::Instrument => "TRACK_KIND_INSTRUMENT",
            Self::Audio => "TRACK_KIND_AUDIO",
            Self::Bus => "TRACK_KIND_BUS",
            Self::Master => "TRACK_KIND_MASTER",
        };
        serializer.serialize_str(variant)
    }
}
impl<'de> serde::Deserialize<'de> for TrackKind {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "TRACK_KIND_UNSPECIFIED",
            "TRACK_KIND_INSTRUMENT",
            "TRACK_KIND_AUDIO",
            "TRACK_KIND_BUS",
            "TRACK_KIND_MASTER",
        ];

        struct GeneratedVisitor;

        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = TrackKind;

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
                    "TRACK_KIND_UNSPECIFIED" => Ok(TrackKind::Unspecified),
                    "TRACK_KIND_INSTRUMENT" => Ok(TrackKind::Instrument),
                    "TRACK_KIND_AUDIO" => Ok(TrackKind::Audio),
                    "TRACK_KIND_BUS" => Ok(TrackKind::Bus),
                    "TRACK_KIND_MASTER" => Ok(TrackKind::Master),
                    _ => Err(serde::de::Error::unknown_variant(value, FIELDS)),
                }
            }
        }
        deserializer.deserialize_any(GeneratedVisitor)
    }
}

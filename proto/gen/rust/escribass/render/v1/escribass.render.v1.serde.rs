// @generated
impl serde::Serialize for PlanAudio {
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
        let mut struct_ser = serializer.serialize_struct("escribass.render.v1.PlanAudio", len)?;
        if let Some(v) = self.clip.as_ref() {
            struct_ser.serialize_field("clip", v)?;
        }
        if true {
            struct_ser.serialize_field("path", &self.path)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for PlanAudio {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "clip",
            "path",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Clip,
            Path,
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
                            "clip" => Ok(GeneratedField::Clip),
                            "path" => Ok(GeneratedField::Path),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = PlanAudio;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct escribass.render.v1.PlanAudio")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<PlanAudio, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut clip__ = None;
                let mut path__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Clip => {
                            if clip__.is_some() {
                                return Err(serde::de::Error::duplicate_field("clip"));
                            }
                            clip__ = map_.next_value()?;
                        }
                        GeneratedField::Path => {
                            if path__.is_some() {
                                return Err(serde::de::Error::duplicate_field("path"));
                            }
                            path__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(PlanAudio {
                    clip: clip__,
                    path: path__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("escribass.render.v1.PlanAudio", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for PlanClip {
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
        if self.content.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("escribass.render.v1.PlanClip", len)?;
        if true {
            struct_ser.serialize_field("start_tick", &self.start_tick)?;
        }
        if true {
            struct_ser.serialize_field("length_ticks", &self.length_ticks)?;
        }
        if let Some(v) = self.content.as_ref() {
            match v {
                plan_clip::Content::Notes(v) => {
                    struct_ser.serialize_field("notes", v)?;
                }
                plan_clip::Content::Audio(v) => {
                    struct_ser.serialize_field("audio", v)?;
                }
            }
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for PlanClip {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "start_tick",
            "startTick",
            "length_ticks",
            "lengthTicks",
            "notes",
            "audio",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            StartTick,
            LengthTicks,
            Notes,
            Audio,
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
                            "lengthTicks" | "length_ticks" => Ok(GeneratedField::LengthTicks),
                            "notes" => Ok(GeneratedField::Notes),
                            "audio" => Ok(GeneratedField::Audio),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = PlanClip;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct escribass.render.v1.PlanClip")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<PlanClip, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut start_tick__ = None;
                let mut length_ticks__ = None;
                let mut content__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
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
                        GeneratedField::Notes => {
                            if content__.is_some() {
                                return Err(serde::de::Error::duplicate_field("notes"));
                            }
                            content__ = map_.next_value::<::std::option::Option<_>>()?.map(plan_clip::Content::Notes)
;
                        }
                        GeneratedField::Audio => {
                            if content__.is_some() {
                                return Err(serde::de::Error::duplicate_field("audio"));
                            }
                            content__ = map_.next_value::<::std::option::Option<_>>()?.map(plan_clip::Content::Audio)
;
                        }
                    }
                }
                Ok(PlanClip {
                    start_tick: start_tick__.unwrap_or_default(),
                    length_ticks: length_ticks__.unwrap_or_default(),
                    content: content__,
                })
            }
        }
        deserializer.deserialize_struct("escribass.render.v1.PlanClip", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for PlanEffect {
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
        let mut struct_ser = serializer.serialize_struct("escribass.render.v1.PlanEffect", len)?;
        if let Some(v) = self.effect.as_ref() {
            struct_ser.serialize_field("effect", v)?;
        }
        if true {
            struct_ser.serialize_field("lanes", &self.lanes)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for PlanEffect {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "effect",
            "lanes",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Effect,
            Lanes,
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
                            "effect" => Ok(GeneratedField::Effect),
                            "lanes" => Ok(GeneratedField::Lanes),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = PlanEffect;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct escribass.render.v1.PlanEffect")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<PlanEffect, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut effect__ = None;
                let mut lanes__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Effect => {
                            if effect__.is_some() {
                                return Err(serde::de::Error::duplicate_field("effect"));
                            }
                            effect__ = map_.next_value()?;
                        }
                        GeneratedField::Lanes => {
                            if lanes__.is_some() {
                                return Err(serde::de::Error::duplicate_field("lanes"));
                            }
                            lanes__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(PlanEffect {
                    effect: effect__,
                    lanes: lanes__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("escribass.render.v1.PlanEffect", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for PlanInstrument {
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
        let mut struct_ser = serializer.serialize_struct("escribass.render.v1.PlanInstrument", len)?;
        if let Some(v) = self.instrument.as_ref() {
            struct_ser.serialize_field("instrument", v)?;
        }
        if true {
            struct_ser.serialize_field("lanes", &self.lanes)?;
        }
        if true {
            struct_ser.serialize_field("sfz_path", &self.sfz_path)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for PlanInstrument {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "instrument",
            "lanes",
            "sfz_path",
            "sfzPath",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Instrument,
            Lanes,
            SfzPath,
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
                            "instrument" => Ok(GeneratedField::Instrument),
                            "lanes" => Ok(GeneratedField::Lanes),
                            "sfzPath" | "sfz_path" => Ok(GeneratedField::SfzPath),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = PlanInstrument;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct escribass.render.v1.PlanInstrument")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<PlanInstrument, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut instrument__ = None;
                let mut lanes__ = None;
                let mut sfz_path__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Instrument => {
                            if instrument__.is_some() {
                                return Err(serde::de::Error::duplicate_field("instrument"));
                            }
                            instrument__ = map_.next_value()?;
                        }
                        GeneratedField::Lanes => {
                            if lanes__.is_some() {
                                return Err(serde::de::Error::duplicate_field("lanes"));
                            }
                            lanes__ = Some(map_.next_value()?);
                        }
                        GeneratedField::SfzPath => {
                            if sfz_path__.is_some() {
                                return Err(serde::de::Error::duplicate_field("sfzPath"));
                            }
                            sfz_path__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(PlanInstrument {
                    instrument: instrument__,
                    lanes: lanes__.unwrap_or_default(),
                    sfz_path: sfz_path__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("escribass.render.v1.PlanInstrument", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for PlanLane {
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
        let mut struct_ser = serializer.serialize_struct("escribass.render.v1.PlanLane", len)?;
        if true {
            struct_ser.serialize_field("param", &self.param)?;
        }
        if true {
            struct_ser.serialize_field("points", &self.points)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for PlanLane {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "param",
            "points",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Param,
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
                            "param" => Ok(GeneratedField::Param),
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
            type Value = PlanLane;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct escribass.render.v1.PlanLane")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<PlanLane, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut param__ = None;
                let mut points__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Param => {
                            if param__.is_some() {
                                return Err(serde::de::Error::duplicate_field("param"));
                            }
                            param__ = Some(map_.next_value()?);
                        }
                        GeneratedField::Points => {
                            if points__.is_some() {
                                return Err(serde::de::Error::duplicate_field("points"));
                            }
                            points__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(PlanLane {
                    param: param__.unwrap_or_default(),
                    points: points__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("escribass.render.v1.PlanLane", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for PlanNotes {
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
        let mut struct_ser = serializer.serialize_struct("escribass.render.v1.PlanNotes", len)?;
        if true {
            struct_ser.serialize_field("notes", &self.notes)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for PlanNotes {
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
            type Value = PlanNotes;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct escribass.render.v1.PlanNotes")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<PlanNotes, V::Error>
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
                Ok(PlanNotes {
                    notes: notes__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("escribass.render.v1.PlanNotes", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for PlanTrack {
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
        let mut struct_ser = serializer.serialize_struct("escribass.render.v1.PlanTrack", len)?;
        if let Some(v) = self.instrument.as_ref() {
            struct_ser.serialize_field("instrument", v)?;
        }
        if true {
            struct_ser.serialize_field("effects", &self.effects)?;
        }
        if let Some(v) = self.mix.as_ref() {
            struct_ser.serialize_field("mix", v)?;
        }
        if true {
            struct_ser.serialize_field("clips", &self.clips)?;
        }
        if true {
            struct_ser.serialize_field("mix_lanes", &self.mix_lanes)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for PlanTrack {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "instrument",
            "effects",
            "mix",
            "clips",
            "mix_lanes",
            "mixLanes",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Instrument,
            Effects,
            Mix,
            Clips,
            MixLanes,
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
                            "instrument" => Ok(GeneratedField::Instrument),
                            "effects" => Ok(GeneratedField::Effects),
                            "mix" => Ok(GeneratedField::Mix),
                            "clips" => Ok(GeneratedField::Clips),
                            "mixLanes" | "mix_lanes" => Ok(GeneratedField::MixLanes),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = PlanTrack;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct escribass.render.v1.PlanTrack")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<PlanTrack, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut instrument__ = None;
                let mut effects__ = None;
                let mut mix__ = None;
                let mut clips__ = None;
                let mut mix_lanes__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Instrument => {
                            if instrument__.is_some() {
                                return Err(serde::de::Error::duplicate_field("instrument"));
                            }
                            instrument__ = map_.next_value()?;
                        }
                        GeneratedField::Effects => {
                            if effects__.is_some() {
                                return Err(serde::de::Error::duplicate_field("effects"));
                            }
                            effects__ = Some(map_.next_value()?);
                        }
                        GeneratedField::Mix => {
                            if mix__.is_some() {
                                return Err(serde::de::Error::duplicate_field("mix"));
                            }
                            mix__ = map_.next_value()?;
                        }
                        GeneratedField::Clips => {
                            if clips__.is_some() {
                                return Err(serde::de::Error::duplicate_field("clips"));
                            }
                            clips__ = Some(map_.next_value()?);
                        }
                        GeneratedField::MixLanes => {
                            if mix_lanes__.is_some() {
                                return Err(serde::de::Error::duplicate_field("mixLanes"));
                            }
                            mix_lanes__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(PlanTrack {
                    instrument: instrument__,
                    effects: effects__.unwrap_or_default(),
                    mix: mix__,
                    clips: clips__.unwrap_or_default(),
                    mix_lanes: mix_lanes__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("escribass.render.v1.PlanTrack", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for PreviewCommand {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.command.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("escribass.render.v1.PreviewCommand", len)?;
        if let Some(v) = self.command.as_ref() {
            match v {
                preview_command::Command::Play(v) => {
                    struct_ser.serialize_field("play", v)?;
                }
                preview_command::Command::Seek(v) => {
                    struct_ser.serialize_field("seek", v)?;
                }
                preview_command::Command::Loop(v) => {
                    struct_ser.serialize_field("loop", v)?;
                }
                preview_command::Command::Stop(v) => {
                    struct_ser.serialize_field("stop", v)?;
                }
            }
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for PreviewCommand {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "play",
            "seek",
            "loop",
            "stop",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
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
            type Value = PreviewCommand;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct escribass.render.v1.PreviewCommand")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<PreviewCommand, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut command__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Play => {
                            if command__.is_some() {
                                return Err(serde::de::Error::duplicate_field("play"));
                            }
                            command__ = map_.next_value::<::std::option::Option<_>>()?.map(preview_command::Command::Play)
;
                        }
                        GeneratedField::Seek => {
                            if command__.is_some() {
                                return Err(serde::de::Error::duplicate_field("seek"));
                            }
                            command__ = map_.next_value::<::std::option::Option<_>>()?.map(preview_command::Command::Seek)
;
                        }
                        GeneratedField::Loop => {
                            if command__.is_some() {
                                return Err(serde::de::Error::duplicate_field("loop"));
                            }
                            command__ = map_.next_value::<::std::option::Option<_>>()?.map(preview_command::Command::Loop)
;
                        }
                        GeneratedField::Stop => {
                            if command__.is_some() {
                                return Err(serde::de::Error::duplicate_field("stop"));
                            }
                            command__ = map_.next_value::<::std::option::Option<_>>()?.map(preview_command::Command::Stop)
;
                        }
                    }
                }
                Ok(PreviewCommand {
                    command: command__,
                })
            }
        }
        deserializer.deserialize_struct("escribass.render.v1.PreviewCommand", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for PreviewEvent {
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
        let mut struct_ser = serializer.serialize_struct("escribass.render.v1.PreviewEvent", len)?;
        if true {
            struct_ser.serialize_field("tick", &self.tick)?;
        }
        if true {
            let v = PreviewState::try_from(self.state)
                .map_err(|_| serde::ser::Error::custom(format!("Invalid variant {}", self.state)))?;
            struct_ser.serialize_field("state", &v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for PreviewEvent {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "tick",
            "state",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Tick,
            State,
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
                            "tick" => Ok(GeneratedField::Tick),
                            "state" => Ok(GeneratedField::State),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = PreviewEvent;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct escribass.render.v1.PreviewEvent")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<PreviewEvent, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut tick__ = None;
                let mut state__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Tick => {
                            if tick__.is_some() {
                                return Err(serde::de::Error::duplicate_field("tick"));
                            }
                            tick__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::State => {
                            if state__.is_some() {
                                return Err(serde::de::Error::duplicate_field("state"));
                            }
                            state__ = Some(map_.next_value::<PreviewState>()? as i32);
                        }
                    }
                }
                Ok(PreviewEvent {
                    tick: tick__.unwrap_or_default(),
                    state: state__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("escribass.render.v1.PreviewEvent", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for PreviewLoop {
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
        let mut struct_ser = serializer.serialize_struct("escribass.render.v1.PreviewLoop", len)?;
        if true {
            struct_ser.serialize_field("start_tick", &self.start_tick)?;
        }
        if true {
            struct_ser.serialize_field("end_tick", &self.end_tick)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for PreviewLoop {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "start_tick",
            "startTick",
            "end_tick",
            "endTick",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
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
            type Value = PreviewLoop;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct escribass.render.v1.PreviewLoop")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<PreviewLoop, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut start_tick__ = None;
                let mut end_tick__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
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
                Ok(PreviewLoop {
                    start_tick: start_tick__.unwrap_or_default(),
                    end_tick: end_tick__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("escribass.render.v1.PreviewLoop", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for PreviewPlay {
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
        let mut struct_ser = serializer.serialize_struct("escribass.render.v1.PreviewPlay", len)?;
        if let Some(v) = self.plan.as_ref() {
            struct_ser.serialize_field("plan", v)?;
        }
        if true {
            struct_ser.serialize_field("start_tick", &self.start_tick)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for PreviewPlay {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "plan",
            "start_tick",
            "startTick",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Plan,
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
                            "plan" => Ok(GeneratedField::Plan),
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
            type Value = PreviewPlay;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct escribass.render.v1.PreviewPlay")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<PreviewPlay, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut plan__ = None;
                let mut start_tick__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Plan => {
                            if plan__.is_some() {
                                return Err(serde::de::Error::duplicate_field("plan"));
                            }
                            plan__ = map_.next_value()?;
                        }
                        GeneratedField::StartTick => {
                            if start_tick__.is_some() {
                                return Err(serde::de::Error::duplicate_field("startTick"));
                            }
                            start_tick__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                    }
                }
                Ok(PreviewPlay {
                    plan: plan__,
                    start_tick: start_tick__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("escribass.render.v1.PreviewPlay", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for PreviewSeek {
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
        let mut struct_ser = serializer.serialize_struct("escribass.render.v1.PreviewSeek", len)?;
        if true {
            struct_ser.serialize_field("tick", &self.tick)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for PreviewSeek {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "tick",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
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
            type Value = PreviewSeek;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct escribass.render.v1.PreviewSeek")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<PreviewSeek, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut tick__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
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
                Ok(PreviewSeek {
                    tick: tick__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("escribass.render.v1.PreviewSeek", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for PreviewState {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let variant = match self {
            Self::Unspecified => "PREVIEW_STATE_UNSPECIFIED",
            Self::Playing => "PREVIEW_STATE_PLAYING",
            Self::Stopped => "PREVIEW_STATE_STOPPED",
        };
        serializer.serialize_str(variant)
    }
}
impl<'de> serde::Deserialize<'de> for PreviewState {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "PREVIEW_STATE_UNSPECIFIED",
            "PREVIEW_STATE_PLAYING",
            "PREVIEW_STATE_STOPPED",
        ];

        struct GeneratedVisitor;

        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = PreviewState;

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
                    "PREVIEW_STATE_UNSPECIFIED" => Ok(PreviewState::Unspecified),
                    "PREVIEW_STATE_PLAYING" => Ok(PreviewState::Playing),
                    "PREVIEW_STATE_STOPPED" => Ok(PreviewState::Stopped),
                    _ => Err(serde::de::Error::unknown_variant(value, FIELDS)),
                }
            }
        }
        deserializer.deserialize_any(GeneratedVisitor)
    }
}
impl serde::Serialize for PreviewStop {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let len = 0;
        let struct_ser = serializer.serialize_struct("escribass.render.v1.PreviewStop", len)?;
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for PreviewStop {
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
            type Value = PreviewStop;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct escribass.render.v1.PreviewStop")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<PreviewStop, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                while map_.next_key::<GeneratedField>()?.is_some() {
                    let _ = map_.next_value::<serde::de::IgnoredAny>()?;
                }
                Ok(PreviewStop {
                })
            }
        }
        deserializer.deserialize_struct("escribass.render.v1.PreviewStop", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for RenderPlan {
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
        let mut struct_ser = serializer.serialize_struct("escribass.render.v1.RenderPlan", len)?;
        if true {
            struct_ser.serialize_field("tracks", &self.tracks)?;
        }
        if let Some(v) = self.master.as_ref() {
            struct_ser.serialize_field("master", v)?;
        }
        if true {
            struct_ser.serialize_field("tempo", &self.tempo)?;
        }
        if let Some(v) = self.target.as_ref() {
            struct_ser.serialize_field("target", v)?;
        }
        if true {
            struct_ser.serialize_field("length_ticks", &self.length_ticks)?;
        }
        if true {
            struct_ser.serialize_field("output_path", &self.output_path)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for RenderPlan {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "tracks",
            "master",
            "tempo",
            "target",
            "length_ticks",
            "lengthTicks",
            "output_path",
            "outputPath",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Tracks,
            Master,
            Tempo,
            Target,
            LengthTicks,
            OutputPath,
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
                            "tracks" => Ok(GeneratedField::Tracks),
                            "master" => Ok(GeneratedField::Master),
                            "tempo" => Ok(GeneratedField::Tempo),
                            "target" => Ok(GeneratedField::Target),
                            "lengthTicks" | "length_ticks" => Ok(GeneratedField::LengthTicks),
                            "outputPath" | "output_path" => Ok(GeneratedField::OutputPath),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = RenderPlan;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct escribass.render.v1.RenderPlan")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<RenderPlan, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut tracks__ = None;
                let mut master__ = None;
                let mut tempo__ = None;
                let mut target__ = None;
                let mut length_ticks__ = None;
                let mut output_path__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Tracks => {
                            if tracks__.is_some() {
                                return Err(serde::de::Error::duplicate_field("tracks"));
                            }
                            tracks__ = Some(map_.next_value()?);
                        }
                        GeneratedField::Master => {
                            if master__.is_some() {
                                return Err(serde::de::Error::duplicate_field("master"));
                            }
                            master__ = map_.next_value()?;
                        }
                        GeneratedField::Tempo => {
                            if tempo__.is_some() {
                                return Err(serde::de::Error::duplicate_field("tempo"));
                            }
                            tempo__ = Some(map_.next_value()?);
                        }
                        GeneratedField::Target => {
                            if target__.is_some() {
                                return Err(serde::de::Error::duplicate_field("target"));
                            }
                            target__ = map_.next_value()?;
                        }
                        GeneratedField::LengthTicks => {
                            if length_ticks__.is_some() {
                                return Err(serde::de::Error::duplicate_field("lengthTicks"));
                            }
                            length_ticks__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::OutputPath => {
                            if output_path__.is_some() {
                                return Err(serde::de::Error::duplicate_field("outputPath"));
                            }
                            output_path__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(RenderPlan {
                    tracks: tracks__.unwrap_or_default(),
                    master: master__,
                    tempo: tempo__.unwrap_or_default(),
                    target: target__,
                    length_ticks: length_ticks__.unwrap_or_default(),
                    output_path: output_path__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("escribass.render.v1.RenderPlan", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for RenderResult {
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
        let mut struct_ser = serializer.serialize_struct("escribass.render.v1.RenderResult", len)?;
        if true {
            struct_ser.serialize_field("pcm_sha256", &self.pcm_sha256)?;
        }
        if true {
            struct_ser.serialize_field("commits", &self.commits)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for RenderResult {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "pcm_sha256",
            "pcmSha256",
            "commits",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            PcmSha256,
            Commits,
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
                            "pcmSha256" | "pcm_sha256" => Ok(GeneratedField::PcmSha256),
                            "commits" => Ok(GeneratedField::Commits),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = RenderResult;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct escribass.render.v1.RenderResult")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<RenderResult, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut pcm_sha256__ = None;
                let mut commits__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::PcmSha256 => {
                            if pcm_sha256__.is_some() {
                                return Err(serde::de::Error::duplicate_field("pcmSha256"));
                            }
                            pcm_sha256__ = Some(map_.next_value()?);
                        }
                        GeneratedField::Commits => {
                            if commits__.is_some() {
                                return Err(serde::de::Error::duplicate_field("commits"));
                            }
                            commits__ = Some(
                                map_.next_value::<std::collections::BTreeMap<_, _>>()?
                            );
                        }
                    }
                }
                Ok(RenderResult {
                    pcm_sha256: pcm_sha256__.unwrap_or_default(),
                    commits: commits__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("escribass.render.v1.RenderResult", FIELDS, GeneratedVisitor)
    }
}

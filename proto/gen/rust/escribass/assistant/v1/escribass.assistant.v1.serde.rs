// @generated
impl serde::Serialize for AssistantCommand {
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
        let mut struct_ser = serializer.serialize_struct("escribass.assistant.v1.AssistantCommand", len)?;
        if let Some(v) = self.command.as_ref() {
            match v {
                assistant_command::Command::Prompt(v) => {
                    struct_ser.serialize_field("prompt", v)?;
                }
                assistant_command::Command::Result(v) => {
                    struct_ser.serialize_field("result", v)?;
                }
            }
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for AssistantCommand {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "prompt",
            "result",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Prompt,
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
                            "prompt" => Ok(GeneratedField::Prompt),
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
            type Value = AssistantCommand;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct escribass.assistant.v1.AssistantCommand")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<AssistantCommand, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut command__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Prompt => {
                            if command__.is_some() {
                                return Err(serde::de::Error::duplicate_field("prompt"));
                            }
                            command__ = map_.next_value::<::std::option::Option<_>>()?.map(assistant_command::Command::Prompt)
;
                        }
                        GeneratedField::Result => {
                            if command__.is_some() {
                                return Err(serde::de::Error::duplicate_field("result"));
                            }
                            command__ = map_.next_value::<::std::option::Option<_>>()?.map(assistant_command::Command::Result)
;
                        }
                    }
                }
                Ok(AssistantCommand {
                    command: command__,
                })
            }
        }
        deserializer.deserialize_struct("escribass.assistant.v1.AssistantCommand", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for AssistantEvent {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.event.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("escribass.assistant.v1.AssistantEvent", len)?;
        if let Some(v) = self.event.as_ref() {
            match v {
                assistant_event::Event::Text(v) => {
                    struct_ser.serialize_field("text", v)?;
                }
                assistant_event::Event::Call(v) => {
                    struct_ser.serialize_field("call", v)?;
                }
                assistant_event::Event::Done(v) => {
                    struct_ser.serialize_field("done", v)?;
                }
            }
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for AssistantEvent {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "text",
            "call",
            "done",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Text,
            Call,
            Done,
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
                            "text" => Ok(GeneratedField::Text),
                            "call" => Ok(GeneratedField::Call),
                            "done" => Ok(GeneratedField::Done),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = AssistantEvent;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct escribass.assistant.v1.AssistantEvent")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<AssistantEvent, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut event__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Text => {
                            if event__.is_some() {
                                return Err(serde::de::Error::duplicate_field("text"));
                            }
                            event__ = map_.next_value::<::std::option::Option<_>>()?.map(assistant_event::Event::Text)
;
                        }
                        GeneratedField::Call => {
                            if event__.is_some() {
                                return Err(serde::de::Error::duplicate_field("call"));
                            }
                            event__ = map_.next_value::<::std::option::Option<_>>()?.map(assistant_event::Event::Call)
;
                        }
                        GeneratedField::Done => {
                            if event__.is_some() {
                                return Err(serde::de::Error::duplicate_field("done"));
                            }
                            event__ = map_.next_value::<::std::option::Option<_>>()?.map(assistant_event::Event::Done)
;
                        }
                    }
                }
                Ok(AssistantEvent {
                    event: event__,
                })
            }
        }
        deserializer.deserialize_struct("escribass.assistant.v1.AssistantEvent", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for CallResult {
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
        let mut struct_ser = serializer.serialize_struct("escribass.assistant.v1.CallResult", len)?;
        if true {
            struct_ser.serialize_field("call_id", &self.call_id)?;
        }
        if let Some(v) = self.result.as_ref() {
            struct_ser.serialize_field("result", v)?;
        }
        if let Some(v) = self.song.as_ref() {
            struct_ser.serialize_field("song", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for CallResult {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "call_id",
            "callId",
            "result",
            "song",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            CallId,
            Result,
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
                            "callId" | "call_id" => Ok(GeneratedField::CallId),
                            "result" => Ok(GeneratedField::Result),
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
            type Value = CallResult;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct escribass.assistant.v1.CallResult")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<CallResult, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut call_id__ = None;
                let mut result__ = None;
                let mut song__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::CallId => {
                            if call_id__.is_some() {
                                return Err(serde::de::Error::duplicate_field("callId"));
                            }
                            call_id__ = Some(map_.next_value()?);
                        }
                        GeneratedField::Result => {
                            if result__.is_some() {
                                return Err(serde::de::Error::duplicate_field("result"));
                            }
                            result__ = map_.next_value()?;
                        }
                        GeneratedField::Song => {
                            if song__.is_some() {
                                return Err(serde::de::Error::duplicate_field("song"));
                            }
                            song__ = map_.next_value()?;
                        }
                    }
                }
                Ok(CallResult {
                    call_id: call_id__.unwrap_or_default(),
                    result: result__,
                    song: song__,
                })
            }
        }
        deserializer.deserialize_struct("escribass.assistant.v1.CallResult", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for CompletedCall {
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
        let mut struct_ser = serializer.serialize_struct("escribass.assistant.v1.CompletedCall", len)?;
        if let Some(v) = self.call.as_ref() {
            struct_ser.serialize_field("call", v)?;
        }
        if let Some(v) = self.result.as_ref() {
            struct_ser.serialize_field("result", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for CompletedCall {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "call",
            "result",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Call,
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
                            "call" => Ok(GeneratedField::Call),
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
            type Value = CompletedCall;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct escribass.assistant.v1.CompletedCall")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<CompletedCall, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut call__ = None;
                let mut result__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Call => {
                            if call__.is_some() {
                                return Err(serde::de::Error::duplicate_field("call"));
                            }
                            call__ = map_.next_value()?;
                        }
                        GeneratedField::Result => {
                            if result__.is_some() {
                                return Err(serde::de::Error::duplicate_field("result"));
                            }
                            result__ = map_.next_value()?;
                        }
                    }
                }
                Ok(CompletedCall {
                    call: call__,
                    result: result__,
                })
            }
        }
        deserializer.deserialize_struct("escribass.assistant.v1.CompletedCall", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for Done {
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
        let mut struct_ser = serializer.serialize_struct("escribass.assistant.v1.Done", len)?;
        if true {
            struct_ser.serialize_field("text", &self.text)?;
        }
        if true {
            struct_ser.serialize_field("model_id", &self.model_id)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for Done {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "text",
            "model_id",
            "modelId",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Text,
            ModelId,
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
                            "text" => Ok(GeneratedField::Text),
                            "modelId" | "model_id" => Ok(GeneratedField::ModelId),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = Done;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct escribass.assistant.v1.Done")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<Done, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut text__ = None;
                let mut model_id__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Text => {
                            if text__.is_some() {
                                return Err(serde::de::Error::duplicate_field("text"));
                            }
                            text__ = Some(map_.next_value()?);
                        }
                        GeneratedField::ModelId => {
                            if model_id__.is_some() {
                                return Err(serde::de::Error::duplicate_field("modelId"));
                            }
                            model_id__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(Done {
                    text: text__.unwrap_or_default(),
                    model_id: model_id__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("escribass.assistant.v1.Done", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for Prompt {
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
        let mut struct_ser = serializer.serialize_struct("escribass.assistant.v1.Prompt", len)?;
        if true {
            struct_ser.serialize_field("text", &self.text)?;
        }
        if let Some(v) = self.song.as_ref() {
            struct_ser.serialize_field("song", v)?;
        }
        if true {
            struct_ser.serialize_field("tools", &self.tools)?;
        }
        if true {
            struct_ser.serialize_field("model_id", &self.model_id)?;
        }
        if true {
            struct_ser.serialize_field("conversation", &self.conversation)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for Prompt {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "text",
            "song",
            "tools",
            "model_id",
            "modelId",
            "conversation",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Text,
            Song,
            Tools,
            ModelId,
            Conversation,
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
                            "text" => Ok(GeneratedField::Text),
                            "song" => Ok(GeneratedField::Song),
                            "tools" => Ok(GeneratedField::Tools),
                            "modelId" | "model_id" => Ok(GeneratedField::ModelId),
                            "conversation" => Ok(GeneratedField::Conversation),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = Prompt;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct escribass.assistant.v1.Prompt")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<Prompt, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut text__ = None;
                let mut song__ = None;
                let mut tools__ = None;
                let mut model_id__ = None;
                let mut conversation__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Text => {
                            if text__.is_some() {
                                return Err(serde::de::Error::duplicate_field("text"));
                            }
                            text__ = Some(map_.next_value()?);
                        }
                        GeneratedField::Song => {
                            if song__.is_some() {
                                return Err(serde::de::Error::duplicate_field("song"));
                            }
                            song__ = map_.next_value()?;
                        }
                        GeneratedField::Tools => {
                            if tools__.is_some() {
                                return Err(serde::de::Error::duplicate_field("tools"));
                            }
                            tools__ = Some(map_.next_value()?);
                        }
                        GeneratedField::ModelId => {
                            if model_id__.is_some() {
                                return Err(serde::de::Error::duplicate_field("modelId"));
                            }
                            model_id__ = Some(map_.next_value()?);
                        }
                        GeneratedField::Conversation => {
                            if conversation__.is_some() {
                                return Err(serde::de::Error::duplicate_field("conversation"));
                            }
                            conversation__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(Prompt {
                    text: text__.unwrap_or_default(),
                    song: song__,
                    tools: tools__.unwrap_or_default(),
                    model_id: model_id__.unwrap_or_default(),
                    conversation: conversation__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("escribass.assistant.v1.Prompt", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for ReplyText {
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
        let mut struct_ser = serializer.serialize_struct("escribass.assistant.v1.ReplyText", len)?;
        if true {
            struct_ser.serialize_field("text", &self.text)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for ReplyText {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "text",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Text,
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
                            "text" => Ok(GeneratedField::Text),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = ReplyText;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct escribass.assistant.v1.ReplyText")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<ReplyText, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut text__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Text => {
                            if text__.is_some() {
                                return Err(serde::de::Error::duplicate_field("text"));
                            }
                            text__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(ReplyText {
                    text: text__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("escribass.assistant.v1.ReplyText", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for ToolCall {
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
        let mut struct_ser = serializer.serialize_struct("escribass.assistant.v1.ToolCall", len)?;
        if true {
            struct_ser.serialize_field("call_id", &self.call_id)?;
        }
        if true {
            struct_ser.serialize_field("name", &self.name)?;
        }
        if true {
            struct_ser.serialize_field("args_json", &self.args_json)?;
        }
        if true {
            struct_ser.serialize_field("model_id", &self.model_id)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for ToolCall {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "call_id",
            "callId",
            "name",
            "args_json",
            "argsJson",
            "model_id",
            "modelId",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            CallId,
            Name,
            ArgsJson,
            ModelId,
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
                            "callId" | "call_id" => Ok(GeneratedField::CallId),
                            "name" => Ok(GeneratedField::Name),
                            "argsJson" | "args_json" => Ok(GeneratedField::ArgsJson),
                            "modelId" | "model_id" => Ok(GeneratedField::ModelId),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = ToolCall;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct escribass.assistant.v1.ToolCall")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<ToolCall, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut call_id__ = None;
                let mut name__ = None;
                let mut args_json__ = None;
                let mut model_id__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::CallId => {
                            if call_id__.is_some() {
                                return Err(serde::de::Error::duplicate_field("callId"));
                            }
                            call_id__ = Some(map_.next_value()?);
                        }
                        GeneratedField::Name => {
                            if name__.is_some() {
                                return Err(serde::de::Error::duplicate_field("name"));
                            }
                            name__ = Some(map_.next_value()?);
                        }
                        GeneratedField::ArgsJson => {
                            if args_json__.is_some() {
                                return Err(serde::de::Error::duplicate_field("argsJson"));
                            }
                            args_json__ = Some(map_.next_value()?);
                        }
                        GeneratedField::ModelId => {
                            if model_id__.is_some() {
                                return Err(serde::de::Error::duplicate_field("modelId"));
                            }
                            model_id__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(ToolCall {
                    call_id: call_id__.unwrap_or_default(),
                    name: name__.unwrap_or_default(),
                    args_json: args_json__.unwrap_or_default(),
                    model_id: model_id__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("escribass.assistant.v1.ToolCall", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for ToolSchema {
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
        let mut struct_ser = serializer.serialize_struct("escribass.assistant.v1.ToolSchema", len)?;
        if true {
            struct_ser.serialize_field("name", &self.name)?;
        }
        if true {
            struct_ser.serialize_field("description", &self.description)?;
        }
        if true {
            struct_ser.serialize_field("input_schema", &self.input_schema)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for ToolSchema {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "name",
            "description",
            "input_schema",
            "inputSchema",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Name,
            Description,
            InputSchema,
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
                            "description" => Ok(GeneratedField::Description),
                            "inputSchema" | "input_schema" => Ok(GeneratedField::InputSchema),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = ToolSchema;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct escribass.assistant.v1.ToolSchema")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<ToolSchema, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut name__ = None;
                let mut description__ = None;
                let mut input_schema__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Name => {
                            if name__.is_some() {
                                return Err(serde::de::Error::duplicate_field("name"));
                            }
                            name__ = Some(map_.next_value()?);
                        }
                        GeneratedField::Description => {
                            if description__.is_some() {
                                return Err(serde::de::Error::duplicate_field("description"));
                            }
                            description__ = Some(map_.next_value()?);
                        }
                        GeneratedField::InputSchema => {
                            if input_schema__.is_some() {
                                return Err(serde::de::Error::duplicate_field("inputSchema"));
                            }
                            input_schema__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(ToolSchema {
                    name: name__.unwrap_or_default(),
                    description: description__.unwrap_or_default(),
                    input_schema: input_schema__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("escribass.assistant.v1.ToolSchema", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for Turn {
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
        let mut struct_ser = serializer.serialize_struct("escribass.assistant.v1.Turn", len)?;
        if true {
            struct_ser.serialize_field("prompt", &self.prompt)?;
        }
        if true {
            struct_ser.serialize_field("calls", &self.calls)?;
        }
        if true {
            struct_ser.serialize_field("reply", &self.reply)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for Turn {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "prompt",
            "calls",
            "reply",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Prompt,
            Calls,
            Reply,
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
                            "prompt" => Ok(GeneratedField::Prompt),
                            "calls" => Ok(GeneratedField::Calls),
                            "reply" => Ok(GeneratedField::Reply),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = Turn;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct escribass.assistant.v1.Turn")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<Turn, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut prompt__ = None;
                let mut calls__ = None;
                let mut reply__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Prompt => {
                            if prompt__.is_some() {
                                return Err(serde::de::Error::duplicate_field("prompt"));
                            }
                            prompt__ = Some(map_.next_value()?);
                        }
                        GeneratedField::Calls => {
                            if calls__.is_some() {
                                return Err(serde::de::Error::duplicate_field("calls"));
                            }
                            calls__ = Some(map_.next_value()?);
                        }
                        GeneratedField::Reply => {
                            if reply__.is_some() {
                                return Err(serde::de::Error::duplicate_field("reply"));
                            }
                            reply__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(Turn {
                    prompt: prompt__.unwrap_or_default(),
                    calls: calls__.unwrap_or_default(),
                    reply: reply__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("escribass.assistant.v1.Turn", FIELDS, GeneratedVisitor)
    }
}

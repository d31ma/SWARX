use crate::models::NativeHookToolInput;
use crate::utils::ActionPreviewUtils;
use serde::de::{IgnoredAny, MapAccess, SeqAccess, Visitor};

pub struct ClaudeHookToolInputReader {}

impl ClaudeHookToolInputReader {
    const EXPECTED: &'static str = "a hook tool input value";
    const COMMAND_KEY: &'static str = "command";
    const FILE_KEY: &'static str = "file_path";
    const NOTEBOOK_KEY: &'static str = "notebook_path";
    const BACKGROUND_KEY: &'static str = "run_in_background";
    const DESCRIPTION_KEY: &'static str = "description";
    const WORD_SEPARATOR: &'static str = " ";

    pub fn text(value: serde_json::Value) -> Option<String> {
        value.as_str().map(str::to_owned)
    }

    fn read_fields<'de, Fields: MapAccess<'de>>(
        mut fields: Fields,
    ) -> Result<NativeHookToolInput, Fields::Error> {
        let mut input: NativeHookToolInput = NativeHookToolInput::default();
        while let Some(key) = fields.next_key::<String>()? {
            Self::read_value(&mut input, &key, &mut fields)?;
        }
        Ok(input)
    }

    fn read_value<'de, Fields: MapAccess<'de>>(
        input: &mut NativeHookToolInput,
        key: &str,
        fields: &mut Fields,
    ) -> Result<(), Fields::Error> {
        match key {
            Self::COMMAND_KEY => input.command = Self::command(fields.next_value()?),
            Self::FILE_KEY | Self::NOTEBOOK_KEY => input.file = Self::file(fields.next_value()?),
            Self::BACKGROUND_KEY => {
                input.background = fields.next_value::<serde_json::Value>()?.as_bool()
            }
            Self::DESCRIPTION_KEY => input.description = Self::description(fields.next_value()?),
            _ => fields.next_value::<IgnoredAny>().map(drop)?,
        }
        Ok(())
    }

    fn command(value: serde_json::Value) -> Option<String> {
        Self::command_text(value)
            .as_deref()
            .and_then(ActionPreviewUtils::command)
    }

    fn file(value: serde_json::Value) -> Option<String> {
        Self::text(value)
            .as_deref()
            .and_then(ActionPreviewUtils::file)
    }

    fn description(value: serde_json::Value) -> Option<String> {
        Self::text(value)
            .as_deref()
            .and_then(ActionPreviewUtils::description)
    }

    fn command_text(value: serde_json::Value) -> Option<String> {
        match value {
            serde_json::Value::String(text) => Some(text),
            serde_json::Value::Array(words) => Self::joined(&words),
            _ => None,
        }
    }

    fn joined(words: &[serde_json::Value]) -> Option<String> {
        words
            .iter()
            .map(serde_json::Value::as_str)
            .collect::<Option<Vec<&str>>>()
            .map(|parts: Vec<&str>| -> String { parts.join(Self::WORD_SEPARATOR) })
    }

    fn skipped<Failure>(_value: IgnoredAny) -> Result<NativeHookToolInput, Failure> {
        Ok(NativeHookToolInput::default())
    }
}

impl<'de> serde::de::DeserializeSeed<'de> for ClaudeHookToolInputReader {
    type Value = NativeHookToolInput;

    fn deserialize<Input: serde::Deserializer<'de>>(
        self,
        input: Input,
    ) -> Result<NativeHookToolInput, Input::Error> {
        input.deserialize_any(self)
    }
}

impl<'de> Visitor<'de> for ClaudeHookToolInputReader {
    type Value = NativeHookToolInput;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str(Self::EXPECTED)
    }

    fn visit_map<Fields: MapAccess<'de>>(
        self,
        fields: Fields,
    ) -> Result<NativeHookToolInput, Fields::Error> {
        Self::read_fields(fields)
    }

    fn visit_seq<Items: SeqAccess<'de>>(
        self,
        items: Items,
    ) -> Result<NativeHookToolInput, Items::Error> {
        IgnoredAny.visit_seq(items).and_then(Self::skipped)
    }

    fn visit_str<Failure: serde::de::Error>(
        self,
        _value: &str,
    ) -> Result<NativeHookToolInput, Failure> {
        Self::skipped(IgnoredAny)
    }

    fn visit_bool<Failure: serde::de::Error>(
        self,
        _value: bool,
    ) -> Result<NativeHookToolInput, Failure> {
        Self::skipped(IgnoredAny)
    }

    fn visit_i64<Failure: serde::de::Error>(
        self,
        _value: i64,
    ) -> Result<NativeHookToolInput, Failure> {
        Self::skipped(IgnoredAny)
    }

    fn visit_u64<Failure: serde::de::Error>(
        self,
        _value: u64,
    ) -> Result<NativeHookToolInput, Failure> {
        Self::skipped(IgnoredAny)
    }

    fn visit_f64<Failure: serde::de::Error>(
        self,
        _value: f64,
    ) -> Result<NativeHookToolInput, Failure> {
        Self::skipped(IgnoredAny)
    }

    fn visit_unit<Failure: serde::de::Error>(self) -> Result<NativeHookToolInput, Failure> {
        Self::skipped(IgnoredAny)
    }
}

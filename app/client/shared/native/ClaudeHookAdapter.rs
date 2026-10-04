use crate::models::{
    NativeClaudeHookPayload, NativeHookDialect, NativeLogicalEvent, NativeLogicalReceipt,
};
use crate::utils::HarnessFileUtils;
use crate::ClaudeHookMapper;
use std::io::Read;

pub struct ClaudeHookAdapter {
    client: crate::services::LogicalEventClientService,
    dialect: NativeHookDialect,
}

impl ClaudeHookAdapter {
    pub const MAXIMUM_PAYLOAD_BYTES: u64 = 16_777_216;
    pub const CLAUDE_FAMILY: &'static str = "Claude";
    pub const CLAUDE_AGENT_NAME: &'static str = "Claude Code";
    pub const CODEX_FAMILY: &'static str = "Codex";
    pub const CODEX_AGENT_NAME: &'static str = "Codex CLI";
    pub const QWEN_FAMILY: &'static str = "Qwen";
    pub const QWEN_AGENT_NAME: &'static str = "Qwen Code";
    pub const PI_FAMILY: &'static str = "Pi";
    pub const PI_AGENT_NAME: &'static str = Self::PI_FAMILY;
    pub const GOOSE_FAMILY: &'static str = "Goose";
    pub const GOOSE_AGENT_NAME: &'static str = Self::GOOSE_FAMILY;
    pub const GEMINI_FAMILY: &'static str = "Gemini";
    pub const GEMINI_AGENT_NAME: &'static str = "Gemini CLI";
    pub const OPENCODE_FAMILY: &'static str = "OpenCode";
    pub const KILO_FAMILY: &'static str = "Kilo";
    pub const GEMINI_BEFORE_AGENT: &'static str = "BeforeAgent";
    pub const GEMINI_AFTER_AGENT: &'static str = "AfterAgent";
    const EXPECTED: &'static str = "one hook JSON object";
    const EVENT_KEY: &'static str = "hook_event_name";
    const GOOSE_EVENT_KEY: &'static str = "event";
    const SESSION_KEY: &'static str = "session_id";
    const PROMPT_KEY: &'static str = "prompt_id";
    const TURN_KEY: &'static str = "turn_id";
    const AGENT_KEY: &'static str = "agent_id";
    const AGENT_TYPE_KEY: &'static str = "agent_type";
    const TOOL_NAME_KEY: &'static str = "tool_name";
    const TOOL_USE_KEY: &'static str = "tool_use_id";
    const HARNESS_KEY: &'static str = "harness";
    const ORIGIN_KEY: &'static str = "cursor_version";
    const MODEL_KEY: &'static str = "model";
    const SHARED_FIELDS: &'static [(&'static str, &'static str)] = &[
        (Self::EVENT_KEY, Self::EVENT_KEY),
        (Self::SESSION_KEY, Self::SESSION_KEY),
        (Self::PROMPT_KEY, Self::PROMPT_KEY),
        (Self::TURN_KEY, Self::TURN_KEY),
        (Self::AGENT_KEY, Self::AGENT_KEY),
        (Self::AGENT_TYPE_KEY, Self::AGENT_TYPE_KEY),
        (Self::TOOL_NAME_KEY, Self::TOOL_NAME_KEY),
        (Self::TOOL_USE_KEY, Self::TOOL_USE_KEY),
        (Self::ORIGIN_KEY, Self::ORIGIN_KEY),
    ];
    const OPENCODE_FIELDS: &'static [(&'static str, &'static str)] = &[
        (Self::HARNESS_KEY, Self::HARNESS_KEY),
        (Self::TOOL_USE_KEY, Self::TOOL_USE_KEY),
        (Self::TOOL_NAME_KEY, Self::TOOL_NAME_KEY),
        (Self::PROMPT_KEY, Self::PROMPT_KEY),
        (Self::SESSION_KEY, Self::SESSION_KEY),
        (Self::EVENT_KEY, Self::EVENT_KEY),
    ];
    const GOOSE_FIELDS: &'static [(&'static str, &'static str)] = &[
        (Self::GOOSE_EVENT_KEY, Self::EVENT_KEY),
        (Self::SESSION_KEY, Self::SESSION_KEY),
    ];
    const GEMINI_FIELDS: &'static [(&'static str, &'static str)] = &[
        (Self::SESSION_KEY, Self::SESSION_KEY),
        (Self::EVENT_KEY, Self::EVENT_KEY),
    ];
    const GEMINI_EVENTS: &'static [(&'static str, &'static str)] = &[
        (
            ClaudeHookMapper::SESSION_START,
            ClaudeHookMapper::SESSION_START,
        ),
        (
            Self::GEMINI_BEFORE_AGENT,
            ClaudeHookMapper::USER_PROMPT_SUBMIT,
        ),
        (Self::GEMINI_AFTER_AGENT, ClaudeHookMapper::STOP),
        (ClaudeHookMapper::SESSION_END, ClaudeHookMapper::SESSION_END),
    ];
    const OPENCODE_FAMILIES: &'static [&'static str] = &[Self::OPENCODE_FAMILY, Self::KILO_FAMILY];
    pub const CLAUDE: NativeHookDialect = Self::reporting_model(Self::fixed(
        Self::CLAUDE_FAMILY,
        Self::CLAUDE_AGENT_NAME,
        Self::SHARED_FIELDS,
    ));
    pub const CODEX: NativeHookDialect = Self::reporting_model(Self::fixed(
        Self::CODEX_FAMILY,
        Self::CODEX_AGENT_NAME,
        Self::SHARED_FIELDS,
    ));
    pub const QWEN: NativeHookDialect = Self::fixed(
        Self::QWEN_FAMILY,
        Self::QWEN_AGENT_NAME,
        Self::SHARED_FIELDS,
    );
    pub const PI: NativeHookDialect =
        Self::fixed(Self::PI_FAMILY, Self::PI_AGENT_NAME, Self::SHARED_FIELDS);
    pub const GOOSE: NativeHookDialect = Self::fixed(
        Self::GOOSE_FAMILY,
        Self::GOOSE_AGENT_NAME,
        Self::GOOSE_FIELDS,
    );
    pub const GEMINI: NativeHookDialect = NativeHookDialect {
        event_names: Some(Self::GEMINI_EVENTS),
        ..Self::fixed(
            Self::GEMINI_FAMILY,
            Self::GEMINI_AGENT_NAME,
            Self::GEMINI_FIELDS,
        )
    };
    pub const OPENCODE: NativeHookDialect = NativeHookDialect {
        family: None,
        default_agent_name: None,
        fields: Self::OPENCODE_FIELDS,
        model_key: None,
        event_names: None,
        reported_families: Self::OPENCODE_FAMILIES,
    };

    pub fn new(endpoint: String, dialect: NativeHookDialect) -> Self {
        Self {
            client: crate::services::LogicalEventClientService::new(endpoint),
            dialect,
        }
    }

    pub fn deliver(&self, input: &mut dyn Read) -> () {
        let events: Vec<NativeLogicalEvent> = self.events(input);
        let _delivery: Result<(), String> = self.send_all(&events);
    }

    pub fn events(&self, input: &mut dyn Read) -> Vec<NativeLogicalEvent> {
        let payload: Option<NativeClaudeHookPayload> = self.payload(input);
        payload
            .as_ref()
            .map(ClaudeHookMapper::events)
            .unwrap_or_default()
    }

    const fn fixed(
        family: &'static str,
        default_agent_name: &'static str,
        fields: &'static [(&'static str, &'static str)],
    ) -> NativeHookDialect {
        NativeHookDialect {
            family: Some(family),
            default_agent_name: Some(default_agent_name),
            fields,
            model_key: None,
            event_names: None,
            reported_families: &[],
        }
    }

    const fn reporting_model(dialect: NativeHookDialect) -> NativeHookDialect {
        NativeHookDialect {
            model_key: Some(Self::MODEL_KEY),
            ..dialect
        }
    }

    fn payload(&self, input: &mut dyn Read) -> Option<NativeClaudeHookPayload> {
        let mut payload: NativeClaudeHookPayload = self.parse(input)?;
        payload.hook_event_name = self.hook_name(payload.hook_event_name.take());
        payload.family = self.family(payload.harness.take())?;
        payload.default_agent_name = self.agent_name(&payload.family);
        payload.model = HarnessFileUtils::reported_identifier(payload.model.as_deref());
        Some(payload)
    }

    fn parse(&self, input: &mut dyn Read) -> Option<NativeClaudeHookPayload> {
        let mut reader: serde_json::Deserializer<
            serde_json::de::IoRead<std::io::Take<&mut dyn Read>>,
        > = serde_json::Deserializer::from_reader(input.take(Self::MAXIMUM_PAYLOAD_BYTES));
        let payload: NativeClaudeHookPayload =
            serde::de::DeserializeSeed::deserialize(self, &mut reader).ok()?;
        reader.end().ok()?;
        Some(payload)
    }

    fn read_fields<'de, Fields: serde::de::MapAccess<'de>>(
        &self,
        mut fields: Fields,
    ) -> Result<NativeClaudeHookPayload, Fields::Error> {
        let mut payload: NativeClaudeHookPayload = NativeClaudeHookPayload::default();
        while let Some(key) = fields.next_key::<String>()? {
            let shared: Option<&str> = Self::lookup(self.dialect.fields, &key)
                .or_else(|| -> Option<&str> { self.model_field(&key) });
            Self::read_value(&mut payload, shared, &mut fields)?;
        }
        Ok(payload)
    }

    fn read_value<'de, Fields: serde::de::MapAccess<'de>>(
        payload: &mut NativeClaudeHookPayload,
        shared: Option<&str>,
        fields: &mut Fields,
    ) -> Result<(), Fields::Error> {
        match shared {
            Some(Self::EVENT_KEY) => payload.hook_event_name = fields.next_value()?,
            Some(Self::SESSION_KEY) => payload.session_id = fields.next_value()?,
            Some(Self::PROMPT_KEY) => payload.prompt_id = fields.next_value()?,
            Some(Self::TURN_KEY) => payload.turn_id = fields.next_value()?,
            Some(Self::AGENT_KEY) => payload.agent_id = fields.next_value()?,
            Some(Self::AGENT_TYPE_KEY) => payload.agent_type = fields.next_value()?,
            Some(Self::TOOL_NAME_KEY) => payload.tool_name = fields.next_value()?,
            Some(Self::TOOL_USE_KEY) => payload.tool_use_id = fields.next_value()?,
            Some(Self::HARNESS_KEY) => payload.harness = fields.next_value()?,
            Some(Self::ORIGIN_KEY) => payload.origin_version = fields.next_value()?,
            Some(Self::MODEL_KEY) => payload.model = fields.next_value()?,
            _ => fields.next_value::<serde::de::IgnoredAny>().map(drop)?,
        }
        Ok(())
    }

    fn lookup(table: &'static [(&'static str, &'static str)], key: &str) -> Option<&'static str> {
        table
            .iter()
            .find(|(source, _): &&(&str, &str)| -> bool { *source == key })
            .map(|(_, shared): &(&'static str, &'static str)| -> &'static str { *shared })
    }

    fn model_field(&self, key: &str) -> Option<&'static str> {
        (self.dialect.model_key == Some(key)).then_some(Self::MODEL_KEY)
    }

    fn hook_name(&self, name: Option<String>) -> Option<String> {
        match self.dialect.event_names {
            Some(table) => Self::lookup(table, name.as_deref()?).map(str::to_owned),
            None => name,
        }
    }

    fn family(&self, harness: Option<String>) -> Option<String> {
        let reported: Option<String> = harness.filter(|name: &String| -> bool {
            self.dialect.reported_families.contains(&name.as_str())
        });
        reported.or(self.dialect.family.map(str::to_owned))
    }

    fn agent_name(&self, family: &str) -> String {
        self.dialect.default_agent_name.unwrap_or(family).to_owned()
    }

    fn send_all(&self, events: &[NativeLogicalEvent]) -> Result<(), String> {
        events
            .iter()
            .try_for_each(|event: &NativeLogicalEvent| -> Result<(), String> { self.send(event) })
    }

    fn send(&self, event: &NativeLogicalEvent) -> Result<(), String> {
        self.client.send(event).map(Self::ignore_receipt)
    }

    fn ignore_receipt(_receipt: NativeLogicalReceipt) -> () {}
}

impl<'de, 'adapter> serde::de::DeserializeSeed<'de> for &'adapter ClaudeHookAdapter {
    type Value = NativeClaudeHookPayload;

    fn deserialize<Input: serde::Deserializer<'de>>(
        self,
        input: Input,
    ) -> Result<NativeClaudeHookPayload, Input::Error> {
        input.deserialize_map(self)
    }
}

impl<'de, 'adapter> serde::de::Visitor<'de> for &'adapter ClaudeHookAdapter {
    type Value = NativeClaudeHookPayload;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str(ClaudeHookAdapter::EXPECTED)
    }

    fn visit_map<Fields: serde::de::MapAccess<'de>>(
        self,
        fields: Fields,
    ) -> Result<NativeClaudeHookPayload, Fields::Error> {
        self.read_fields(fields)
    }
}

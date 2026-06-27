use anyhow::{anyhow, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

use crate::commands::settings::{CleanupIntensity, LlmProvider};

/// Build the cleanup prompt according to the user's chosen intensity.
pub fn build_prompt(intensity: CleanupIntensity, raw_text: &str) -> String {
    let base_rule = "去掉口头禅（嗯/啊/那个/然后就是说）、修正明显语序错误、添加恰当标点、保留原意和说话人语气。只输出整理后的纯文本，不要任何解释或标签。";
    let extra = match intensity {
        CleanupIntensity::Light => "尽量少改动，只修正明显错误。",
        CleanupIntensity::Normal => "",
        CleanupIntensity::Aggressive => "进一步压缩重复内容、合并短句，让输出更简洁书面。",
    };
    format!(
        "你是语音输入整理助手。{}{}\n输入：{}",
        base_rule, extra, raw_text
    )
}

#[derive(Debug, Clone)]
pub struct CloudConfig {
    pub provider: LlmProvider,
    pub api_key: String,
    pub model: String,
}

impl CloudConfig {
    pub fn is_configured(&self) -> bool {
        !self.api_key.trim().is_empty() && !self.model.trim().is_empty()
    }
}

/// OpenAI-compatible chat completion request (used by DeepSeek, also a
/// common shape for many providers).
#[derive(Serialize)]
struct ChatReq<'a> {
    model: &'a str,
    messages: Vec<ChatMsg<'a>>,
    temperature: f32,
    max_tokens: u32,
    stream: bool,
}

#[derive(Serialize)]
struct ChatMsg<'a> {
    role: &'a str,
    content: &'a str,
}

#[derive(Deserialize)]
struct ChatResp {
    choices: Vec<ChatChoice>,
}

#[derive(Deserialize)]
struct ChatChoice {
    message: ChatRespMsg,
}

#[derive(Deserialize)]
struct ChatRespMsg {
    content: String,
}

pub struct CloudLlm {
    cfg: CloudConfig,
    client: Client,
}

impl CloudLlm {
    pub fn new(cfg: CloudConfig) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(60))
            .build()
            .expect("reqwest client");
        Self { cfg, client }
    }

    /// Dispatch the cleanup request to the configured cloud provider.
    pub async fn cleanup(&self, raw_text: &str, intensity: CleanupIntensity) -> Result<String> {
        if !self.cfg.is_configured() {
            return Err(anyhow!("cloud LLM not configured (missing api_key or model)"));
        }
        let prompt = build_prompt(intensity, raw_text);
        match self.cfg.provider {
            LlmProvider::Local => Err(anyhow!("CloudLlm called with Local provider")),
            LlmProvider::DeepSeek => self.openai_compat(
                "https://api.deepseek.com/v1/chat/completions",
                &self.cfg.api_key,
                &self.cfg.model,
                &prompt,
            ).await,
            LlmProvider::QwenDashScope => self.dashscope_completions(&prompt).await,
        }
    }

    /// Generic OpenAI-compatible chat completion (DeepSeek, also works for many
    /// providers using the same shape).
    async fn openai_compat(
        &self,
        url: &str,
        api_key: &str,
        model: &str,
        prompt: &str,
    ) -> Result<String> {
        let req = ChatReq {
            model,
            messages: vec![ChatMsg { role: "user", content: prompt }],
            temperature: 0.2,
            max_tokens: 1024,
            stream: false,
        };
        let res = self.client.post(url)
            .bearer_auth(api_key)
            .json(&req)
            .send()
            .await?;
        let status = res.status();
        let body_text = res.text().await?;
        if !status.is_success() {
            return Err(anyhow!("{} {}", status, body_text));
        }
        let resp: ChatResp = serde_json::from_str(&body_text)
            .map_err(|e| anyhow!("json {}: {} | body={}", status, e, body_text))?;
        resp.choices.into_iter().next()
            .map(|c| c.message.content.trim().to_string())
            .ok_or_else(|| anyhow!("empty response from provider"))
    }

    /// Alibaba DashScope uses the OpenAI-compatible /compatible-mode/v1/chat/completions
    /// endpoint, which avoids the need to implement their proprietary Message API.
    async fn dashscope_completions(&self, prompt: &str) -> Result<String> {
        let url = "https://dashscope.aliyuncs.com/compatible-mode/v1/chat/completions";
        self.openai_compat(url, &self.cfg.api_key, &self.cfg.model, prompt).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prompt_contains_input_text() {
        let p = build_prompt(CleanupIntensity::Normal, "嗯那个这个就是测试一下");
        assert!(p.contains("测试一下"));
        assert!(p.contains("整理"));
    }

    #[test]
    fn prompt_intensity_changes_rules() {
        let light = build_prompt(CleanupIntensity::Light, "x");
        let aggr = build_prompt(CleanupIntensity::Aggressive, "x");
        assert_ne!(light, aggr);
    }

    #[test]
    fn cloud_config_detects_unconfigured() {
        let c = CloudConfig {
            provider: LlmProvider::DeepSeek,
            api_key: "".into(),
            model: "deepseek-chat".into(),
        };
        assert!(!c.is_configured());

        let c2 = CloudConfig {
            provider: LlmProvider::DeepSeek,
            api_key: "sk-xxx".into(),
            model: "deepseek-chat".into(),
        };
        assert!(c2.is_configured());
    }
}
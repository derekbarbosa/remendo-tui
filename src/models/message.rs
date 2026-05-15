//! Email message domain types.
//!
//! Named `EmailMessage` (not `Message`) to avoid collision with the TEA
//! `Message` enum in `src/update.rs`.

use serde::Deserialize;

/// A full email message from the Sashiko instance.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct EmailMessage {
    /// Database identifier.
    #[serde(default)]
    pub id: i64,
    /// RFC 2822 `Message-ID`.
    #[serde(default)]
    pub message_id: String,
    /// Thread identifier.
    #[serde(default)]
    pub thread_id: Option<i64>,
    /// `Message-ID` this message replies to.
    #[serde(default)]
    pub in_reply_to: Option<String>,
    /// Author email or name.
    #[serde(default)]
    pub author: Option<String>,
    /// Subject line.
    #[serde(default)]
    pub subject: Option<String>,
    /// Unix timestamp.
    #[serde(default)]
    pub date: Option<i64>,
    /// Email body text.
    #[serde(default)]
    pub body: Option<String>,
    /// `To` header recipients.
    #[serde(default)]
    pub to: Option<String>,
    /// `Cc` header recipients.
    #[serde(default)]
    pub cc: Option<String>,
    /// NNTP group / mailing list identifier.
    #[serde(default)]
    pub mailing_list: Option<String>,
    /// Patch diff content, if applicable.
    #[serde(default)]
    pub diff: Option<String>,
}

/// A lightweight thread entry from patchset detail responses.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct ThreadMessage {
    /// Database identifier.
    #[serde(default)]
    pub id: i64,
    /// RFC 2822 `Message-ID`.
    #[serde(default)]
    pub message_id: Option<String>,
    /// Author email or name.
    #[serde(default)]
    pub author: Option<String>,
    /// Unix timestamp.
    #[serde(default)]
    pub date: Option<i64>,
    /// Subject line.
    #[serde(default)]
    pub subject: Option<String>,
    /// `Message-ID` this message replies to.
    #[serde(default)]
    pub in_reply_to: Option<String>,
}

#[cfg(test)]
    #[allow(clippy::expect_used)]
    mod tests {
    use super::*;

    #[test]
    fn deserialize_email_message() {
        let json = r#"{
            "id": 141626,
            "message_id": "20260513-test@example.com",
            "thread_id": 25647,
            "author": "dev@example.com",
            "subject": "[PATCH v2 1/4] fix null deref",
            "date": 1778690984,
            "mailing_list": "org.kernel.vger.linux-devicetree"
        }"#;
        let msg: EmailMessage = serde_json::from_str(json).expect("deserialize email");
        assert_eq!(msg.id, 141_626);
        assert_eq!(msg.message_id, "20260513-test@example.com");
        assert_eq!(msg.thread_id, Some(25647));
    }

    #[test]
    fn deserialize_thread_message() {
        let json = r#"{
            "id": 1,
            "message_id": "msg@example.com",
            "author": "test@example.com",
            "date": 1778575801,
            "subject": "Re: [PATCH]",
            "in_reply_to": "parent@example.com"
        }"#;
        let tm: ThreadMessage = serde_json::from_str(json).expect("deserialize thread msg");
        assert_eq!(tm.id, 1);
        assert_eq!(tm.in_reply_to.as_deref(), Some("parent@example.com"));
    }
}

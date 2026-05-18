//! Test fixture constructors for domain model types.
//!
//! Provides `::fixture()` methods that return realistically populated
//! instances suitable for unit and snapshot tests without network calls.

#![allow(clippy::must_use_candidate, clippy::wildcard_imports)]

use super::*;

impl Patchset {
    /// Returns a fully populated fixture patchset for testing.
    pub fn fixture() -> Self {
        Self {
            id: 1,
            subject: Some("[PATCH v2 1/3] Fix null deref in netfilter".to_string()),
            status: PatchsetStatus::Reviewed,
            thread_id: Some(100),
            author: Some("developer@example.com".to_string()),
            date: Some(1_778_690_980),
            message_id: Some("20260513-fix-null-deref@example.com".to_string()),
            total_parts: Some(3),
            received_parts: Some(3),
            subsystems: vec!["netfilter-devel".to_string(), "LKML".to_string()],
            findings: FindingCounts {
                low: 1,
                medium: 2,
                high: 0,
                critical: 0,
            },
            baseline_id: Some(1),
            failed_reason: None,
            model_name: Some("gemini-3.1-pro-preview".to_string()),
            provider: Some("gemini".to_string()),
        }
    }
}

impl Patch {
    /// Returns a fixture patch for testing.
    pub fn fixture() -> Self {
        Self {
            id: 10,
            message_id: Some("20260513-patch-1@example.com".to_string()),
            part_index: Some(1),
            subject: Some("[PATCH v2 1/3] Fix null deref in nf_tables".to_string()),
            status: Some(PatchStatus::Reviewed),
            apply_error: None,
            msg_db_id: Some(500),
        }
    }
}

impl Review {
    /// Returns a fixture review for testing.
    pub fn fixture() -> Self {
        Self {
            id: 100,
            patch_id: 10,
            status: ReviewStatus::Reviewed,
            model: Some("gemini-3.1-pro-preview".to_string()),
            provider: Some("gemini".to_string()),
            inline_review: Some("LGTM with minor suggestions.".to_string()),
            summary: Some("The patch correctly fixes a null pointer dereference.".to_string()),
            created_at: Some(1_778_699_636),
        }
    }
}

impl EmailMessage {
    /// Returns a fixture email message for testing.
    pub fn fixture() -> Self {
        Self {
            id: 141_626,
            message_id: "20260513-test@example.com".to_string(),
            thread_id: Some(25_647),
            in_reply_to: None,
            author: Some("developer@example.com".to_string()),
            subject: Some("[PATCH v2 0/3] Fix null deref series".to_string()),
            date: Some(1_778_690_984),
            body: Some("This patch series fixes a null pointer dereference...".to_string()),
            to: Some("netfilter-devel@vger.kernel.org".to_string()),
            cc: Some("linux-kernel@vger.kernel.org".to_string()),
            mailing_list: Some("org.kernel.vger.netfilter-devel".to_string()),
            diff: None,
        }
    }

    /// Returns a fixture email message with diff content for testing.
    pub fn fixture_with_diff() -> Self {
        Self {
            diff: Some(concat!(
                "diff --git a/foo.c b/foo.c\n",
                "index abc123..def456 100644\n",
                "--- a/foo.c\n",
                "+++ b/foo.c\n",
                "@@ -10,3 +10,4 @@ int main(void)\n",
                " int x = 0;\n",
                "-    return 0;\n",
                "+    x = compute();\n",
                "+    return x;\n",
            ).to_string()),
            ..Self::fixture()
        }
    }
}

impl PatchsetDetail {
    /// Returns a fixture patchset detail with nested data for testing.
    pub fn fixture() -> Self {
        Self {
            id: 1,
            subject: Some("[PATCH v2 0/3] Fix null deref in netfilter".to_string()),
            status: PatchsetStatus::Reviewed,
            author: Some("developer@example.com".to_string()),
            date: Some(1_778_690_980),
            message_id: Some("20260513-fix-null-deref@example.com".to_string()),
            total_parts: Some(3),
            received_parts: Some(3),
            subsystems: vec!["netfilter-devel".to_string()],
            baseline: Some(common::Baseline {
                branch: Some("nf/HEAD".to_string()),
                commit: Some("abc123def456".to_string()),
                repo_url: Some(
                    "git://git.kernel.org/pub/scm/linux/kernel/git/netfilter/nf.git".to_string(),
                ),
            }),
            baseline_logs: None,
            patches: vec![Patch::fixture()],
            reviews: vec![Review::fixture()],
            thread: vec![ThreadMessage::fixture()],
            model_name: Some("gemini-3.1-pro-preview".to_string()),
            provider: Some("gemini".to_string()),
            failed_reason: None,
        }
    }
}

impl ServerStats {
    /// Returns a fixture server stats for testing.
    pub fn fixture() -> Self {
        Self {
            status: "ok".to_string(),
            version: "0.1.6".to_string(),
            pending: 42,
            reviewing: 3,
            messages: 136_621,
            patchsets: 19_558,
        }
    }
}

impl MailingList {
    /// Returns a fixture mailing list for testing.
    pub fn fixture() -> Self {
        Self {
            name: "LKML".to_string(),
            group: Some("org.kernel.vger.linux-kernel".to_string()),
        }
    }
}

impl ThreadMessage {
    /// Returns a fixture thread message for testing.
    pub fn fixture() -> Self {
        Self {
            id: 1,
            message_id: Some("20260513-thread@example.com".to_string()),
            author: Some("reviewer@example.com".to_string()),
            date: Some(1_778_700_000),
            subject: Some("Re: [PATCH v2 1/3] Fix null deref".to_string()),
            in_reply_to: Some("20260513-patch-1@example.com".to_string()),
        }
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn patchset_fixture_is_valid() {
        let ps = Patchset::fixture();
        assert_eq!(ps.id, 1);
        assert_eq!(ps.status, PatchsetStatus::Reviewed);
        assert!(ps.subject.is_some());
        assert!(ps.author.is_some());
        assert!(!ps.findings.is_empty());
    }

    #[test]
    fn patch_fixture_is_valid() {
        let p = Patch::fixture();
        assert_eq!(p.id, 10);
        assert_eq!(p.status, Some(PatchStatus::Reviewed));
    }

    #[test]
    fn review_fixture_is_valid() {
        let r = Review::fixture();
        assert_eq!(r.status, ReviewStatus::Reviewed);
        assert!(r.summary.is_some());
    }

    #[test]
    fn email_message_fixture_is_valid() {
        let m = EmailMessage::fixture();
        assert!(!m.message_id.is_empty());
        assert!(m.body.is_some());
    }

    #[test]
    fn thread_message_fixture_is_valid() {
        let tm = ThreadMessage::fixture();
        assert!(tm.message_id.is_some());
        assert!(tm.in_reply_to.is_some());
    }
}

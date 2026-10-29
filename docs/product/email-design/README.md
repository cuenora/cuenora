# Cuenora email designs

Review the [seven-page design reference](export.pdf): a reusable email foundation,
email verification and password recovery, including narrow and dark specimens.
These are static designs extending [P04.12](../email-verification-and-recovery.md),
not implemented templates, working links or a mail-delivery integration.

## Review the designs

| Reference | Preview | Pen frame |
| --- | --- | --- |
| Base, with and without an action | [PNG](boards/cuenora-email-base-template.png) | `Cvdqb` |
| Verification, desktop | [PNG](boards/cuenora-email-verification-desktop.png) | `T23dO` |
| Recovery, desktop | [PNG](boards/cuenora-email-recovery-desktop.png) | `B5taoG` |
| Verification, narrow | [PNG](boards/cuenora-email-verification-mobile.png) | `XJJTr` |
| Recovery, narrow | [PNG](boards/cuenora-email-recovery-mobile.png) | `ApxKs` |
| Verification, dark | [PNG](boards/cuenora-email-verification-dark.png) | `C79uj` |
| Recovery, dark | [PNG](boards/cuenora-email-recovery-dark.png) | `J3JBF` |

`messages/` contains the six message-only PNG exports at 2x, without the specimen
labels, subject and preview annotations. `boards/` contains complete references
at 1x. Subject and preview annotations are handoff metadata, not email body copy.

The reusable component is `Email / Base / Reusable service message` (`RN1jn`) in
the open [Pen document](../../cuenora-design.pen). The new messages instance it;
the header instances the existing vector symbol, not a regenerated logo.

## Reusable foundation

The single-column message has a 600-pixel desktop width, 40-pixel inner padding,
and an independent height that grows with content. Narrow references use a
360-pixel outer frame, a 328-pixel message and 24-pixel inner padding. Typography
retains Bodoni Moda headings and IBM Plex Mono body copy; light/dark colors reuse
the existing [semantic roles](../ui-design-system.md#token-contract).

| Block | Template field | Rule |
| --- | --- | --- |
| Message metadata | Subject and preview | Set separately for each message; preview is not visible body copy. |
| Category and heading | `message_category`, `message_title` | State the purpose plainly. |
| Main content | `message_body` | Required; must make sense without images. |
| Primary action | `action_label`, `action_url` | Optional; omit entirely for informational messages. |
| Context notice | `context_title`, `context_body` | Optional; reserve for useful consequences or limits. |
| Supporting copy | `supporting_copy` | Optional and specific to the message. |
| Link fallback | Same `action_url` | Include only when an action exists; never a different destination. |
| Delivery reason | `delivery_reason` | Explain why this message was sent. |
| Brand signature | Approved Cuenora message | Shared footer; no invented sender or support address. |

The base board shows both action and information-only compositions. It is a
visual foundation for future email types, not permission to reuse security copy
or authentication links in unrelated messages. Marketing/subscription behavior,
if ever introduced, needs its own requirements.

## Authentication copy and behavior

| Detail | Verification | Password recovery |
| --- | --- | --- |
| Subject | Verify your email address — Cuenora | Reset your password — Cuenora |
| Preview | Confirm your email address to continue setting up Cuenora. | Choose a new password. This recovery link is valid for 30 minutes. |
| Button | Continue to verification | Choose a new password |
| URL placeholder | `{{verification_url}}` | `{{recovery_url}}` |
| Lifetime | 24 hours from issuance | 30 minutes from issuance |
| Authority and consequence | Sign in to the same account before explicit confirmation, including on another device. | Saving a new password signs out every device; normal login follows. |

Opening either email link does not complete the operation. Copy explains
single-use expiry, what to do with an unsolicited message, and not sharing the
link. A reset does not verify email or issue a session. Exact expiry remains the
server's challenge expiry from committed creation, not receipt or opening time.
The message describes this as “after it was issued.”

## Implementation boundary and checks

- Replace placeholders only in future implementation; never send literal braces
  or put live credentials/tokens into design exports. Escape template text.
- Follow the approved P04.12 allowlisted HTTPS origin, fragment-token and explicit
  confirmation policies. Authentication mail must disable tracking and link
  rewriting. No sender/domain/provider access or real email send was performed.
- Add font fallbacks, accessible linked text/image alternatives and a plain-text
  equivalent during implementation. Test with images disabled and long URLs.
- Dark and narrow boards are visual references, not proof of automatic dark mode
  or compatibility with Gmail, Outlook, Apple Mail or assistive technologies.
- All seven boards were visually inspected; resolved visible-node bounds had no
  clipping. The eight text/button color pairings checked exceed 4.5:1. Exported
  PNG files were decoded successfully. These checks do not certify email clients.

The native document was subsequently saved in Pen; its on-disk timestamp advanced
to 2026-10-29 22:02:49 +01:00 after the email additions. The editable source and
exports are persisted together. Existing logo/system layers were preserved. No
roadmap checkbox or application code changed.

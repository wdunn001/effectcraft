//! Landing on a keyed invite link (`?invite=<token>`), through `craft_ui::cloud`'s contract and
//! nothing else.
//!
//! A pre-RFC branch (`quasarke`, never merged) redeemed `?invite` through
//! `craft_cloud_client::invite`, a personal fork's API with no equivalent in
//! `storytold/craft-libs#160`'s contract and no equivalent planned: deep-link redemption is a
//! deployment's own front door, not something a vendor-neutral `Files`/`Sharing` contract has an
//! opinion on. [`land`] is the contract half of that handoff, exactly as
//! `craft_ui::cloud::CloudFiles::as_guest`'s own doc comment describes it ("called once, before
//! anything is drawn, by whatever landed them here"): it calls `as_guest` for the title bar and
//! [`effectcraft_ui_egui::cloud::CloudApp::open_bytes`] for the document, both already covered by
//! this repository's own tests (`crates/ui-egui/tests/it/cloud_files.rs`).
//!
//! [`InviteTransport`] is the other half, and ships with no implementation: redeeming a token
//! means reaching a specific service's specific redemption endpoint, and no storytold-hosted
//! service exists yet to reach. [`start`]'s caller always passes `None`, so a `?invite` link on
//! this build today is detected and reported, never opened; the moment a storytold crate
//! implements [`InviteTransport`], wiring it in is the entire remaining change.

use effectcraft_ui_egui::EffectcraftApp;
use effectcraft_ui_egui::cloud::CloudApp;

/// What landing on an invite needs, once something has redeemed the token: who it was sent to,
/// whether their copy is read-only, and the document itself.
pub struct Invitation {
    pub recipient: String,
    pub read_only: bool,
    pub name: String,
    pub bytes: Vec<u8>,
}

/// Turns an invite `token` (the value of `?invite=`) into an [`Invitation`], by whatever means a
/// specific deployment's service uses to answer that. No type here names one: see the module
/// doc comment for why this crate ships no implementation.
pub trait InviteTransport {
    fn redeem(&self, token: &str) -> Result<Invitation, String>;
}

/// The token in `?invite=<token>`, if the page was opened with one.
pub fn token() -> Option<String> {
    crate::query_value("invite")
}

/// Lands an already-redeemed invite on `app`. `CloudFiles::as_guest` runs first, so the title
/// bar reflects a recipient with no account before the document it names appears; `open_bytes`
/// runs through the same engine path every other open does (`effectcraft_ui_egui::cloud`), not a
/// separate one for invites.
pub fn land(app: &mut EffectcraftApp, invitation: Invitation) -> Result<(), String> {
    app.cloud_mut().as_guest(invitation.recipient, invitation.read_only);
    app.open_bytes(&invitation.name, invitation.bytes)?;
    Ok(())
}

/// Checks for `?invite=<token>` and redeems it through `transport`, landing the result on `app`.
/// Does nothing when there is no token, and reports why rather than opening anything when
/// `transport` is `None`: today, always, since no build of this crate constructs one yet.
pub fn start(app: &mut EffectcraftApp, transport: Option<&dyn InviteTransport>) {
    let Some(token) = token() else { return };
    match transport {
        None => log::warn!("invite link ignored: no cloud service is configured to redeem it"),
        Some(transport) => match transport.redeem(&token) {
            Ok(invitation) => {
                if let Err(e) = land(app, invitation) {
                    log::error!("invite open failed: {e}");
                }
            }
            Err(e) => log::warn!("invite redemption failed: {e}"),
        },
    }
}

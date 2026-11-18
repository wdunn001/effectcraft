//! Wiring for `craft_ui::cloud`'s vendor-neutral Open/Save/Share contract
//! (`storytold/craft-libs#160`, `storytold/effectcraft#1623`).
//!
//! [`EffectcraftApp`] implements [`craft_ui::cloud::CloudApp`] in full, so the contract's own
//! gating, conflict handling and account-generation fencing apply exactly as every other Craft
//! app gets them. No part of that implementation names a provider, and nothing in this crate
//! constructs one: [`EffectcraftApp::new`](super::EffectcraftApp::new) always starts with
//! [`CloudFiles::default`], which has no backend connected. [`craft_ui::cloud::available`] is
//! `false` until something calls [`CloudFiles::connect`], and this crate never does.
//!
//! No cloud UI is wired in this app yet: no menu item, account badge, Open/Save page or share
//! dialog is drawn anywhere. There is nothing to hide behind [`craft_ui::cloud::available`],
//! because nothing offers it in the first place. [`can_save`] exists so a future control has a
//! correct guard ready to call, documented and tested, before it is added; see the regression
//! test on `craft_ui::cloud::save` in `tests/it/cloud_files.rs` for why that guard is load
//! bearing and not merely tidy.
//!
//! The project this app opens and saves has one document per run (`active` always answers
//! `Some(0)`): [`CloudApp::open_bytes`] and [`CloudApp::encode`] go through the engine's
//! `file.open` (its `text` parameter) and `file.toJson` commands, the same project reader and
//! writer every other open and save already uses, just handed bytes instead of a path.
//!
//! A pre-RFC branch (`quasarke`, never merged) opened a keyed invite link on EffectCraft's web
//! startup through `craft_cloud_client::invite`, a personal fork's API with nothing like it in
//! this contract. The contract half of that handoff is settled and wired here: the apps crate's
//! landing hook calls [`CloudFiles::as_guest`] and [`CloudApp::open_bytes`] exactly as the
//! contract's own doc comment describes ("called once, before anything is drawn, by whatever
//! landed them here"). Only the transport, fetching the invited bytes from a service, stays
//! deferred: `apps/effectcraft-web/src/invite.rs` states a small seam for it with no
//! implementation, because no storytold-hosted service exists yet to implement it against.

use craft_cloud::EntryKind;
// Re-exported: a host (the apps crates' startup code, e.g. `apps/effectcraft-web/src/invite.rs`)
// reaches the trait through this crate alone, with no direct `craft-ui` dependency of its own.
pub use craft_ui::cloud::CloudApp;
use craft_ui::cloud::{CloudFiles, Destination, DocumentKind};
use egui::{Color32, Painter, Rect};
use serde_json::{Value, json};

use crate::EffectcraftApp;

/// What EffectCraft's documents are, for the Open page's file list and the Save As page's
/// default extension. One `const`, per [`DocumentKind`]'s own doc comment.
const KIND: DocumentKind = DocumentKind { extensions: &["ecproj", "ecprojx"], mimes: &["application/json"], noun: "project", default_extension: "ecproj" };

/// Whether a cloud save has somewhere safe to go.
///
/// `craft_ui::cloud::save` and `save_as` do not check this themselves: both set
/// [`CloudFiles`]'s `busy` before calling [`craft_ui::cloud::submit`], and `submit` is a
/// silent no-op with no backend connected, so `busy` then never clears (see the regression
/// test `calling_save_with_no_backend_wedges_busy_forever` in `tests/it/cloud_files.rs`). A
/// host must call this, not just [`craft_ui::cloud::available`] alone, before offering a
/// control that calls either: a backend can be wired with nobody signed in, or signed in with
/// no provider linked yet, and [`craft_ui::cloud::has_target`]/`save`/`save_as` all assume a
/// writable target already exists.
pub fn can_save<A: CloudApp>(app: &A) -> bool {
    craft_ui::cloud::available(app) && app.cloud().account.as_ref().is_some_and(|account| account.provider_linked)
}

impl CloudApp for EffectcraftApp {
    fn cloud(&self) -> &CloudFiles {
        &self.cloud
    }

    fn cloud_mut(&mut self) -> &mut CloudFiles {
        &mut self.cloud
    }

    fn kind(&self) -> DocumentKind {
        KIND
    }

    /// One document per run: this app has no concept of a second one open at once, so there is
    /// always exactly one, never none.
    fn active(&self) -> Option<u64> {
        Some(0)
    }

    fn title(&self, _uid: u64) -> String {
        crate::panels::unsaved::project_name(self)
    }

    fn open_bytes(&mut self, name: &str, bytes: Vec<u8>) -> Result<Option<u64>, String> {
        let text = String::from_utf8(bytes).map_err(|_| format!("{name} is not a text project file"))?;
        self.session.execute("file.open", json!({"text": text})).map_err(|e| e.to_string())?;
        Ok(Some(0))
    }

    fn encode(&mut self, _uid: u64, into: Destination<'_>) -> Result<Vec<u8>, String> {
        let extension = into.extension();
        if !extension.is_empty() && !self.kind().extensions.contains(&extension.as_str()) {
            return Err(format!("EffectCraft writes .{} project files, not .{extension}", self.kind().default_extension));
        }
        let answer = self.session.execute("file.toJson", json!({})).map_err(|e| e.to_string())?;
        let text = answer.get("text").and_then(Value::as_str).ok_or_else(|| "file.toJson answered with no text".to_owned())?;
        Ok(text.as_bytes().to_vec())
    }

    /// Never runs: a reload replaces the one document in place (`into` and the freshly opened
    /// uid are always both `Some(0)`, so `craft_ui::cloud`'s `old != new` guard never holds).
    /// Kept only because the trait has no default for a host with more than one document.
    fn close(&mut self, _uid: u64) {}

    fn mark_saved(&mut self, _uid: u64) {
        self.session.mark_saved();
    }

    fn sharing(&mut self) -> &mut craft_ui::share::ShareDialog {
        &mut self.share
    }

    fn presence(&mut self) -> &mut craft_ui::presence::PresenceBar {
        &mut self.presence
    }

    /// A plain folder/file glyph. No cloud page draws a file list yet, so this never runs; it
    /// exists only to satisfy the trait with something that will not look broken if one ever
    /// does, without reaching into this app's own icon atlas for a provider-neutral contract.
    fn paint_icon(&self, painter: &Painter, rect: Rect, kind: EntryKind, colour: Color32) {
        let _ = match kind {
            EntryKind::Folder => painter.rect_filled(rect, 3.0, colour.gamma_multiply(0.35)),
            EntryKind::File => painter.rect_stroke(rect, 2.0, egui::Stroke::new(1.3, colour), egui::StrokeKind::Middle),
        };
    }

    /// Opens `url` in the user's browser through the egui context the last tick cached
    /// (`EffectcraftApp::tick`); a no-op before the first tick, which never matters since
    /// nothing in this crate calls a cloud command before one runs.
    fn open_url(&self, url: &str) {
        if let Some(ctx) = &self.egui_ctx {
            ctx.open_url(egui::OpenUrl::new_tab(url));
        }
    }

    fn say(&mut self, text: String) {
        crate::panels::forms::info(self, "EffectCraft", &text);
    }

    fn toast(&mut self, text: String) {
        let now = self.now();
        self.toast = Some((text, now));
    }

    fn app_name(&self) -> &str {
        "EffectCraft"
    }
}

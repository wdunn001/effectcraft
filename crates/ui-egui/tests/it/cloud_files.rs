//! `EffectcraftApp` implements the real `craft_ui::cloud::CloudApp` contract
//! (`storytold/craft-libs#160`), not a hand-rolled stand-in: these tests drive it through
//! `craft_cloud_local::LocalBackend`, the reference `CloudBackend` the contract's own crate
//! ships to prove it needs no specific provider. No mock is defined in this file and none is
//! defined in `effectcraft_ui_egui::cloud` either, so there is exactly one of this round trip in
//! the crate, not two.
//!
//! A build with no backend connected offers nothing cloud-shaped anywhere: no menu item, no
//! account badge, no Open/Save page, no share dialog. That is also the honest truth about this
//! app today independent of a backend: none of those exist yet in `effectcraft-ui-egui`'s own
//! UI code. `cloud_ui_offers_nothing_until_something_connects_a_backend` below is the guard for
//! both facts at once, the way `craft_ui::cloud::available` would gate real controls once they
//! exist.

use std::time::Duration;

use craft_ui::cloud::{self, CloudApp};
use effectcraft_engine::Session;
use effectcraft_ui_egui::EffectcraftApp;
use effectcraft_ui_egui::cloud::can_save;

/// Pumps `app` until `done` is true or two seconds pass. `LocalBackend` answers on a real OS
/// thread, so a test that only pumped once would race it.
fn pump_until(app: &mut EffectcraftApp, ctx: &egui::Context, mut done: impl FnMut(&EffectcraftApp) -> bool) {
    for _ in 0..400 {
        cloud::pump(app, ctx);
        if done(app) {
            return;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    panic!("a cloud job did not answer within the test's timeout");
}

fn new_app() -> EffectcraftApp {
    EffectcraftApp::new(Session::default())
}

/// Nothing cloud-shaped is offered with no backend connected, and this app draws no cloud UI at
/// all yet (no menu item, account badge, Open/Save page or share dialog exist in
/// `effectcraft-ui-egui`'s own code to gate): both halves of `storytold/effectcraft#1623`'s
/// "hidden with no backend" requirement hold, one by the contract's own gate and one because
/// there is nothing drawn to need it.
#[test]
fn cloud_ui_offers_nothing_until_something_connects_a_backend() {
    let mut app = new_app();
    assert!(!cloud::available(&app));
    assert!(app.cloud().account.is_none());
    // Reached through the contract's own trait methods, not this crate's fields: a share
    // dialog and a presence bar with nothing to show are what "no cloud UI" means for them.
    assert!(!app.sharing().available());
    assert!(!app.presence().shared());
}

/// Connecting the reference backend is enough to make the contract available; nothing in this
/// crate's own startup path ever calls `connect`, so this exercises the seam a host would use,
/// not EffectCraft's own production path.
#[test]
fn connecting_the_reference_backend_makes_the_contract_available() {
    let dir = tempfile::tempdir().expect("tempdir");
    let mut app = new_app();
    app.cloud_mut().connect(craft_cloud_local::LocalBackend::new(dir.path().to_path_buf()));
    assert!(cloud::available(&app));

    let ctx = egui::Context::default();
    cloud::start(&mut app);
    pump_until(&mut app, &ctx, |app| app.cloud().account.is_some());
    let account = app.cloud().account.as_ref().expect("account arrived");
    assert!(account.provider_linked, "a local folder is already linked the moment it exists");
    assert!(can_save(&app), "a linked account with a backend wired may save");
}

/// list/open/save, driven only through the contract's own free functions
/// (`browse`/`open_entry`/`save`), against a real on-disk backend: no bytes are faked anywhere in
/// this test.
#[test]
fn list_open_and_save_round_trip_through_the_real_contract() {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::write(
        dir.path().join("a.ecproj"),
        effectcraft_project::Project::default().to_file_json_with_view(&Default::default(), false).expect("seed project json"),
    )
    .expect("seed a.ecproj");

    let mut app = new_app();
    app.cloud_mut().connect(craft_cloud_local::LocalBackend::new(dir.path().to_path_buf()));
    let ctx = egui::Context::default();
    cloud::start(&mut app);
    pump_until(&mut app, &ctx, |app| app.cloud().account.is_some());

    cloud::browse(&mut app, "");
    pump_until(&mut app, &ctx, |app| app.cloud().mine.loaded);
    let entry = app.cloud().mine.entries.iter().find(|e| e.name == "a.ecproj").cloned().expect("a.ecproj listed");
    assert!(cloud::opens(&entry, app.kind()), "a.ecproj matches this app's own DocumentKind");

    cloud::open_entry(&mut app, entry, None);
    pump_until(&mut app, &ctx, |app| app.cloud().busy.is_none());
    assert!(app.cloud().origins.contains_key(&0), "the opened entry is now this document's cloud origin");

    assert!(can_save(&app));
    cloud::save(&mut app).expect("save starts");
    pump_until(&mut app, &ctx, |app| app.cloud().busy.is_none());
    assert!(app.cloud().conflict.is_none(), "an unconditional first save over the entry it came from never conflicts");
    let saved_etag = app.cloud().origins.get(&0).and_then(|origin| origin.entry.etag.clone());
    assert!(saved_etag.is_some(), "a successful save records the new etag `craft_ui::cloud::save` conditions the next one on");
}

/// `craft_ui::cloud::save` carries no `available()` guard of its own (`storytold/craft-libs`'s
/// own code, not this crate's): it sets `busy` before `submit`, and `submit` with no backend
/// connected is a silent no-op, so `busy` is never cleared. This is the wedge `can_save` exists
/// to prevent a host from walking into; without checking it first, a save control would leave
/// the app looking permanently busy for the rest of the session.
#[test]
fn calling_save_with_no_backend_wedges_busy_forever() {
    let mut app = new_app();
    // No `connect` anywhere in this test: `can_save` must refuse before the wedge, not after.
    assert!(!can_save(&app));
    // Manufactures the one precondition `craft_ui::cloud::save` itself checks, a writable
    // origin for the active document, entirely through `CloudFiles`'s public fields: no backend
    // is needed to populate it, which is exactly the gap `can_save` closes.
    let entry = craft_cloud::Entry {
        id: "a.ecproj".into(),
        room: "local:a.ecproj".into(),
        name: "a.ecproj".into(),
        path: "a.ecproj".into(),
        kind: craft_cloud::EntryKind::File,
        size: Some(0),
        modified: None,
        mime: None,
        etag: Some("0".into()),
        writable: true,
        shared_by: None,
    };
    app.cloud_mut().origins.insert(0, cloud::Origin { entry });

    cloud::save(&mut app).expect("craft_ui::cloud::save does not itself refuse an unavailable backend");
    assert!(app.cloud().busy.is_some(), "the documented wedge: busy was set and nothing will ever clear it");

    // The same state a host's own guard must have refused before ever reaching `save`.
    assert!(!can_save(&app));
}

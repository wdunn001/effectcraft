//! `EffectcraftApp`'s cloud seam ([`effectcraft_ui_egui::cloud`]) ships no backend: an app built
//! with [`Session::default`] starts, and stays, with no cloud provider connected, which is what
//! hides the Open/Save/Share cloud UI this contract would back (`storytold/effectcraft#1623`).
//! The in-process mock here stands in for a future `storytold/craft-libs#160` backend, entirely
//! local to this test: no external crate is involved.

use effectcraft_engine::Session;
use effectcraft_ui_egui::EffectcraftApp;
use effectcraft_ui_egui::cloud::{CloudBackend, Entry, EntryKind, ShareLink};

#[derive(Debug, Default)]
struct MockBackend {
    entries: Vec<Entry>,
}

impl CloudBackend for MockBackend {
    fn account_label(&self) -> String {
        "test@example.test".to_owned()
    }

    fn list(&self, _folder: Option<&str>) -> Result<Vec<Entry>, String> {
        Ok(self.entries.clone())
    }

    fn open(&self, entry: &Entry) -> Result<Vec<u8>, String> {
        self.entries.iter().find(|e| e.id == entry.id).map(|_| b"mock bytes".to_vec()).ok_or_else(|| "not found".to_owned())
    }

    fn save(&self, name: &str, _bytes: &[u8]) -> Result<Entry, String> {
        Ok(Entry { id: "new".into(), name: name.to_owned(), path: "/".into(), kind: EntryKind::File, writable: true })
    }

    fn share(&self, entry: &Entry) -> Result<ShareLink, String> {
        Ok(ShareLink { url: format!("https://example.test/s/{}", entry.id), can_edit: false })
    }
}

/// A freshly built app ships with no cloud backend: no account label, nothing marked available.
#[test]
fn fresh_app_has_no_cloud_backend() {
    let app = EffectcraftApp::new(Session::default());
    assert!(!app.cloud.is_available());
    assert!(app.cloud.backend().is_none());
    assert_eq!(app.cloud.account_label(), None);
}

/// Connecting an in-process mock backend is enough to make the seam available; disconnecting it
/// hides it again. Nothing in this crate calls `connect` itself (see the doc comment on
/// [`effectcraft_ui_egui::cloud`]), so this is exercising the seam a future host would use, not
/// EffectCraft's own startup path.
#[test]
fn connecting_and_disconnecting_a_mock_backend() {
    let mut app = EffectcraftApp::new(Session::default());
    app.cloud.connect(Box::new(MockBackend::default()));
    assert!(app.cloud.is_available());
    assert_eq!(app.cloud.account_label(), Some("test@example.test".to_owned()));

    app.cloud.disconnect();
    assert!(!app.cloud.is_available());
    assert_eq!(app.cloud.account_label(), None);
}

/// The mock backend's list/open/save/share round-trip, driven only through the trait.
#[test]
fn mock_backend_list_open_save_share_round_trip() {
    let entry = Entry { id: "1".into(), name: "a.ecproj".into(), path: "/".into(), kind: EntryKind::File, writable: true };
    let mut app = EffectcraftApp::new(Session::default());
    app.cloud.connect(Box::new(MockBackend { entries: vec![entry.clone()] }));

    let backend = app.cloud.backend().expect("connected");
    assert_eq!(backend.list(None).expect("list"), vec![entry.clone()]);
    assert_eq!(backend.open(&entry).expect("open"), b"mock bytes");
    assert_eq!(backend.save("b.ecproj", b"data").expect("save").name, "b.ecproj");
    assert!(backend.share(&entry).expect("share").url.contains(&entry.id));
}

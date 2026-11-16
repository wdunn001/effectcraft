//! A local, self-contained seam for a future cloud provider.
//!
//! `storytold/craft-libs#160` is expected to add a `cloud` feature to `craft-ui` and a
//! `craft-cloud` crate with a shared `CloudBackend`/`CloudApp` contract every Craft app can take
//! Open, Save and Share through. Until that lands, this module defines a small trait of the same
//! shape, owned entirely by this crate, so EffectCraft's own code has somewhere to plug a backend
//! in later without depending on a personal fork (`wdunn001/craft-libs`) or a personal service
//! (`wdunn001/collab-craft`) that the project doesn't host or review
//! (see `storytold/effectcraft#1623` and the review on `storytold/designcraft#878`).
//!
//! No backend ships from this crate: [`EffectcraftApp::new`](super::EffectcraftApp::new) always
//! starts with [`CloudState::default`], which has none connected. Every cloud menu item, account
//! badge and share dialog this contract would back is built conditionally on
//! [`CloudState::is_available`], so none of them appear until something calls
//! [`CloudState::connect`] with a real [`CloudBackend`]. When `storytold/craft-libs#160` lands,
//! this module is expected to be replaced by that crate's contract, not extended.
//!
//! A pre-RFC branch also had a keyed-invite open on web startup, redeeming a link
//! (`?invite` with a guest cookie) before the app constructed anything, through
//! `craft_cloud_client::invite`. That redemption step happens before a [`CloudBackend`] is
//! connected. No account exists yet to call a trait method on. `storytold/craft-libs#160`
//! has not yet settled how (or whether) it models deep-link redemption. Expressing this flow
//! through [`CloudBackend`] as defined here would mean guessing at that shape. This module
//! leaves the flow out and defers it to a follow-up once the real contract exists.

use std::fmt;

/// Whether a [`Entry`] names a file or a folder.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntryKind {
    File,
    Folder,
}

/// One file or folder a [`CloudBackend`] lists.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    /// The provider's id for this entry, stable across renames and moves.
    pub id: String,
    /// The display name, with extension.
    pub name: String,
    /// The folder path it lives in, provider-relative.
    pub path: String,
    pub kind: EntryKind,
    /// Whether the signed-in account can write to this entry.
    pub writable: bool,
}

/// A link a [`CloudBackend`] mints for sharing an [`Entry`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShareLink {
    pub url: String,
    /// Whether the link grants edit access, as opposed to read-only.
    pub can_edit: bool,
}

/// What a cloud provider offers a Craft app: list, open, save and share, with no vendor and no
/// transport detail leaking into the app. Mirrors the shape `craft_cloud`'s contract is expected
/// to take once `storytold/craft-libs#160` lands.
pub trait CloudBackend: fmt::Debug + Send + Sync {
    /// A short, non-secret label for the account menu (for example `"name@host"`).
    fn account_label(&self) -> String;

    /// Entries in `folder`, or the root's entries when `folder` is `None`.
    fn list(&self, folder: Option<&str>) -> Result<Vec<Entry>, String>;

    /// The current bytes of `entry`.
    fn open(&self, entry: &Entry) -> Result<Vec<u8>, String>;

    /// Writes `bytes` as `name` in the root folder, creating or overwriting it, and returns the
    /// saved entry.
    fn save(&self, name: &str, bytes: &[u8]) -> Result<Entry, String>;

    /// Mints a share link for `entry`.
    fn share(&self, entry: &Entry) -> Result<ShareLink, String>;
}

/// The app's optional connection to a cloud provider. Defaults to no backend connected, which is
/// the only state this crate ever ships: nothing here constructs a [`CloudBackend`].
#[derive(Default)]
pub struct CloudState {
    backend: Option<Box<dyn CloudBackend>>,
}

impl fmt::Debug for CloudState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CloudState").field("connected", &self.backend.is_some()).finish()
    }
}

impl CloudState {
    /// Connects a real backend. Nothing in this crate calls this; it exists for a future host
    /// (or a test) to call once a [`CloudBackend`] implementation exists.
    pub fn connect(&mut self, backend: Box<dyn CloudBackend>) {
        self.backend = Some(backend);
    }

    /// Drops the connected backend, if any.
    pub fn disconnect(&mut self) {
        self.backend = None;
    }

    /// Whether a backend is connected. The cloud menu items, account badge and share dialog this
    /// contract would back are built only when this is `true`.
    pub fn is_available(&self) -> bool {
        self.backend.is_some()
    }

    /// The connected backend, if any.
    pub fn backend(&self) -> Option<&dyn CloudBackend> {
        self.backend.as_deref()
    }

    /// The account label to show in the account menu, if a backend is connected.
    pub fn account_label(&self) -> Option<String> {
        self.backend.as_deref().map(CloudBackend::account_label)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Default)]
    struct MockBackend {
        entries: Vec<Entry>,
    }

    impl CloudBackend for MockBackend {
        fn account_label(&self) -> String {
            "mock@example.test".to_owned()
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

    #[test]
    fn no_backend_by_default() {
        let state = CloudState::default();
        assert!(!state.is_available());
        assert!(state.backend().is_none());
        assert_eq!(state.account_label(), None);
    }

    #[test]
    fn connecting_a_backend_makes_it_available() {
        let mut state = CloudState::default();
        state.connect(Box::new(MockBackend::default()));
        assert!(state.is_available());
        assert_eq!(state.account_label(), Some("mock@example.test".to_owned()));
    }

    #[test]
    fn disconnecting_hides_it_again() {
        let mut state = CloudState::default();
        state.connect(Box::new(MockBackend::default()));
        state.disconnect();
        assert!(!state.is_available());
    }

    #[test]
    fn mock_backend_round_trips_list_open_save_share() {
        let entry = Entry { id: "1".into(), name: "a.ecproj".into(), path: "/".into(), kind: EntryKind::File, writable: true };
        let backend = MockBackend { entries: vec![entry.clone()] };
        let mut state = CloudState::default();
        state.connect(Box::new(backend));
        let listed = state.backend().expect("connected").list(None).expect("list");
        assert_eq!(listed, vec![entry.clone()]);
        let bytes = state.backend().expect("connected").open(&entry).expect("open");
        assert_eq!(bytes, b"mock bytes");
        let saved = state.backend().expect("connected").save("b.ecproj", b"data").expect("save");
        assert_eq!(saved.name, "b.ecproj");
        let link = state.backend().expect("connected").share(&entry).expect("share");
        assert!(link.url.contains(&entry.id));
    }
}

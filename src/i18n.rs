//! The interface's words. Spotsie is English only: these helpers mark the
//! text the interface shows and pick the singular or plural form.

use std::borrow::Cow;

/// The language the interface is drawn in. English is the only one.
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
pub enum Locale {
    #[default]
    English,
}

pub fn gettext(_locale: Locale, source: &'static str) -> Cow<'static, str> {
    Cow::Borrowed(source)
}

/// `context` tells apart two uses of the same word at the call site.
pub fn pgettext(
    _locale: Locale,
    _context: &'static str,
    source: &'static str,
) -> Cow<'static, str> {
    Cow::Borrowed(source)
}

pub fn ngettext(
    _locale: Locale,
    singular: &'static str,
    plural: &'static str,
    count: u32,
) -> Cow<'static, str> {
    Cow::Borrowed(if count == 1 { singular } else { plural })
}

impl Locale {
    pub fn liked_song_count(self, count: u32) -> String {
        ngettext(
            self,
            "Playlist • {count} song",
            "Playlist • {count} songs",
            count,
        )
        .replace("{count}", &count.to_string())
    }

    pub fn song_count(self, count: u32) -> String {
        ngettext(self, "{count} song", "{count} songs", count)
            .replace("{count}", &count.to_string())
    }

    pub fn playlist_count(self, count: u32) -> String {
        ngettext(self, "{count} playlist", "{count} playlists", count)
            .replace("{count}", &count.to_string())
    }

    pub fn folder_playlist_count(self, count: u32) -> String {
        ngettext(
            self,
            "Folder • {count} playlist",
            "Folder • {count} playlists",
            count,
        )
        .replace("{count}", &count.to_string())
    }

    pub fn folder_state_label(self, name: &str, collapsed: bool) -> String {
        let label = if collapsed {
            gettext(self, "{name}, folder, collapsed")
        } else {
            gettext(self, "{name}, folder, expanded")
        };
        label.replace("{name}", name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_take_the_singular_only_for_one() {
        let english = Locale::English;
        assert_eq!(english.liked_song_count(0), "Playlist • 0 songs");
        assert_eq!(english.liked_song_count(1), "Playlist • 1 song");
        assert_eq!(english.liked_song_count(2), "Playlist • 2 songs");
        assert_eq!(english.song_count(1), "1 song");
        assert_eq!(english.song_count(2), "2 songs");
        assert_eq!(english.playlist_count(1), "1 playlist");
        assert_eq!(english.playlist_count(2), "2 playlists");
        assert_eq!(english.folder_playlist_count(1), "Folder • 1 playlist");
        assert_eq!(english.folder_playlist_count(5), "Folder • 5 playlists");
    }

    #[test]
    fn folder_state_labels_name_the_folder_and_its_state() {
        let english = Locale::English;
        assert_eq!(
            english.folder_state_label("Road trips", true),
            "Road trips, folder, collapsed"
        );
        assert_eq!(
            english.folder_state_label("Road trips", false),
            "Road trips, folder, expanded"
        );
    }
}

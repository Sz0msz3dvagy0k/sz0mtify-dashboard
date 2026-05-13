#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParsedArtist {
    pub name: String,
    pub role: ArtistRole,
    pub position: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ArtistRole {
    Primary,
    Featured,
}

impl ArtistRole {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Primary => "primary",
            Self::Featured => "featured",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParsedArtistCredit {
    pub raw: String,
    pub artists: Vec<ParsedArtist>,
}

impl ParsedArtistCredit {
    pub fn primary_names(&self) -> Vec<&str> {
        self.artists
            .iter()
            .filter(|artist| artist.role == ArtistRole::Primary)
            .map(|artist| artist.name.as_str())
            .collect()
    }

    pub fn is_simple_single_artist(&self) -> bool {
        self.artists.len() == 1 && self.artists[0].role == ArtistRole::Primary
    }
}

pub fn parse_artist_credit(raw: &str) -> ParsedArtistCredit {
    let raw = raw.trim();
    let raw = if raw.is_empty() {
        "Unknown Artist"
    } else {
        raw
    };
    let (primary_segment, featured_segment) = split_featured_segment(raw);
    let mut artists = Vec::new();

    for name in split_artist_segment(primary_segment) {
        push_unique_artist(&mut artists, name, ArtistRole::Primary);
    }

    if let Some(featured_segment) = featured_segment {
        for name in split_artist_segment(featured_segment) {
            push_unique_artist(&mut artists, name, ArtistRole::Featured);
        }
    }

    if artists.is_empty() {
        artists.push(ParsedArtist {
            name: "Unknown Artist".to_string(),
            role: ArtistRole::Primary,
            position: 0,
        });
    }

    ParsedArtistCredit {
        raw: raw.to_string(),
        artists,
    }
}

fn split_featured_segment(raw: &str) -> (&str, Option<&str>) {
    let lower = raw.to_ascii_lowercase();
    let markers = [
        " featuring ",
        " feat. ",
        " feat ",
        " ft. ",
        " ft ",
        " with ",
    ];

    markers
        .iter()
        .filter_map(|marker| lower.find(marker).map(|index| (index, marker.len())))
        .min_by_key(|(index, _)| *index)
        .map_or((raw, None), |(index, marker_len)| {
            let primary = raw[..index].trim();
            let featured = raw[index + marker_len..].trim();
            (primary, Some(featured))
        })
}

fn split_artist_segment(segment: &str) -> Vec<String> {
    let normalized = segment
        .replace(" vs. ", " & ")
        .replace(" vs ", " & ")
        .replace(" x ", " & ")
        .replace(" X ", " & ")
        .replace(" + ", " & ")
        .replace(';', "&");

    normalized
        .split(" & ")
        .flat_map(|part| part.split(" and "))
        .map(clean_artist_name)
        .filter(|name| !name.is_empty())
        .collect()
}

fn clean_artist_name(name: &str) -> String {
    name.trim()
        .trim_matches(',')
        .trim_matches('/')
        .trim_matches('-')
        .trim()
        .to_string()
}

fn push_unique_artist(artists: &mut Vec<ParsedArtist>, name: String, role: ArtistRole) {
    if artists
        .iter()
        .any(|artist| artist.name.eq_ignore_ascii_case(&name) && artist.role == role)
    {
        return;
    }
    let position = artists.iter().filter(|artist| artist.role == role).count();
    artists.push(ParsedArtist {
        name,
        role,
        position,
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_featured_artist_from_credit() {
        let parsed = parse_artist_credit("Black Atlass feat. Jessie Reyez");

        assert_eq!(parsed.primary_names(), vec!["Black Atlass"]);
        assert_eq!(
            parsed.artists,
            vec![
                ParsedArtist {
                    name: "Black Atlass".to_string(),
                    role: ArtistRole::Primary,
                    position: 0,
                },
                ParsedArtist {
                    name: "Jessie Reyez".to_string(),
                    role: ArtistRole::Featured,
                    position: 0,
                },
            ]
        );
    }

    #[test]
    fn splits_multiple_primary_artists_before_feature_marker() {
        let parsed = parse_artist_credit("Calvin Harris & Alesso feat. Hurts");

        assert_eq!(parsed.primary_names(), vec!["Calvin Harris", "Alesso"]);
        assert_eq!(
            parsed
                .artists
                .iter()
                .map(|artist| (artist.name.as_str(), artist.role, artist.position))
                .collect::<Vec<_>>(),
            vec![
                ("Calvin Harris", ArtistRole::Primary, 0),
                ("Alesso", ArtistRole::Primary, 1),
                ("Hurts", ArtistRole::Featured, 0),
            ]
        );
    }
}

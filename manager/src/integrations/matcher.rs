/// Normalizes a title for comparison by lowercasing, replacing punctuation with spaces,
/// removing non-alphanumeric characters, and collapsing whitespace.
pub fn normalize_title(input: &str) -> String {
    let mut normalized = String::with_capacity(input.len());
    let mut prev_is_space = false;

    for ch in input.chars() {
        if ch.is_alphanumeric() {
            for lower_ch in ch.to_lowercase() {
                normalized.push(lower_ch);
            }
            prev_is_space = false;
        } else if !prev_is_space {
            normalized.push(' ');
            prev_is_space = true;
        }
    }

    normalized.trim().to_string()
}

/// Strips standard video file extensions if present
pub fn strip_video_extension(name: &str) -> &str {
    const EXTENSIONS: &[&str] = &[
        ".mkv", ".mp4", ".avi", ".mov", ".m4v", ".wmv", ".flv", ".webm", ".ts",
    ];

    for ext in EXTENSIONS {
        if let Some(stripped) = name.strip_suffix(ext) {
            return stripped;
        }
        // Case-insensitive check
        if name.len() >= ext.len() {
            let suffix = &name[name.len() - ext.len()..];
            if suffix.eq_ignore_ascii_case(ext) {
                return &name[..name.len() - ext.len()];
            }
        }
    }
    name
}

/// Extracts a 4-digit year (1900..=2100) from parenthesized or bracketed suffixes,
/// or standalone 4-digit sequences.
pub fn parse_title_and_year(raw_name: &str) -> (String, Option<i32>) {
    let name = strip_video_extension(raw_name.trim());

    // 1. Check for trailing (YYYY) or [YYYY]
    if let Some(open_idx) = name.rfind(['(', '[']) {
        let after_open = &name[open_idx + 1..];
        if let Some(close_idx) = after_open.find([')', ']']) {
            let year_str = after_open[..close_idx].trim();
            if year_str.len() == 4
                && let Ok(year) = year_str.parse::<i32>()
                && (1900..=2100).contains(&year)
            {
                let title = name[..open_idx].trim().to_string();
                if !title.is_empty() {
                    return (title, Some(year));
                }
            }
        }
    }

    // 2. Check for dotted/spaced year like "Movie.Name.2024.1080p" or "Movie Name 2024"
    // Split by delimiters and find a 4-digit year
    let parts: Vec<&str> = name
        .split(['.', ' ', '_', '-'])
        .filter(|p| !p.is_empty())
        .collect();
    for (idx, part) in parts.iter().enumerate() {
        if part.len() == 4
            && let Ok(year) = part.parse::<i32>()
            && (1900..=2100).contains(&year)
        {
            // Title is all parts before the year
            if idx > 0 {
                let title = parts[..idx].join(" ");
                return (title, Some(year));
            }
        }
    }

    (name.to_string(), None)
}

/// Parses a season number from folder names like "Season 01", "Season 1", "Specials", "Season 0"
pub fn parse_season_number(folder_name: &str) -> Option<i32> {
    let trimmed = folder_name.trim();

    // Check "Specials"
    if trimmed.eq_ignore_ascii_case("specials") || trimmed.eq_ignore_ascii_case("special") {
        return Some(0);
    }

    // Check "Season XX" or "Season.XX" or "Season_XX" or "Season-XX"
    let normalized = trimmed.to_lowercase();
    if let Some(rest) = normalized.strip_prefix("season") {
        let num_str = rest.trim_matches([' ', '.', '_', '-']);
        if let Ok(num) = num_str.parse::<i32>() {
            return Some(num);
        }
    }

    // Check "S01", "S1", "S00"
    if let Some(rest) = normalized.strip_prefix('s') {
        let num_str = rest.trim_matches([' ', '.', '_', '-']);
        if let Ok(num) = num_str.parse::<i32>() {
            return Some(num);
        }
    }

    None
}

/// Matches an item name against Radarr movie details
pub fn matches_movie(
    item_name: &str,
    movie_title: &str,
    movie_clean_title: Option<&str>,
    movie_year: i32,
    movie_path: Option<&str>,
) -> bool {
    let clean_item = strip_video_extension(item_name.trim());

    // 1. Direct path basename check
    if let Some(path) = movie_path {
        let path_buf = std::path::Path::new(path);
        if let Some(file_name) = path_buf.file_name().and_then(|f| f.to_str())
            && (file_name.eq_ignore_ascii_case(clean_item)
                || normalize_title(file_name) == normalize_title(clean_item))
        {
            return true;
        }
    }

    // 2. Parse title and year from item_name
    let (parsed_title, parsed_year) = parse_title_and_year(clean_item);

    let norm_parsed = normalize_title(&parsed_title);
    let norm_movie_title = normalize_title(movie_title);

    let title_matches = norm_parsed == norm_movie_title
        || movie_clean_title.is_some_and(|c| norm_parsed == normalize_title(c));

    if title_matches {
        if let Some(year) = parsed_year {
            return year == movie_year;
        }
        return true;
    }

    false
}

/// Matches an item name against Sonarr series details
pub fn matches_series(
    item_name: &str,
    series_title: &str,
    series_clean_title: Option<&str>,
    series_year: Option<i32>,
    series_path: Option<&str>,
) -> bool {
    let clean_item = strip_video_extension(item_name.trim());

    // 1. Direct path basename check
    if let Some(path) = series_path {
        let path_buf = std::path::Path::new(path);
        if let Some(file_name) = path_buf.file_name().and_then(|f| f.to_str())
            && (file_name.eq_ignore_ascii_case(clean_item)
                || normalize_title(file_name) == normalize_title(clean_item))
        {
            return true;
        }
    }

    // 2. Parse title and year from item_name
    let (parsed_title, parsed_year) = parse_title_and_year(clean_item);

    let norm_parsed = normalize_title(&parsed_title);
    let norm_series_title = normalize_title(series_title);

    let title_matches = norm_parsed == norm_series_title
        || series_clean_title.is_some_and(|c| norm_parsed == normalize_title(c));

    if title_matches {
        if let (Some(py), Some(sy)) = (parsed_year, series_year) {
            return py == sy;
        }
        return true;
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_title() {
        assert_eq!(normalize_title("Dune: Part Two"), "dune part two");
        assert_eq!(normalize_title("The.Batman.2022"), "the batman 2022");
        assert_eq!(normalize_title("Severance (2022)"), "severance 2022");
        assert_eq!(normalize_title("  Breaking   Bad  "), "breaking bad");
    }

    #[test]
    fn test_parse_title_and_year() {
        let (title, year) = parse_title_and_year("Inception (2010)");
        assert_eq!(title, "Inception");
        assert_eq!(year, Some(2010));

        let (title, year) = parse_title_and_year("Dune: Part Two (2024)");
        assert_eq!(title, "Dune: Part Two");
        assert_eq!(year, Some(2024));

        let (title, year) = parse_title_and_year("The.Batman.2022.1080p.mkv");
        assert_eq!(title, "The Batman");
        assert_eq!(year, Some(2022));

        let (title, year) = parse_title_and_year("Severance");
        assert_eq!(title, "Severance");
        assert_eq!(year, None);
    }

    #[test]
    fn test_parse_season_number() {
        assert_eq!(parse_season_number("Season 01"), Some(1));
        assert_eq!(parse_season_number("Season 1"), Some(1));
        assert_eq!(parse_season_number("Season 02"), Some(2));
        assert_eq!(parse_season_number("Season 10"), Some(10));
        assert_eq!(parse_season_number("season.03"), Some(3));
        assert_eq!(parse_season_number("Season_04"), Some(4));
        assert_eq!(parse_season_number("Specials"), Some(0));
        assert_eq!(parse_season_number("Special"), Some(0));
        assert_eq!(parse_season_number("Season 00"), Some(0));
        assert_eq!(parse_season_number("S01"), Some(1));
        assert_eq!(parse_season_number("S2"), Some(2));
        assert_eq!(parse_season_number("Random Folder"), None);
    }

    #[test]
    fn test_matches_movie() {
        assert!(matches_movie(
            "Inception (2010)",
            "Inception",
            None,
            2010,
            Some("/data/movies/Inception (2010)")
        ));

        assert!(matches_movie(
            "Dune: Part Two (2024)",
            "Dune: Part Two",
            Some("dune part two"),
            2024,
            Some("/data/movies/Dune Part Two (2024)")
        ));

        assert!(matches_movie(
            "The.Batman.2022.1080p.mkv",
            "The Batman",
            None,
            2022,
            None
        ));

        assert!(!matches_movie(
            "Inception (2011)",
            "Inception",
            None,
            2010,
            None
        ));
    }

    #[test]
    fn test_matches_series() {
        assert!(matches_series(
            "Severance",
            "Severance",
            None,
            Some(2022),
            Some("/data/tv/Severance")
        ));

        assert!(matches_series(
            "Severance (2022)",
            "Severance",
            None,
            Some(2022),
            Some("/data/tv/Severance (2022)")
        ));

        assert!(matches_series(
            "Breaking.Bad",
            "Breaking Bad",
            None,
            Some(2008),
            None
        ));
    }
}

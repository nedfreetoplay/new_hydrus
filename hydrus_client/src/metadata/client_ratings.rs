//! Ratings from `hydrus/client/metadata/ClientRatings.py`.

use std::collections::BTreeMap;

pub const LIKE: i32 = 0;
pub const DISLIKE: i32 = 1;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Rating {
    Like,
    Dislike,
    Numerical(f64),
}

impl Rating {
    pub fn value(&self) -> f64 {
        match self {
            Self::Like => 1.0,
            Self::Dislike => 0.0,
            Self::Numerical(value) => *value,
        }
    }

    pub fn is_like(&self) -> bool {
        matches!(self, Self::Like)
    }

    pub fn is_dislike(&self) -> bool {
        matches!(self, Self::Dislike)
    }

    pub fn as_int(&self) -> Option<i32> {
        match self {
            Self::Like => Some(LIKE),
            Self::Dislike => Some(DISLIKE),
            Self::Numerical(_) => None,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct RatingsManager {
    ratings: BTreeMap<Vec<u8>, Rating>,
}

impl RatingsManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_rating(&mut self, service_key: impl Into<Vec<u8>>, rating: Rating) {
        self.ratings.insert(service_key.into(), rating);
    }

    pub fn get_rating(&self, service_key: &[u8]) -> Option<Rating> {
        self.ratings.get(service_key).copied()
    }

    pub fn remove_rating(&mut self, service_key: &[u8]) {
        self.ratings.remove(service_key);
    }

    pub fn has_rating(&self, service_key: &[u8]) -> bool {
        self.ratings.contains_key(service_key)
    }

    /// `GetLikeStateFromRating`: `1` like, `-1` dislike, `0` otherwise.
    pub fn get_like_state_from_rating(rating: Option<Rating>) -> i32 {
        match rating {
            Some(Rating::Like) => 1,
            Some(Rating::Dislike) => -1,
            _ => 0,
        }
    }

    pub fn duplicate(&self) -> Self {
        self.clone()
    }

    pub fn ratings(&self) -> &BTreeMap<Vec<u8>, Rating> {
        &self.ratings
    }
}

/// `ConvertRatingToStars( num_stars, allow_zero, rating )`
pub fn convert_rating_to_stars(num_stars: i32, allow_zero: bool, rating: f64) -> i32 {
    if rating <= 0.0 && allow_zero {
        return 0;
    }

    let stars = (rating * num_stars as f64).round() as i32;

    stars.clamp(1, num_stars)
}

/// `ConvertStarsToRating( num_stars, allow_zero, stars )`
pub fn convert_stars_to_rating(num_stars: i32, allow_zero: bool, stars: i32) -> f64 {
    if allow_zero && stars == 0 {
        return 0.0;
    }

    let denominator = num_stars.max(1) as f64;

    (stars.clamp(if allow_zero { 0 } else { 1 }, num_stars) as f64 / denominator).clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn like_and_dislike_have_fixed_values() {
        assert_eq!(Rating::Like.value(), 1.0);
        assert_eq!(Rating::Dislike.value(), 0.0);
        assert_eq!(Rating::Numerical(0.5).value(), 0.5);
    }

    #[test]
    fn manager_stores_ratings_per_service() {
        let mut manager = RatingsManager::new();
        manager.set_rating(b"my ratings", Rating::Numerical(0.75));

        assert_eq!(
            manager.get_rating(b"my ratings"),
            Some(Rating::Numerical(0.75))
        );
        assert!(!manager.has_rating(b"other ratings"));
    }

    #[test]
    fn like_state_matches_python() {
        assert_eq!(
            RatingsManager::get_like_state_from_rating(Some(Rating::Like)),
            1
        );
        assert_eq!(
            RatingsManager::get_like_state_from_rating(Some(Rating::Dislike)),
            -1
        );
        assert_eq!(RatingsManager::get_like_state_from_rating(None), 0);
    }

    #[test]
    fn stars_round_trip() {
        assert_eq!(convert_rating_to_stars(5, false, 0.5), 3);
        assert_eq!(convert_rating_to_stars(5, true, 0.0), 0);
        assert_eq!(convert_stars_to_rating(5, false, 3), 0.6);
        assert_eq!(convert_stars_to_rating(5, true, 0), 0.0);
    }
}

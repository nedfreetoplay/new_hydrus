use super::*;

#[test]
fn adding_a_favourite_replaces_the_same_name_and_sorts() {
    let mut favourites = Vec::new();

    add_or_replace(
        &mut favourites,
        FavouriteSearch {
            name: "zebra".to_string(),
            query: "zebra".to_string(),
            location: "my files".to_string(),
            tag_service: String::new(),
        },
    );
    add_or_replace(
        &mut favourites,
        FavouriteSearch {
            name: "alpha".to_string(),
            query: "alpha".to_string(),
            location: "trash".to_string(),
            tag_service: "local tags".to_string(),
        },
    );
    add_or_replace(
        &mut favourites,
        FavouriteSearch {
            name: "zebra".to_string(),
            query: "zebra system:inbox".to_string(),
            location: "my files".to_string(),
            tag_service: String::new(),
        },
    );

    assert_eq!(favourites.len(), 2);
    assert_eq!(favourites[0].name, "alpha");
    assert_eq!(favourites[1].query, "zebra system:inbox");
}

#[test]
fn removing_a_favourite_reports_whether_it_existed() {
    let mut favourites = vec![FavouriteSearch {
        name: "one".to_string(),
        query: String::new(),
        location: "my files".to_string(),
        tag_service: String::new(),
    }];

    assert!(remove(&mut favourites, "one"));
    assert!(!remove(&mut favourites, "one"));
    assert!(favourites.is_empty());
}

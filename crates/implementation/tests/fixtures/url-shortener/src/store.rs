//! UrlShortener::LinkStore: keeps short links and answers queries and
//! status changes. (The model keeps one link, enough for each scenario;
//! the code keeps any number.)

use crate::model::{LinkQuery, LinkRecord, ShortLink, StatusChange};
use crate::ports::LinkStorePort;
use std::collections::BTreeMap;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct LinkStore {
    links: BTreeMap<String, ShortLink>,
}

impl LinkStorePort for LinkStore {
    fn save(&mut self, link: ShortLink) {
        self.links.insert(link.code.clone(), link);
    }

    fn query(&mut self, query: LinkQuery) -> LinkRecord {
        LinkRecord {
            link: self.links.get(&query.code).cloned(),
            code: query.code,
        }
    }

    fn change(&mut self, change: StatusChange) -> LinkRecord {
        let link = self.links.get_mut(&change.code).map(|link| {
            link.status = change.status;
            link.clone()
        });
        LinkRecord {
            code: change.code,
            link,
        }
    }
}

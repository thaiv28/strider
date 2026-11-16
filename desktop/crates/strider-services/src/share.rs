//! Clients for the signed-in `/api/v1` routes: Trip share links, geocoding, and food search.
//! These need a device session and a connection; failures map to `Offline` / `Unauthorized`.
use crate::{Result, ServiceError};
use serde::Deserialize;
use serde_json::{Value, json};
use strider_sync::Transport;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LinkKind {
    View,
    Edit,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LinkAction {
    Create,
    Rotate,
    Revoke,
}
impl LinkKind {
    fn name(self) -> &'static str {
        match self {
            Self::View => "view",
            Self::Edit => "edit",
        }
    }
}
impl LinkAction {
    fn name(self) -> &'static str {
        match self {
            Self::Create => "create",
            Self::Rotate => "rotate",
            Self::Revoke => "revoke",
        }
    }
}
/// A live share link. The URL is the one the web share menu shows: `<origin>/share/trips/<token>`,
/// for both view and edit links. Treat it as a secret.
#[derive(Clone, PartialEq, Eq)]
pub struct ShareLink {
    pub token: String,
    pub url: String,
}
impl std::fmt::Debug for ShareLink {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ShareLink").finish_non_exhaustive()
    }
}

fn is_uuid(s: &str) -> bool {
    let groups: Vec<_> = s.split('-').map(str::len).collect();
    groups == [8, 4, 4, 4, 12] && s.bytes().all(|b| b == b'-' || b.is_ascii_hexdigit())
}

/// Create, rotate or revoke a Trip's view or edit link. Returns the link, or `None` after a
/// revoke. `create` returns the existing link when there is one.
pub fn change_share_link(
    transport: &dyn Transport,
    trip_sync_id: &str,
    kind: LinkKind,
    action: LinkAction,
) -> Result<Option<ShareLink>> {
    if !is_uuid(trip_sync_id) {
        return Err(ServiceError::Protocol("Invalid trip id".into()));
    }
    let body = json!({ "kind": kind.name(), "action": action.name() });
    let reply = transport.request(
        "POST",
        &format!("/api/v1/trips/{trip_sync_id}/share"),
        Some(&body),
    )?;
    match reply.get("token") {
        Some(Value::String(token)) if action != LinkAction::Revoke => Ok(Some(ShareLink {
            url: format!("{}/share/trips/{token}", transport.origin()),
            token: token.clone(),
        })),
        Some(Value::Null) if action == LinkAction::Revoke => Ok(None),
        _ => Err(ServiceError::Protocol("Invalid share response".into())),
    }
}

fn get(transport: &dyn Transport, path: &str, params: &[(&str, &str)]) -> Result<Value> {
    let query = url::form_urlencoded::Serializer::new(String::new())
        .extend_pairs(params)
        .finish();
    Ok(transport.request("GET", &format!("{path}?{query}"), None)?)
}

/// `GET /api/v1/geocode`. The server route is not implemented yet and answers 501, which maps to
/// `NotImplemented`; the reply is returned as JSON once it is.
pub fn geocode(transport: &dyn Transport, query: &str) -> Result<Value> {
    get(transport, "/api/v1/geocode", &[("q", query)])
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FoodHit {
    #[serde(default)]
    pub reference: Option<bool>,
    #[serde(default)]
    pub reference_name: Option<String>,
    pub source: String,
    pub source_id: String,
    pub name: String,
    #[serde(default)]
    pub detail: Option<String>,
    #[serde(default)]
    pub kcal_per100g: Option<f64>,
    #[serde(default)]
    pub default_serving_g: Option<f64>,
    #[serde(default)]
    pub density_g_ml: Option<f64>,
}

/// `GET /api/v1/food/search`: USDA reference foods, or packaged products with `packaged`.
pub fn food_search(transport: &dyn Transport, query: &str, packaged: bool) -> Result<Vec<FoodHit>> {
    let q: String = query.chars().take(200).collect();
    let mut params = vec![("q", q.as_str())];
    if packaged {
        params.push(("packaged", "1"));
    }
    let reply = get(transport, "/api/v1/food/search", &params)?;
    serde_json::from_value(reply)
        .map_err(|_| ServiceError::Protocol("Invalid food search response".into()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;
    use strider_sync::SyncError;

    const TRIP: &str = "0192f3a4-5b6c-7d8e-9f01-23456789abcd";
    const FOOD: &str = include_str!("../../../fixtures/http/food-search.json");

    struct Fake {
        calls: Mutex<Vec<(String, String, Option<Value>)>>,
        reply: Mutex<std::result::Result<Value, SyncError>>,
    }
    impl Fake {
        fn replying(v: Value) -> Self {
            Self {
                calls: Mutex::default(),
                reply: Mutex::new(Ok(v)),
            }
        }
        fn failing(e: SyncError) -> Self {
            Self {
                calls: Mutex::default(),
                reply: Mutex::new(Err(e)),
            }
        }
    }
    impl Transport for Fake {
        fn origin(&self) -> String {
            "https://strider.example".into()
        }
        fn request(
            &self,
            method: &str,
            path: &str,
            body: Option<&Value>,
        ) -> strider_sync::Result<Value> {
            self.calls
                .lock()
                .unwrap()
                .push((method.into(), path.into(), body.cloned()));
            match &*self.reply.lock().unwrap() {
                Ok(v) => Ok(v.clone()),
                Err(SyncError::Offline) => Err(SyncError::Offline),
                Err(SyncError::Unauthorized) => Err(SyncError::Unauthorized),
                Err(SyncError::Server(s)) => Err(SyncError::Server(*s)),
                Err(_) => Err(SyncError::Protocol("x".into())),
            }
        }
        fn upload_permit(
            &self,
            _: &str,
            _: &str,
            _: &str,
            _: &[u8],
        ) -> strider_sync::Result<String> {
            unreachable!()
        }
        fn download_permit(&self, _: &str) -> strider_sync::Result<Vec<u8>> {
            unreachable!()
        }
    }

    #[test]
    fn creating_a_link_posts_kind_and_action_and_builds_the_web_url() {
        let t = Fake::replying(json!({"token": "abc_DEF-123"}));
        let link = change_share_link(&t, TRIP, LinkKind::View, LinkAction::Create)
            .unwrap()
            .unwrap();
        assert_eq!(link.url, "https://strider.example/share/trips/abc_DEF-123");
        let calls = t.calls.lock().unwrap();
        assert_eq!(calls[0].0, "POST");
        assert_eq!(calls[0].1, format!("/api/v1/trips/{TRIP}/share"));
        assert_eq!(
            calls[0].2,
            Some(json!({"kind": "view", "action": "create"}))
        );
        assert!(
            !format!("{link:?}").contains("abc"),
            "debug output must not leak the token"
        );
    }
    #[test]
    fn edit_links_use_the_same_url_shape_and_rotate_is_supported() {
        let t = Fake::replying(json!({"token": "tok"}));
        let link = change_share_link(&t, TRIP, LinkKind::Edit, LinkAction::Rotate)
            .unwrap()
            .unwrap();
        assert_eq!(link.url, "https://strider.example/share/trips/tok");
        assert_eq!(
            t.calls.lock().unwrap()[0].2,
            Some(json!({"kind": "edit", "action": "rotate"}))
        );
    }
    #[test]
    fn revoking_returns_none_and_rejects_unexpected_replies() {
        let t = Fake::replying(json!({"token": null}));
        assert_eq!(
            change_share_link(&t, TRIP, LinkKind::View, LinkAction::Revoke).unwrap(),
            None
        );
        let t = Fake::replying(json!({"token": "still-live"}));
        assert!(change_share_link(&t, TRIP, LinkKind::View, LinkAction::Revoke).is_err());
        let t = Fake::replying(json!({}));
        assert!(change_share_link(&t, TRIP, LinkKind::View, LinkAction::Create).is_err());
    }
    #[test]
    fn rejects_malformed_trip_ids_without_a_request() {
        let t = Fake::replying(json!({"token": "x"}));
        for bad in ["", "../etc", "123", &format!("{TRIP}/x")] {
            assert!(change_share_link(&t, bad, LinkKind::View, LinkAction::Create).is_err());
        }
        assert!(t.calls.lock().unwrap().is_empty());
    }
    #[test]
    fn errors_map_to_offline_and_unauthorized() {
        let offline = Fake::failing(SyncError::Offline);
        assert_eq!(
            change_share_link(&offline, TRIP, LinkKind::View, LinkAction::Create).unwrap_err(),
            ServiceError::Offline
        );
        let signed_out = Fake::failing(SyncError::Unauthorized);
        assert_eq!(
            food_search(&signed_out, "oats", false).unwrap_err(),
            ServiceError::Unauthorized
        );
        let missing = Fake::failing(SyncError::Server(404));
        assert_eq!(
            change_share_link(&missing, TRIP, LinkKind::View, LinkAction::Create).unwrap_err(),
            ServiceError::Server(404)
        );
    }
    #[test]
    fn geocode_maps_the_unimplemented_route() {
        let t = Fake::failing(SyncError::Server(501));
        assert_eq!(
            geocode(&t, "Leadville, CO").unwrap_err(),
            ServiceError::NotImplemented
        );
        assert_eq!(
            t.calls.lock().unwrap()[0].1,
            "/api/v1/geocode?q=Leadville%2C+CO"
        );
        let t = Fake::replying(json!({"results": []}));
        assert_eq!(geocode(&t, "x").unwrap(), json!({"results": []}));
    }
    #[test]
    fn food_search_encodes_the_query_and_parses_hits() {
        let t = Fake::replying(serde_json::from_str(FOOD).unwrap());
        let hits = food_search(&t, "rolled oats & honey", true).unwrap();
        assert_eq!(
            t.calls.lock().unwrap()[0].1,
            "/api/v1/food/search?q=rolled+oats+%26+honey&packaged=1"
        );
        assert_eq!(hits.len(), 2);
        assert_eq!(hits[0].name, "Oats, regular and quick, dry");
        assert_eq!(hits[0].reference, Some(true));
        assert_eq!(hits[0].kcal_per100g, Some(379.0));
        assert_eq!(
            (hits[1].source.as_str(), hits[1].detail.as_deref()),
            ("off", Some("Bob's Red Mill"))
        );
        assert_eq!(hits[1].density_g_ml, None);
        let long = "x".repeat(300);
        food_search(&t, &long, false).unwrap();
        let calls = t.calls.lock().unwrap();
        assert_eq!(calls[1].1.len(), "/api/v1/food/search?q=".len() + 200);
    }
}

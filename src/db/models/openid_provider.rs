use model_derive::Model;

// TODO(jck): maybe rename OpenIdProvider
#[derive(Deserialize, Model, Serialize)]
pub struct OpenIdProvider {
    pub id: Option<i64>,
    pub name: String,
    // pub client_id: String, // unique
    // // TODO(jck): maybe remove since we get the id_token in the first reponse?
    // pub client_secret: String,
    // pub auth_url: String,
    // TODO(jck): provider image?

    // // TODO(jck): do we need this?
    // #[model(ref)]
    // pub redirect_uri: Vec<String>,
    // // TODO(jck): can we assume constant scope ahead of time?
    // #[model(ref)]
    // pub scope: Vec<String>,
    // // TODO(jck): remove?
    // // informational
    // pub name: String,
    // pub enabled: bool,
}

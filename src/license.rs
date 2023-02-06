use chrono::NaiveDate;

/// Decoded license information
/// Important: order must be preserved for bincode.
#[derive(Serialize, Deserialize, Debug)]
pub struct License {
    pub company: String,
    pub expiration: NaiveDate,
    pub ldap: bool,
    pub openid: bool,
    pub oauth: bool, // obsolete, but needed for bincode
    pub worker: bool,
    pub enterprise: bool,
}

#[allow(dead_code)]
pub(crate) const PUBLIC_KEY: &str = "-----BEGIN PUBLIC KEY-----
MIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEAoowOenhBJnaS5C/W9kHX
Vz6LQYUXczT1BasE+ehy53LWnj5nPD98J0/h3mUNrYcr28qKfj8MVNBDcvzRDCx2
eVyXoEVffDLaMUU4rqNmIirOOm+Epwiln31Mwhi2G6RS+oHJsEprSoaZSa4GEtLk
YkzPAWoKLfQktwc6AeQp8p2Y+IUnVhIlkiVY+xyTMvMyRzcyFAG1t9fFdOuuCB2Q
vjkIF3OO93WiqSr13Un6U9kKz94p7JouXPBH3KlfbyNpXPkyFVzUD7b9cS8tnz9E
gKOzxk9Guyyj4IwwnBFCanSJR5bey3Cm3vi1QnwAVSQ5I8mqCHu75TfamIBWfVsI
jQIDAQAB
-----END PUBLIC KEY-----
";

#[derive(Serialize)]
pub enum Features {
    Ldap,
    Worker,
    Openid,
}

impl Default for License {
    /// Create default license
    #[must_use]
    fn default() -> Self {
        Self {
            company: "default".into(),
            expiration: NaiveDate::from_ymd_opt(2100, 1, 1).unwrap_or_default(),
            ldap: true,
            oauth: true,
            openid: true,
            worker: true,
            enterprise: true,
        }
    }
}

impl License {
    #[allow(dead_code)]
    fn get(&self, feature: &Features) -> bool {
        match feature {
            Features::Ldap => self.ldap,
            Features::Openid => self.openid,
            Features::Worker => self.worker,
        }
    }

    #[must_use]
    pub fn validate(&self, _feature: &Features) -> bool {
        true
        // // Old license validation
        // if self.expiration < Utc::now().naive_utc().date() {
        //     info!("License expired");
        //     false
        // } else {
        //     self.enterprise || self.get(feature)
        // }
    }

    // Enterprise license enables all features.
    #[allow(dead_code)]
    fn sanitize(&mut self) {
        if self.enterprise {
            self.worker = true;
            self.ldap = true;
            self.openid = true;
        }
    }

    /// decode encoded license string to License instance
    #[must_use]
    pub fn decode(_license: &str) -> Self {
        Self::default()
        // debug!("Checking license");
        // if !license.is_empty() {
        //     // Verify the signature.
        //     let public_key = RsaPublicKey::from_public_key_pem(PUBLIC_KEY).unwrap();
        //     let padding = PaddingScheme::new_pkcs1v15_sign_raw();
        //     let license_decoded = match base64::decode(license) {
        //         Ok(license_decoded) => license_decoded,
        //         Err(e) => {
        //             error!("Error decoding license. Using community features: {}", e);
        //             return Self::default();
        //         }
        //     };
        //     let len = license_decoded.len();
        //     if let Ok(()) = public_key.verify(
        //         padding,
        //         &license_decoded[..len - 256],
        //         &license_decoded[len - 256..],
        //     ) {
        //         match bincode::deserialize::<Self>(&license_decoded[..]) {
        //             Ok(mut license) => {
        //                 license.sanitize();
        //                 info!("License validation successful: {:?}", license);
        //                 return license;
        //             }
        //             Err(e) => {
        //                 error!(
        //                     "Error deserializing license: {}. Using community features.",
        //                     e
        //                 );
        //             }
        //         }
        //     } else {
        //         error!("Invalid license signature. Using community features");
        //     }
        // } else {
        //     info!("No license supplied. Using community features");
        // }
        // Self::default()
    }
}

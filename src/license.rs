use base64;
use chrono::{NaiveDate, Utc};
use rsa::{pkcs8::FromPublicKey, PaddingScheme, PublicKey, RsaPublicKey};

/// Decoded license information
#[derive(Serialize, Deserialize, Debug)]
pub struct License {
    pub company: String,
    pub expiration: NaiveDate,
    pub ldap: bool,
    pub openid: bool,
    pub oauth: bool,
    pub worker: bool,
    pub enterprise: bool,
}

#[cfg(feature = "mock-license-key")]
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

#[cfg(not(feature = "mock-license-key"))]
pub(crate) const PUBLIC_KEY: &str = "-----BEGIN PUBLIC KEY-----
MIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEApI/JdghL3uSNqRbFwAv3
s5tQQKfqL60srY6uaxng4dtpt0juWIhdzhoDEwUqJL8RA7mIRxJZ+FrgwrHm6Q7a
GI1TCKL+7QEjgNRlemtb9LeVo1eK3SVpV3UnXLAOTXnWXZanYcPYDp4MpflTUAIN
/iTCtjwn+0piSCXgj2qlmMiDQfTWcBQgSimDSYN1MXi74OczEnKtEt9WuMfluAib
t08etN/WX8S/FAiWicyL84Ol5htk1iLPwaP8FfAEvmpMY7obXATbBx+HNk8Zd1TU
1jbEqXTQn9RNLAZBwyMs4EeuuvzKgbOvsEyLTOEy9n7VtShG8X5VqFrPGuDmYTvS
7QIDAQAB
-----END PUBLIC KEY-----
";

#[derive(Serialize)]
pub enum Features {
    Ldap,
    Worker,
    Openid,
    Oauth,
}

impl License {
    /// Create default community license in case no license supplied
    #[must_use]
    pub fn default() -> Self {
        Self {
            company: "community".into(),
            expiration: NaiveDate::from_ymd_opt(2100, 1, 1).unwrap_or_default(),
            worker: false,
            ldap: false,
            oauth: false,
            openid: false,
            enterprise: false,
        }
    }

    fn get(&self, feature: &Features) -> bool {
        match feature {
            Features::Ldap => self.ldap,
            Features::Openid => self.openid,
            Features::Oauth => self.oauth,
            Features::Worker => self.worker,
        }
    }

    pub fn validate(&self, feature: &Features) -> bool {
        if self.expiration < Utc::now().naive_utc().date() {
            info!("License expired");
            false
        } else {
            self.enterprise || self.get(feature)
        }
    }

    // Enterprise license enables all features.
    fn sanitize(&mut self) {
        if self.enterprise {
            self.worker = true;
            self.ldap = true;
            self.oauth = true;
            self.openid = true;
        }
    }

    /// decode encoded license string to License instance
    pub fn decode(license: &str) -> Self {
        debug!("Checking license");
        if !license.is_empty() {
            // Verify the signature.
            let public_key = RsaPublicKey::from_public_key_pem(PUBLIC_KEY).unwrap();
            let padding = PaddingScheme::new_pkcs1v15_sign(None);
            let license_decoded = match base64::decode(license) {
                Ok(license_decoded) => license_decoded,
                Err(e) => {
                    error!("Error decoding license. Using community features: {}", e);
                    return Self::default();
                }
            };
            let len = license_decoded.len();
            if let Ok(()) = public_key.verify(
                padding,
                &license_decoded[..len - 256],
                &license_decoded[len - 256..],
            ) {
                match bincode::deserialize::<Self>(&license_decoded[..]) {
                    Ok(mut license) => {
                        license.sanitize();
                        info!("License validation successful: {:?}", license);
                        return license;
                    }
                    Err(e) => {
                        error!(
                            "Error deserializing license: {}. Using community features.",
                            e
                        );
                    }
                }
            } else {
                error!("Invalid license signature. Using community features");
            }
        } else {
            info!("No license supplied. Using community features");
        }
        Self::default()
    }
}

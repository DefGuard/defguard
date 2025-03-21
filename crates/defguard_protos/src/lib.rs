pub mod proto {
    pub mod auth {
        tonic::include_proto!("auth");
    }
    pub mod proxy {
        tonic::include_proto!("defguard.proxy");
    }
    pub mod gateway {
        tonic::include_proto!("gateway");
    }
    pub mod worker {
        tonic::include_proto!("worker");
    }
    pub mod enterprise {
        pub mod firewall {
            tonic::include_proto!("enterprise.firewall");
        }
    }
}

use crate::proto::proxy::CoreError;
use tonic::Status;

impl From<Status> for CoreError {
    fn from(status: Status) -> Self {
        Self {
            status_code: status.code().into(),
            message: status.message().into(),
        }
    }
}

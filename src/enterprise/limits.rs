use sqlx::{error::Error as SqlxError, query_as, PgPool};
use std::{
    sync::{RwLock, RwLockReadGuard},
    time::Duration,
};
use tokio::time::sleep;

#[derive(Debug)]
pub(crate) struct Counts {
    user: i64,
    device: i64,
    wireguard_network: i64,
}

static COUNTS: RwLock<Counts> = RwLock::new(Counts {
    user: 0,
    device: 0,
    wireguard_network: 0,
});

fn set_counts(new_counts: Counts) {
    *COUNTS
        .write()
        .expect("Failed to acquire lock on the enterprise limit counts.") = new_counts;
}

pub(crate) fn get_counts() -> RwLockReadGuard<'static, Counts> {
    COUNTS
        .read()
        .expect("Failed to acquire lock on the enterprise limit counts.")
}

/// Update the counts of users, devices, and wireguard networks stored in the memory.
// TODO: Use it with database triggers when they are implemented
pub async fn update_counts(pool: &PgPool) -> Result<(), SqlxError> {
    debug!("Updating device, user, and wireguard network counts.");
    let counts = query_as!(
        Counts,
        "SELECT \
        (SELECT count(*) FROM \"user\") \"user!\", \
        (SELECT count(*) FROM device) \"device!\", \
        (SELECT count(*) FROM wireguard_network) \"wireguard_network!\"
        "
    )
    .fetch_one(pool)
    .await?;

    set_counts(counts);
    debug!(
        "Updated device, user, and wireguard network counts stored in memory, new counts: {:?}",
        get_counts()
    );

    Ok(())
}

// Just to make sure we don't miss any user/device/network count changes
pub async fn run_periodic_count_update(pool: &PgPool) -> Result<(), SqlxError> {
    let delay = Duration::from_secs(60 * 60);
    loop {
        update_counts(pool).await?;
        sleep(delay).await;
    }
}

impl Counts {
    pub(crate) fn is_over_limit(&self) -> bool {
        self.user > 5 || self.device > 10 || self.wireguard_network > 1
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_counts() {
        let counts = Counts {
            user: 1,
            device: 2,
            wireguard_network: 3,
        };

        set_counts(counts);

        let counts = get_counts();

        assert_eq!(counts.user, 1);
        assert_eq!(counts.device, 2);
        assert_eq!(counts.wireguard_network, 3);
    }

    #[test]
    fn test_is_over_limit() {
        // User limit
        {
            let counts = Counts {
                user: 6,
                device: 1,
                wireguard_network: 1,
            };
            set_counts(counts);
            let counts = get_counts();
            assert!(counts.is_over_limit());
        }

        // Device limit
        {
            let counts = Counts {
                user: 1,
                device: 11,
                wireguard_network: 1,
            };
            set_counts(counts);
            let counts = get_counts();
            assert!(counts.is_over_limit());
        }

        // Wireguard network limit
        {
            let counts = Counts {
                user: 1,
                device: 1,
                wireguard_network: 2,
            };
            set_counts(counts);
            let counts = get_counts();
            assert!(counts.is_over_limit());
        }

        // No limit
        {
            let counts = Counts {
                user: 1,
                device: 1,
                wireguard_network: 1,
            };
            set_counts(counts);
            let counts = get_counts();
            assert!(!counts.is_over_limit());
        }

        // All limits
        {
            let counts = Counts {
                user: 6,
                device: 11,
                wireguard_network: 2,
            };
            set_counts(counts);
            let counts = get_counts();
            assert!(counts.is_over_limit());
        }
    }
}

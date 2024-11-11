use sqlx::{error::Error as SqlxError, query, PgPool};
use std::sync::{RwLock, RwLockReadGuard};

struct Counts {
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
        .expect("Failed to acquire lock on the enterprise limit counts mutex.") = new_counts;
}

fn get_counts() -> RwLockReadGuard<'static, Counts> {
    COUNTS
        .read()
        .expect("Failed to acquire lock on the enterprise limit counts mutex.")
}

pub async fn update_counts(pool: &PgPool) -> Result<(), SqlxError> {
    let counts = query!(
        r#"
        select
            (select count(*) from "user") as user_count,
            (select count(*) from device) as device_count,
            (select count(*) from wireguard_network) as wireguard_network_count
    "#
    )
    .fetch_one(pool)
    .await?;

    set_counts(Counts {
        user: counts.user_count.unwrap_or(0),
        device: counts.device_count.unwrap_or(0),
        wireguard_network: counts.wireguard_network_count.unwrap_or(0),
    });

    Ok(())
}

pub(crate) fn is_over_limit() -> bool {
    let counts = get_counts();
    counts.user > 5 || counts.device > 10 || counts.wireguard_network > 1
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
        let counts = Counts {
            user: 6,
            device: 1,
            wireguard_network: 1,
        };
        set_counts(counts);
        assert!(is_over_limit());

        // Device limit
        let counts = Counts {
            user: 1,
            device: 11,
            wireguard_network: 1,
        };
        set_counts(counts);
        assert!(is_over_limit());

        // Wireguard network limit
        let counts = Counts {
            user: 1,
            device: 1,
            wireguard_network: 2,
        };
        set_counts(counts);
        assert!(is_over_limit());

        // No limit
        let counts = Counts {
            user: 1,
            device: 1,
            wireguard_network: 1,
        };
        set_counts(counts);
        assert!(!is_over_limit());

        // All limits
        let counts = Counts {
            user: 6,
            device: 11,
            wireguard_network: 2,
        };
        set_counts(counts);
        assert!(is_over_limit());
    }
}

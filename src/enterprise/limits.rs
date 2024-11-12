use sqlx::{error::Error as SqlxError, query, PgPool};
use std::sync::{RwLock, RwLockReadGuard};

#[derive(Debug)]
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

/// Update the counts of users, devices, and wireguard networks stored in the memory.
// TODO: Use it with database triggers when they are implemented
pub async fn update_counts(pool: &PgPool) -> Result<(), SqlxError> {
    debug!("Updating device, user, and wireguard network counts.");
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

    let (user, device, wireguard_network) = if counts.user_count.is_none()
        || counts.device_count.is_none()
        || counts.wireguard_network_count.is_none()
    {
        return Err(SqlxError::RowNotFound);
    } else {
        (
            counts.user_count.unwrap(),
            counts.device_count.unwrap(),
            counts.wireguard_network_count.unwrap(),
        )
    };

    set_counts(Counts {
        user,
        device,
        wireguard_network,
    });
    debug!(
        "Updated device, user, and wireguard network counts stored in memory, new counts: {:?}",
        get_counts()
    );

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

//! Player accounts (the dedicated server's, `--accounts FILE`): an SQLite database of the
//! players who registered, their XP and their stats. A player logs in (or registers) before
//! playing. The game never sends the password itself: the client sends a secret derived from
//! it and the account name (`ps_net::protocol::login_secret`), and the database keeps only an
//! Argon2 hash of that.
//!
//! Stats are counters by key ("kills", "deaths", "kills:Pulse Rifle", ...), added to as they
//! happen (`add_stats`).

/// What the database says of a player who logged in.
#[derive(Clone, Debug, PartialEq)]
pub struct Account {
    pub id: i64,
    /// The name as registered.
    pub name: String,
    pub xp: u64,
}

/// Whether `name` will do for an account: 3 to 24 letters, digits, spaces, `-` or `_`, not
/// starting or ending with a space.
pub fn valid_name(name: &str) -> bool {
    let n = name.chars().count();
    (3..=24).contains(&n)
        && name.trim() == name
        && name
            .chars()
            .all(|c| c.is_alphanumeric() || c == ' ' || c == '-' || c == '_')
}

/// A login secret: 64 hex digits (see `ps_net::protocol::login_secret`).
#[cfg(feature = "accounts")]
fn valid_secret(s: &str) -> bool {
    s.len() == 64 && s.chars().all(|c| c.is_ascii_hexdigit())
}

#[cfg(feature = "accounts")]
mod store {
    use super::*;
    use argon2::Argon2;
    use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
    use rusqlite::{Connection, OptionalExtension, params};
    use std::path::Path;

    pub struct Accounts {
        db: Connection,
    }

    fn now() -> i64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_secs() as i64)
    }

    impl Accounts {
        /// Opens (or makes) the database at `path` (":memory:" for one that isn't kept).
        pub fn open(path: &Path) -> Result<Accounts, String> {
            let db = Connection::open(path).map_err(|e| e.to_string())?;
            db.execute_batch(
                "PRAGMA journal_mode = WAL;
                 CREATE TABLE IF NOT EXISTS users (
                     id INTEGER PRIMARY KEY,
                     name TEXT NOT NULL UNIQUE COLLATE NOCASE,
                     secret TEXT NOT NULL,
                     created INTEGER NOT NULL,
                     last_seen INTEGER NOT NULL,
                     xp INTEGER NOT NULL DEFAULT 0
                 );
                 CREATE TABLE IF NOT EXISTS stats (
                     user INTEGER NOT NULL REFERENCES users(id),
                     key TEXT NOT NULL,
                     value INTEGER NOT NULL,
                     PRIMARY KEY (user, key)
                 );",
            )
            .map_err(|e| e.to_string())?;
            Ok(Accounts { db })
        }

        /// Registers `name` with `secret`.
        pub fn register(&mut self, name: &str, secret: &str) -> Result<Account, String> {
            if !valid_name(name) {
                return Err("Names are 3 to 24 letters, digits, spaces, - or _".into());
            }
            if !valid_secret(secret) {
                return Err("Bad login".into());
            }
            let taken: Option<i64> = self
                .db
                .query_row("SELECT id FROM users WHERE name = ?1", [name], |r| r.get(0))
                .optional()
                .map_err(|e| e.to_string())?;
            if taken.is_some() {
                return Err(format!("\"{name}\" is taken"));
            }
            let salt =
                SaltString::encode_b64(&rand::random::<[u8; 16]>()).map_err(|e| e.to_string())?;
            let hash = Argon2::default()
                .hash_password(secret.as_bytes(), &salt)
                .map_err(|e| e.to_string())?
                .to_string();
            let t = now();
            self.db
                .execute(
                    "INSERT INTO users (name, secret, created, last_seen) VALUES (?1, ?2, ?3, ?3)",
                    params![name, hash, t],
                )
                .map_err(|e| e.to_string())?;
            Ok(Account {
                id: self.db.last_insert_rowid(),
                name: name.to_string(),
                xp: 0,
            })
        }

        /// Logs `name` in with `secret`.
        pub fn login(&mut self, name: &str, secret: &str) -> Result<Account, String> {
            let row: Option<(i64, String, String, i64)> = self
                .db
                .query_row(
                    "SELECT id, name, secret, xp FROM users WHERE name = ?1",
                    [name],
                    |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
                )
                .optional()
                .map_err(|e| e.to_string())?;
            let wrong = || "Wrong name or password".to_string();
            let Some((id, name, hash, xp)) = row else {
                return Err(wrong());
            };
            let parsed = PasswordHash::new(&hash).map_err(|e| e.to_string())?;
            if !valid_secret(secret)
                || Argon2::default()
                    .verify_password(secret.as_bytes(), &parsed)
                    .is_err()
            {
                return Err(wrong());
            }
            let _ = self.db.execute(
                "UPDATE users SET last_seen = ?1 WHERE id = ?2",
                params![now(), id],
            );
            Ok(Account {
                id,
                name,
                xp: xp.max(0) as u64,
            })
        }

        /// Adds `xp` to `user`'s; its XP in all.
        pub fn add_xp(&mut self, user: i64, xp: u32) -> u64 {
            let _ = self.db.execute(
                "UPDATE users SET xp = xp + ?1 WHERE id = ?2",
                params![xp as i64, user],
            );
            self.db
                .query_row("SELECT xp FROM users WHERE id = ?1", [user], |r| {
                    r.get::<_, i64>(0)
                })
                .map_or(0, |x| x.max(0) as u64)
        }

        /// Adds to `user`'s stats counters.
        pub fn add_stats(&mut self, user: i64, add: &[(String, i64)]) {
            let Ok(tx) = self.db.transaction() else {
                return;
            };
            for (k, v) in add {
                let _ = tx.execute(
                    "INSERT INTO stats (user, key, value) VALUES (?1, ?2, ?3)
                     ON CONFLICT (user, key) DO UPDATE SET value = value + ?3",
                    params![user, k, v],
                );
            }
            let _ = tx.commit();
        }

        /// `user`'s stats counters, by key.
        pub fn stats(&self, user: i64) -> Vec<(String, i64)> {
            let Ok(mut q) = self
                .db
                .prepare("SELECT key, value FROM stats WHERE user = ?1 ORDER BY key")
            else {
                return Vec::new();
            };
            q.query_map([user], |r| Ok((r.get(0)?, r.get(1)?)))
                .map(|rows| rows.flatten().collect())
                .unwrap_or_default()
        }
    }
}

#[cfg(not(feature = "accounts"))]
mod store {
    use super::*;
    use std::path::Path;

    /// (Built without accounts: none can be opened.)
    pub struct Accounts;

    impl Accounts {
        pub fn open(_: &Path) -> Result<Accounts, String> {
            Err("built without accounts".into())
        }
        pub fn register(&mut self, _: &str, _: &str) -> Result<Account, String> {
            Err("no accounts here".into())
        }
        pub fn login(&mut self, _: &str, _: &str) -> Result<Account, String> {
            Err("no accounts here".into())
        }
        pub fn add_xp(&mut self, _: i64, _: u32) -> u64 {
            0
        }
        pub fn add_stats(&mut self, _: i64, _: &[(String, i64)]) {}
        pub fn stats(&self, _: i64) -> Vec<(String, i64)> {
            Vec::new()
        }
    }
}

pub use store::Accounts;

#[cfg(all(test, feature = "accounts"))]
mod tests {
    use super::*;

    const S1: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
    const S2: &str = "f123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

    #[test]
    fn register_log_in_and_keep_xp_and_stats() {
        let mut a = Accounts::open(std::path::Path::new(":memory:")).unwrap();
        assert!(a.register("ab", S1).is_err(), "too short");
        assert!(a.register(" Spaced", S1).is_err());
        let me = a.register("Tester", S1).unwrap();
        assert!(
            a.register("tester", S2).is_err(),
            "taken, whatever the case"
        );
        assert!(a.login("Tester", S2).is_err(), "wrong password");
        assert!(a.login("Nobody", S1).is_err());
        let back = a.login("TESTER", S1).unwrap();
        assert_eq!((back.id, back.name.as_str(), back.xp), (me.id, "Tester", 0));
        assert_eq!(a.add_xp(me.id, 150), 150);
        assert_eq!(a.add_xp(me.id, 50), 200);
        a.add_stats(me.id, &[("kills".into(), 2), ("deaths".into(), 1)]);
        a.add_stats(me.id, &[("kills".into(), 3)]);
        assert_eq!(
            a.stats(me.id),
            vec![("deaths".to_string(), 1), ("kills".to_string(), 5)]
        );
        assert_eq!(a.login("Tester", S1).unwrap().xp, 200);
    }
}

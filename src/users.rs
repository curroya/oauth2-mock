use std::collections::HashSet;

use serde::Deserialize;
use sha2::{Digest, Sha256};

#[derive(Clone, Debug, Deserialize)]
pub struct UserEntry {
    pub email: String,
    pub name: Option<String>,
    pub id: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct User {
    pub id: String,
    pub email: String,
    pub name: String,
}

pub struct Directory {
    users: Vec<User>,
}

impl UserEntry {
    pub fn for_email(email: &str) -> Self {
        Self {
            email: email.to_owned(),
            name: None,
            id: None,
        }
    }
}

impl User {
    fn from_entry(entry: &UserEntry) -> Self {
        let email = normalized(&entry.email);
        let id = entry.id.clone().unwrap_or_else(|| derived_id(&email));
        let name = entry
            .name
            .clone()
            .unwrap_or_else(|| local_part(&email).to_owned());
        Self { id, email, name }
    }

    fn unknown(email: &str) -> Self {
        Self::from_entry(&UserEntry::for_email(email))
    }
}

impl Directory {
    pub fn new(entries: &[UserEntry]) -> Self {
        let mut seen_emails = HashSet::new();
        let users = entries
            .iter()
            .map(User::from_entry)
            .filter(|user| seen_emails.insert(user.email.clone()))
            .collect();
        Self { users }
    }

    pub fn users(&self) -> &[User] {
        &self.users
    }

    pub fn user_for(&self, email: &str) -> User {
        let email = normalized(email);
        let known_user = self.users.iter().find(|user| user.email == email);
        known_user.cloned().unwrap_or_else(|| User::unknown(&email))
    }
}

pub fn derived_id(email: &str) -> String {
    let digest = Sha256::digest(normalized(email).as_bytes());
    let hex: String = digest.iter().map(|byte| format!("{byte:02x}")).collect();
    uuid_shaped(&hex)
}

fn uuid_shaped(hex: &str) -> String {
    format!(
        "{}-{}-{}-{}-{}",
        &hex[0..8],
        &hex[8..12],
        &hex[12..16],
        &hex[16..20],
        &hex[20..32]
    )
}

fn normalized(email: &str) -> String {
    email.trim().to_lowercase()
}

fn local_part(email: &str) -> &str {
    email.split('@').next().unwrap_or(email)
}

#[cfg(test)]
mod tests {
    use super::*;

    const EMAIL: &str = "alice@curroya.test";

    #[test]
    fn derives_the_same_id_for_the_same_email() {
        let id = derived_id(EMAIL);

        assert_eq!(id, derived_id(" Alice@CurroYa.test "));
        assert_eq!(id.len(), 36);
    }

    #[test]
    fn derives_different_ids_for_different_emails() {
        assert_ne!(derived_id(EMAIL), derived_id("bob@curroya.test"));
    }

    #[test]
    fn keeps_a_configured_id_and_name() {
        let entry = UserEntry {
            email: EMAIL.to_owned(),
            name: Some("Alice Doe".to_owned()),
            id: Some("fixed-id".to_owned()),
        };
        let directory = Directory::new(&[entry]);

        let user = directory.user_for(EMAIL);

        assert_eq!(user.id, "fixed-id");
        assert_eq!(user.name, "Alice Doe");
    }

    #[test]
    fn signs_in_an_unknown_email_with_a_derived_id() {
        let directory = Directory::new(&[]);

        let user = directory.user_for(EMAIL);

        assert_eq!(user.id, derived_id(EMAIL));
        assert_eq!(user.name, "alice");
    }

    #[test]
    fn keeps_the_first_entry_for_a_repeated_email() {
        let first = UserEntry {
            id: Some("from-file".to_owned()),
            ..UserEntry::for_email(EMAIL)
        };
        let directory = Directory::new(&[first, UserEntry::for_email(EMAIL)]);

        assert_eq!(directory.users().len(), 1);
        assert_eq!(directory.user_for(EMAIL).id, "from-file");
    }
}

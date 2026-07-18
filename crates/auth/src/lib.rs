pub mod captcha;
pub mod jwt;
pub mod password;
pub mod token;

#[cfg(test)]
mod tests {
    use super::{jwt::JwtService, password::PasswordService};

    #[test]
    fn password_service_hashes_and_verifies_passwords() {
        let password_service = PasswordService::new();
        let password_hash = password_service
            .hash_password("123456")
            .expect("password hash should be generated");

        assert!(
            password_service
                .verify_password("123456", &password_hash)
                .expect("password verification should succeed")
        );
        assert!(
            !password_service
                .verify_password("wrong-password", &password_hash)
                .expect("password verification should return false for mismatched passwords")
        );
    }

    #[test]
    fn jwt_service_round_trips_claims() {
        let jwt_service = JwtService::new("test-secret");
        let token = jwt_service
            .issue_token(1, "admin", "session-123")
            .expect("token should be issued");
        let claims = jwt_service
            .decode_token(&token)
            .expect("token should decode");

        assert_eq!(claims.user_id, 1);
        assert_eq!(claims.username, "admin");
        assert_eq!(claims.sid, "session-123");
    }
}

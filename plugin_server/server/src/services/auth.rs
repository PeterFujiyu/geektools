use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use sqlx::PgPool;
use ipnetwork::IpNetwork;
use std::sync::Arc;
use std::collections::HashMap;
use tokio::sync::RwLock;

use crate::{
    models::{LoginResponse, TokenClaims, User, UserResponse, VerificationCode, AuthResponse, UserInfo},
    utils::config::Config,
};

pub struct AuthService {
    db_pool: PgPool,
    config: Arc<Config>,
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
    verification_codes: Arc<RwLock<HashMap<String, VerificationCode>>>,
}

impl AuthService {
    pub fn new(db_pool: PgPool, config: Arc<Config>) -> Self {
        let encoding_key = EncodingKey::from_secret(config.jwt.secret.as_bytes());
        let decoding_key = DecodingKey::from_secret(config.jwt.secret.as_bytes());

        Self {
            db_pool,
            config,
            encoding_key,
            decoding_key,
            verification_codes: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn user_exists(&self, username: &str, email: &str) -> sqlx::Result<bool> {
        let count = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM users WHERE username = $1 OR email = $2"
        )
        .bind(username)
        .bind(email)
        .fetch_one(&self.db_pool)
        .await?;

        Ok(count > 0)
    }

    pub async fn register_user(
        &self,
        username: String,
        email: String,
        password: String,
        display_name: Option<String>,
    ) -> sqlx::Result<UserResponse> {
        let password_hash = bcrypt::hash(password, bcrypt::DEFAULT_COST)
            .map_err(|_| sqlx::Error::Protocol("Password hashing failed".to_string()))?;

        let user = sqlx::query_as::<_, User>(
            r#"
            INSERT INTO users (username, email, password_hash, display_name)
            VALUES ($1, $2, $3, $4)
            RETURNING *
            "#,
        )
        .bind(&username)
        .bind(&email)
        .bind(&password_hash)
        .bind(&display_name)
        .fetch_one(&self.db_pool)
        .await?;

        Ok(user.into())
    }

    pub async fn authenticate(
        &self,
        username: &str,
        password: &str,
    ) -> sqlx::Result<Option<LoginResponse>> {
        let user = sqlx::query_as::<_, User>(
            "SELECT * FROM users WHERE username = $1 AND is_active = true"
        )
        .bind(username)
        .fetch_optional(&self.db_pool)
        .await?;

        if let Some(user) = user {
            if bcrypt::verify(password, &user.password_hash)
                .map_err(|_| sqlx::Error::Protocol("Password verification failed".to_string()))?
            {
                let (access_token, refresh_token) = self.generate_tokens(&user)?;
                
                return Ok(Some(LoginResponse {
                    access_token,
                    refresh_token,
                    expires_in: self.config.jwt.access_token_expires_in,
                    user: user.into(),
                }));
            }
        }

        Ok(None)
    }

    pub async fn refresh_token(&self, refresh_token: &str) -> sqlx::Result<Option<LoginResponse>> {
        let claims = self.verify_token(refresh_token)
            .map_err(|_| sqlx::Error::Protocol("Invalid token".to_string()))?;
        
        let user_id = claims.sub.parse::<i32>()
            .map_err(|_| sqlx::Error::Protocol("Invalid user ID in token".to_string()))?;
        
        let user = sqlx::query_as::<_, User>(
            "SELECT * FROM users WHERE id = $1 AND is_active = true"
        )
        .bind(user_id)
        .fetch_optional(&self.db_pool)
        .await?;

        if let Some(user) = user {
            let (access_token, refresh_token) = self.generate_tokens(&user)?;
            
            return Ok(Some(LoginResponse {
                access_token,
                refresh_token,
                expires_in: self.config.jwt.access_token_expires_in,
                user: user.into(),
            }));
        }

        Ok(None)
    }

    pub fn verify_token(&self, token: &str) -> Result<TokenClaims, jsonwebtoken::errors::Error> {
        let validation = Validation::default();
        let token_data = decode::<TokenClaims>(token, &self.decoding_key, &validation)?;
        Ok(token_data.claims)
    }

    fn generate_tokens(&self, user: &User) -> sqlx::Result<(String, String)> {
        let now = Utc::now();
        let access_exp = (now + Duration::seconds(self.config.jwt.access_token_expires_in)).timestamp() as usize;
        let refresh_exp = (now + Duration::seconds(self.config.jwt.refresh_token_expires_in)).timestamp() as usize;

        let access_claims = TokenClaims {
            sub: user.id.to_string(),
            username: user.username.clone(),
            exp: access_exp,
            iat: now.timestamp() as usize,
        };

        let refresh_claims = TokenClaims {
            sub: user.id.to_string(),
            username: user.username.clone(),
            exp: refresh_exp,
            iat: now.timestamp() as usize,
        };

        let access_token = encode(&Header::default(), &access_claims, &self.encoding_key)
            .map_err(|_| sqlx::Error::Protocol("Token generation failed".to_string()))?;

        let refresh_token = encode(&Header::default(), &refresh_claims, &self.encoding_key)
            .map_err(|_| sqlx::Error::Protocol("Token generation failed".to_string()))?;

        Ok((access_token, refresh_token))
    }

    // Email verification code methods
    pub async fn send_verification_code(&self, email: String) -> anyhow::Result<String> {
        // Generate 6-digit code
        let code = format!("{:06}", fastrand::u32(100000..1000000));
        
        // Store code with 10 minute expiration
        let verification_code = VerificationCode {
            email: email.clone(),
            code: code.clone(),
            expires_at: Utc::now() + Duration::minutes(10),
        };

        // Clean up expired codes and store new one
        self.cleanup_expired_codes().await;
        {
            let mut codes = self.verification_codes.write().await;
            codes.insert(email, verification_code);
        }

        // TODO: Send email via SMTP if configured
        // For now, just return the code for display
        Ok(code)
    }

    pub async fn verify_code_and_auth(&self, email: String, code: String, ip_address: Option<std::net::IpAddr>, user_agent: Option<&str>) -> anyhow::Result<AuthResponse> {
        // Check verification code
        {
            let codes = self.verification_codes.read().await;
            if let Some(stored_code) = codes.get(&email) {
                if stored_code.expires_at < Utc::now() {
                    return Err(anyhow::anyhow!("验证码已过期"));
                }
                if stored_code.code != code {
                    return Err(anyhow::anyhow!("验证码错误"));
                }
            } else {
                return Err(anyhow::anyhow!("验证码不存在或已过期"));
            }
        }

        // Remove used code
        {
            let mut codes = self.verification_codes.write().await;
            codes.remove(&email);
        }

        // Find or create user
        let user = self.find_or_create_user_by_email(&email).await?;

        // Generate JWT token
        let (access_token, _) = self.generate_tokens(&user)
            .map_err(|e| anyhow::anyhow!("Token generation failed: {}", e))?;

        // Record login activity
        if let Err(e) = self.record_login_activity(user.id, &user.email, ip_address, user_agent, true, None).await {
            tracing::warn!("Failed to record login activity: {}", e);
        }

        Ok(AuthResponse {
            token: access_token,
            user: UserInfo {
                id: user.id,
                email: user.email,
                display_name: user.display_name,
                role: user.role,
            },
        })
    }

    async fn find_or_create_user_by_email(&self, email: &str) -> anyhow::Result<User> {
        // Try to find existing user
        if let Ok(user) = sqlx::query_as::<_, User>("SELECT * FROM users WHERE email = $1")
            .bind(email)
            .fetch_one(&self.db_pool)
            .await
        {
            return Ok(user);
        }

        // Create new user
        let username = email.split('@').next().unwrap_or("user").to_string();
        let display_name = format!("用户{}", &username[..std::cmp::min(username.len(), 6)]);

        let user = sqlx::query_as::<_, User>(
            "INSERT INTO users (username, email, password_hash, display_name, is_active, is_verified, created_at, updated_at)
             VALUES ($1, $2, $3, $4, true, true, NOW(), NOW())
             RETURNING *"
        )
        .bind(&username)
        .bind(email)
        .bind("") // Empty password for email-only auth
        .bind(&display_name)
        .fetch_one(&self.db_pool)
        .await?;

        Ok(user)
    }

    async fn cleanup_expired_codes(&self) {
        let now = Utc::now();
        let mut codes = self.verification_codes.write().await;
        codes.retain(|_, code| code.expires_at > now);
    }

    // Record login activity
    async fn record_login_activity(
        &self,
        user_id: i32,
        email: &str,
        ip_address: Option<std::net::IpAddr>,
        user_agent: Option<&str>,
        is_successful: bool,
        failure_reason: Option<&str>,
    ) -> sqlx::Result<()> {
        sqlx::query(
            r#"
            INSERT INTO user_login_activities 
            (user_id, email, ip_address, user_agent, login_time, login_method, is_successful, failure_reason)
            VALUES ($1, $2, $3, $4, NOW(), 'email_verification', $5, $6)
            "#,
        )
        .bind(user_id)
        .bind(email)
        .bind(ip_address.map(|ip| IpNetwork::from(ip)))
        .bind(user_agent)
        .bind(is_successful)
        .bind(failure_reason)
        .execute(&self.db_pool)
        .await?;

        Ok(())
    }

    // Public getter for database pool (needed for auth middleware)
    pub fn get_db_pool(&self) -> &PgPool {
        &self.db_pool
    }
}
use chrono::{DateTime, Duration, Utc};
use sqlx::{PgPool, Row, Column, ValueRef, TypeInfo};
use ipnetwork::IpNetwork;
use std::net::IpAddr;
use std::sync::Arc;
use std::time::Instant;

use crate::{
    models::{
        AdminDashboardStats, AdminSqlLog, ExecuteSqlRequest, SqlExecutionResult,
        UpdateUserEmailRequest, UserLoginActivity, UserManagementInfo,
        AdminPaginationQuery, DeletePluginRequest, BanUserRequest, UnbanUserRequest,
    },
    utils::config::Config,
};

pub struct AdminService {
    db_pool: PgPool,
    config: Arc<Config>,
}

impl AdminService {
    pub fn new(db_pool: PgPool, config: Arc<Config>) -> Self {
        Self { db_pool, config }
    }

    // Check if user has admin role
    pub async fn is_admin(&self, user_id: i32) -> anyhow::Result<bool> {
        let role: Option<String> = sqlx::query_scalar("SELECT role FROM users WHERE id = $1")
            .bind(user_id)
            .fetch_optional(&self.db_pool)
            .await?;

        Ok(role.map(|r| r == "admin").unwrap_or(false))
    }

    // Record user login activity
    pub async fn record_login_activity(
        &self,
        user_id: i32,
        email: &str,
        ip_address: Option<IpAddr>,
        user_agent: Option<&str>,
        is_successful: bool,
        failure_reason: Option<&str>,
    ) -> anyhow::Result<i32> {
        let activity_id = sqlx::query_scalar::<_, i32>(
            r#"
            INSERT INTO user_login_activities 
            (user_id, email, ip_address, user_agent, login_time, login_method, is_successful, failure_reason)
            VALUES ($1, $2, $3, $4, NOW(), 'email_verification', $5, $6)
            RETURNING id
            "#,
        )
        .bind(user_id)
        .bind(email)
        .bind(ip_address.map(|ip| IpNetwork::from(ip)))
        .bind(user_agent)
        .bind(is_successful)
        .bind(failure_reason)
        .fetch_one(&self.db_pool)
        .await?;

        Ok(activity_id)
    }

    // Get admin dashboard statistics
    pub async fn get_dashboard_stats(&self) -> anyhow::Result<AdminDashboardStats> {
        // Get basic statistics
        let total_users: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users")
            .fetch_one(&self.db_pool)
            .await?;

        let total_plugins: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM plugins")
            .fetch_one(&self.db_pool)
            .await?;

        let total_downloads: i64 = sqlx::query_scalar("SELECT COALESCE(SUM(downloads), 0) FROM plugins")
            .fetch_one(&self.db_pool)
            .await?;

        // Get active sessions (logins in last 24 hours without logout)
        let active_sessions: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM user_login_activities 
             WHERE login_time > NOW() - INTERVAL '24 hours' 
             AND logout_time IS NULL AND is_successful = true"
        )
        .fetch_one(&self.db_pool)
        .await?;

        // Get recent login activities
        let recent_logins = sqlx::query_as::<_, UserLoginActivity>(
            "SELECT * FROM user_login_activities 
             ORDER BY login_time DESC LIMIT 10"
        )
        .fetch_all(&self.db_pool)
        .await?;

        // Get recent SQL executions
        let recent_sql_executions = sqlx::query_as::<_, AdminSqlLog>(
            "SELECT * FROM admin_sql_logs 
             ORDER BY executed_at DESC LIMIT 10"
        )
        .fetch_all(&self.db_pool)
        .await?;

        Ok(AdminDashboardStats {
            total_users,
            total_plugins,
            total_downloads,
            active_sessions,
            recent_logins,
            recent_sql_executions,
        })
    }

    // Get user management information
    pub async fn get_users_for_management(
        &self,
        pagination: AdminPaginationQuery,
    ) -> anyhow::Result<(Vec<UserManagementInfo>, i64)> {
        let page = pagination.page.unwrap_or(1);
        let limit = pagination.limit.unwrap_or(20);
        let offset = (page - 1) * limit;

        // Get total count
        let total_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users")
            .fetch_one(&self.db_pool)
            .await?;

        // Get users with their latest login info
        let users = sqlx::query_as::<_, UserManagementInfo>(
            r#"
            SELECT 
                u.id,
                u.username,
                u.email,
                u.display_name,
                COALESCE(u.role, 'user') as role,
                u.is_active,
                u.is_verified,
                u.created_at,
                u.updated_at,
                la.last_login,
                COALESCE(la.login_count, 0) as login_count
            FROM users u
            LEFT JOIN (
                SELECT 
                    user_id,
                    MAX(login_time) as last_login,
                    COUNT(*) as login_count
                FROM user_login_activities 
                WHERE is_successful = true
                GROUP BY user_id
            ) la ON u.id = la.user_id
            ORDER BY u.created_at DESC
            LIMIT $1 OFFSET $2
            "#,
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.db_pool)
        .await?;

        Ok((users, total_count))
    }

    // Update user email
    pub async fn update_user_email(
        &self,
        admin_user_id: i32,
        request: UpdateUserEmailRequest,
        ip_address: Option<IpAddr>,
    ) -> anyhow::Result<()> {
        let mut tx = self.db_pool.begin().await?;

        // Get current email
        let current_email: String = sqlx::query_scalar("SELECT email FROM users WHERE id = $1")
            .bind(request.user_id)
            .fetch_one(&mut *tx)
            .await?;

        // Update email
        sqlx::query("UPDATE users SET email = $1, updated_at = NOW() WHERE id = $2")
            .bind(&request.new_email)
            .bind(request.user_id)
            .execute(&mut *tx)
            .await?;

        // Record the change
        sqlx::query(
            r#"
            INSERT INTO user_profile_changes 
            (user_id, changed_by_user_id, field_name, old_value, new_value, change_reason, ip_address)
            VALUES ($1, $2, 'email', $3, $4, $5, $6)
            "#,
        )
        .bind(request.user_id)
        .bind(admin_user_id)
        .bind(&current_email)
        .bind(&request.new_email)
        .bind(&request.reason)
        .bind(ip_address.map(|ip| IpNetwork::from(ip)))
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(())
    }

    // Execute SQL query with logging
    pub async fn execute_sql(
        &self,
        admin_user_id: i32,
        admin_email: &str,
        request: ExecuteSqlRequest,
        ip_address: Option<IpAddr>,
    ) -> anyhow::Result<SqlExecutionResult> {
        let start_time = Instant::now();
        let mut result = SqlExecutionResult {
            is_successful: false,
            rows_affected: None,
            execution_time_ms: 0,
            data: None,
            error_message: None,
        };

        // Security check: prevent dangerous operations
        let query_lower = request.sql_query.to_lowercase();
        if query_lower.contains("drop table")
            || query_lower.contains("drop database")
            || query_lower.contains("truncate")
            || query_lower.contains("delete from users")
            || query_lower.contains("update users set password_hash")
        {
            return Err(anyhow::anyhow!("禁止执行危险的SQL操作"));
        }

        let execution_result = if query_lower.trim().starts_with("select") {
            // For SELECT queries, return data
            match sqlx::query(&request.sql_query).fetch_all(&self.db_pool).await {
                Ok(rows) => {
                    let data: Vec<serde_json::Value> = rows
                        .into_iter()
                        .map(|row| {
                            let mut map = serde_json::Map::new();
                            for (i, column) in row.columns().iter().enumerate() {
                                let value = match row.try_get_raw(i) {
                                    Ok(raw_value) => {
                                        if raw_value.is_null() {
                                            serde_json::Value::Null
                                        } else {
                                            // Try to get as different types
                                            if let Ok(v) = row.try_get::<String, _>(i) {
                                                serde_json::Value::String(v)
                                            } else if let Ok(v) = row.try_get::<i32, _>(i) {
                                                serde_json::Value::Number(v.into())
                                            } else if let Ok(v) = row.try_get::<i64, _>(i) {
                                                serde_json::Value::Number(v.into())
                                            } else if let Ok(v) = row.try_get::<f64, _>(i) {
                                                serde_json::Value::Number(
                                                    serde_json::Number::from_f64(v).unwrap_or(0.into())
                                                )
                                            } else if let Ok(v) = row.try_get::<bool, _>(i) {
                                                serde_json::Value::Bool(v)
                                            } else {
                                                serde_json::Value::String("Unsupported value type".to_string())
                                            }
                                        }
                                    }
                                    Err(_) => serde_json::Value::String("Error reading value".to_string()),
                                };
                                map.insert(column.name().to_string(), value);
                            }
                            serde_json::Value::Object(map)
                        })
                        .collect();

                    result.is_successful = true;
                    result.data = Some(data.clone());
                    result.rows_affected = Some(data.len() as i32);
                    Ok(())
                }
                Err(e) => {
                    result.error_message = Some(e.to_string());
                    Err(e.into())
                }
            }
        } else {
            // For INSERT, UPDATE, DELETE queries
            match sqlx::query(&request.sql_query).execute(&self.db_pool).await {
                Ok(exec_result) => {
                    result.is_successful = true;
                    result.rows_affected = Some(exec_result.rows_affected() as i32);
                    Ok(())
                }
                Err(e) => {
                    result.error_message = Some(e.to_string());
                    Err(e.into())
                }
            }
        };

        let execution_time = start_time.elapsed();
        result.execution_time_ms = execution_time.as_millis() as i32;

        // Log the SQL execution
        let _ = sqlx::query(
            r#"
            INSERT INTO admin_sql_logs 
            (admin_user_id, admin_email, sql_query, execution_time_ms, rows_affected, is_successful, error_message, ip_address)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            "#,
        )
        .bind(admin_user_id)
        .bind(admin_email)
        .bind(&request.sql_query)
        .bind(result.execution_time_ms)
        .bind(result.rows_affected)
        .bind(result.is_successful)
        .bind(&result.error_message)
        .bind(ip_address.map(|ip| IpNetwork::from(ip)))
        .execute(&self.db_pool)
        .await;

        match execution_result {
            Ok(_) => Ok(result),
            Err(e) => Err(e),
        }
    }

    // Get user login activities
    pub async fn get_user_login_activities(
        &self,
        user_id: Option<i32>,
        pagination: AdminPaginationQuery,
    ) -> anyhow::Result<(Vec<UserLoginActivity>, i64)> {
        let page = pagination.page.unwrap_or(1);
        let limit = pagination.limit.unwrap_or(20);
        let offset = (page - 1) * limit;

        let (query, count_query) = if let Some(uid) = user_id {
            (
                "SELECT * FROM user_login_activities WHERE user_id = $1 ORDER BY login_time DESC LIMIT $2 OFFSET $3",
                "SELECT COUNT(*) FROM user_login_activities WHERE user_id = $1"
            )
        } else {
            (
                "SELECT * FROM user_login_activities ORDER BY login_time DESC LIMIT $1 OFFSET $2",
                "SELECT COUNT(*) FROM user_login_activities"
            )
        };

        let (activities, total_count) = if let Some(uid) = user_id {
            let activities = sqlx::query_as::<_, UserLoginActivity>(query)
                .bind(uid)
                .bind(limit)
                .bind(offset)
                .fetch_all(&self.db_pool)
                .await?;

            let total_count: i64 = sqlx::query_scalar(count_query)
                .bind(uid)
                .fetch_one(&self.db_pool)
                .await?;

            (activities, total_count)
        } else {
            let activities = sqlx::query_as::<_, UserLoginActivity>(query)
                .bind(limit)
                .bind(offset)
                .fetch_all(&self.db_pool)
                .await?;

            let total_count: i64 = sqlx::query_scalar(count_query)
                .fetch_one(&self.db_pool)
                .await?;

            (activities, total_count)
        };

        Ok((activities, total_count))
    }

    // Delete plugin (soft delete by setting status = banned)
    pub async fn delete_plugin(
        &self,
        admin_user_id: i32,
        request: DeletePluginRequest,
        ip_address: Option<IpAddr>,
    ) -> anyhow::Result<()> {
        let mut tx = self.db_pool.begin().await?;

        // Soft delete the plugin by setting status to banned
        let rows_affected = sqlx::query("UPDATE plugins SET status = 'banned', updated_at = NOW() WHERE id = $1")
            .bind(&request.plugin_id)
            .execute(&mut *tx)
            .await?
            .rows_affected();

        if rows_affected == 0 {
            return Err(anyhow::anyhow!("Plugin not found"));
        }

        // Log the deletion action
        sqlx::query(
            r#"
            INSERT INTO admin_sql_logs 
            (admin_user_id, admin_email, sql_query, execution_time_ms, rows_affected, is_successful, error_message, ip_address)
            VALUES ($1, (SELECT email FROM users WHERE id = $1), $2, 0, $3, true, NULL, $4)
            "#,
        )
        .bind(admin_user_id)
        .bind(&format!("DELETE PLUGIN: {} - REASON: {}", request.plugin_id, request.reason.unwrap_or("No reason provided".to_string())))
        .bind(rows_affected as i32)
        .bind(ip_address.map(|ip| IpNetwork::from(ip)))
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(())
    }

    // Ban user
    pub async fn ban_user(
        &self,
        admin_user_id: i32,
        request: BanUserRequest,
        ip_address: Option<IpAddr>,
    ) -> anyhow::Result<()> {
        let mut tx = self.db_pool.begin().await?;

        // Calculate ban expiry date
        let ban_expires_at = if let Some(days) = request.ban_duration_days {
            Some(Utc::now() + Duration::days(days as i64))
        } else {
            None // Permanent ban
        };

        // Update user status
        let rows_affected = sqlx::query(
            "UPDATE users SET is_active = false, updated_at = NOW() WHERE id = $1"
        )
        .bind(request.user_id)
        .execute(&mut *tx)
        .await?
        .rows_affected();

        if rows_affected == 0 {
            return Err(anyhow::anyhow!("User not found"));
        }

        // Record the ban action
        sqlx::query(
            r#"
            INSERT INTO user_profile_changes 
            (user_id, changed_by_user_id, field_name, old_value, new_value, change_reason, ip_address)
            VALUES ($1, $2, 'ban_status', 'active', 'banned', $3, $4)
            "#,
        )
        .bind(request.user_id)
        .bind(admin_user_id)
        .bind(&request.reason.unwrap_or("No reason provided".to_string()))
        .bind(ip_address.map(|ip| IpNetwork::from(ip)))
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(())
    }

    // Unban user
    pub async fn unban_user(
        &self,
        admin_user_id: i32,
        request: UnbanUserRequest,
        ip_address: Option<IpAddr>,
    ) -> anyhow::Result<()> {
        let mut tx = self.db_pool.begin().await?;

        // Update user status
        let rows_affected = sqlx::query(
            "UPDATE users SET is_active = true, updated_at = NOW() WHERE id = $1"
        )
        .bind(request.user_id)
        .execute(&mut *tx)
        .await?
        .rows_affected();

        if rows_affected == 0 {
            return Err(anyhow::anyhow!("User not found"));
        }

        // Record the unban action
        sqlx::query(
            r#"
            INSERT INTO user_profile_changes 
            (user_id, changed_by_user_id, field_name, old_value, new_value, change_reason, ip_address)
            VALUES ($1, $2, 'ban_status', 'banned', 'active', $3, $4)
            "#,
        )
        .bind(request.user_id)
        .bind(admin_user_id)
        .bind(&request.reason.unwrap_or("No reason provided".to_string()))
        .bind(ip_address.map(|ip| IpNetwork::from(ip)))
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(())
    }

    // Get plugin list for admin management
    pub async fn get_plugins_for_management(
        &self,
        pagination: AdminPaginationQuery,
    ) -> anyhow::Result<(Vec<serde_json::Value>, i64)> {
        let page = pagination.page.unwrap_or(1);
        let limit = pagination.limit.unwrap_or(20);
        let offset = (page - 1) * limit;

        // Get total count
        let total_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM plugins")
            .fetch_one(&self.db_pool)
            .await?;

        // Get plugins with basic info
        let plugins = sqlx::query(
            r#"
            SELECT 
                id,
                name,
                description,
                author,
                current_version,
                downloads,
                rating,
                status::text as status,
                created_at,
                updated_at
            FROM plugins
            ORDER BY created_at DESC
            LIMIT $1 OFFSET $2
            "#,
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.db_pool)
        .await?;

        let plugin_data: Vec<serde_json::Value> = plugins
            .into_iter()
            .map(|row| {
                let mut map = serde_json::Map::new();
                for (i, column) in row.columns().iter().enumerate() {
                    let column_name = column.name();
                    let value = match row.try_get_raw(i) {
                        Ok(raw_value) => {
                            if raw_value.is_null() {
                                serde_json::Value::Null
                            } else {
                                if let Ok(v) = row.try_get::<String, _>(i) {
                                    // Special handling for status column
                                    if column_name == "status" {
                                        // Add both status and is_active
                                        map.insert("status".to_string(), serde_json::Value::String(v.clone()));
                                        map.insert("is_active".to_string(), serde_json::Value::Bool(v == "active"));
                                        continue;
                                    }
                                    serde_json::Value::String(v)
                                } else if let Ok(v) = row.try_get::<i32, _>(i) {
                                    serde_json::Value::Number(v.into())
                                } else if let Ok(v) = row.try_get::<i64, _>(i) {
                                    serde_json::Value::Number(v.into())
                                } else if let Ok(v) = row.try_get::<f64, _>(i) {
                                    serde_json::Value::Number(
                                        serde_json::Number::from_f64(v).unwrap_or(0.into())
                                    )
                                } else if let Ok(v) = row.try_get::<bool, _>(i) {
                                    serde_json::Value::Bool(v)
                                } else if let Ok(v) = row.try_get::<chrono::DateTime<chrono::Utc>, _>(i) {
                                    serde_json::Value::String(v.to_rfc3339())
                                } else {
                                    // Try to convert any remaining type to string as fallback
                                    match raw_value.type_info().name() {
                                        "NUMERIC" | "DECIMAL" => {
                                            if let Ok(v) = row.try_get::<String, _>(i) {
                                                if let Ok(num) = v.parse::<f64>() {
                                                    serde_json::Value::Number(
                                                        serde_json::Number::from_f64(num).unwrap_or(0.into())
                                                    )
                                                } else {
                                                    serde_json::Value::String(v)
                                                }
                                            } else {
                                                serde_json::Value::String("0".to_string())
                                            }
                                        }
                                        // Handle custom PostgreSQL enum types
                                        type_name if type_name.starts_with("plugin_") => {
                                            if let Ok(v) = row.try_get::<String, _>(i) {
                                                serde_json::Value::String(v)
                                            } else {
                                                serde_json::Value::String("unknown".to_string())
                                            }
                                        }
                                        _ => {
                                            // Final fallback - try as string
                                            if let Ok(v) = row.try_get::<String, _>(i) {
                                                serde_json::Value::String(v)
                                            } else {
                                                serde_json::Value::String("Unsupported value type".to_string())
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        Err(_) => serde_json::Value::String("Error reading value".to_string()),
                    };
                    map.insert(column_name.to_string(), value);
                }
                serde_json::Value::Object(map)
            })
            .collect();

        Ok((plugin_data, total_count))
    }
}
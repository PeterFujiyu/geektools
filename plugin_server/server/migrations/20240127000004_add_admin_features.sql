-- Add admin features: user roles and login activity tracking

-- Add role column to users table
ALTER TABLE users ADD COLUMN IF NOT EXISTS role VARCHAR(20) DEFAULT 'user';
UPDATE users SET role = 'admin' WHERE id = 1; -- Make first user admin

-- Create user_login_activities table
CREATE TABLE IF NOT EXISTS user_login_activities (
    id SERIAL PRIMARY KEY,
    user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    email VARCHAR(255) NOT NULL,
    ip_address INET,
    user_agent TEXT,
    login_time TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    logout_time TIMESTAMPTZ,
    session_duration INTEGER, -- in seconds
    login_method VARCHAR(50) DEFAULT 'email_verification',
    is_successful BOOLEAN DEFAULT true,
    failure_reason TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Create indexes for performance
CREATE INDEX IF NOT EXISTS idx_user_login_activities_user_id ON user_login_activities(user_id);
CREATE INDEX IF NOT EXISTS idx_user_login_activities_login_time ON user_login_activities(login_time);
CREATE INDEX IF NOT EXISTS idx_user_login_activities_email ON user_login_activities(email);

-- Create admin_sql_logs table for SQL console activity tracking
CREATE TABLE IF NOT EXISTS admin_sql_logs (
    id SERIAL PRIMARY KEY,
    admin_user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    admin_email VARCHAR(255) NOT NULL,
    sql_query TEXT NOT NULL,
    execution_time_ms INTEGER,
    rows_affected INTEGER,
    is_successful BOOLEAN DEFAULT true,
    error_message TEXT,
    ip_address INET,
    executed_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Create index for admin SQL logs
CREATE INDEX IF NOT EXISTS idx_admin_sql_logs_admin_user_id ON admin_sql_logs(admin_user_id);
CREATE INDEX IF NOT EXISTS idx_admin_sql_logs_executed_at ON admin_sql_logs(executed_at);

-- Create user_profile_changes table for tracking email changes
CREATE TABLE IF NOT EXISTS user_profile_changes (
    id SERIAL PRIMARY KEY,
    user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    changed_by_user_id INTEGER NOT NULL REFERENCES users(id),
    field_name VARCHAR(50) NOT NULL,
    old_value TEXT,
    new_value TEXT,
    change_reason TEXT,
    ip_address INET,
    changed_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Create index for user profile changes
CREATE INDEX IF NOT EXISTS idx_user_profile_changes_user_id ON user_profile_changes(user_id);
CREATE INDEX IF NOT EXISTS idx_user_profile_changes_changed_at ON user_profile_changes(changed_at);
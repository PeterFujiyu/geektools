-- Create custom types
CREATE TYPE plugin_status AS ENUM ('active', 'deprecated', 'banned');

-- Users table
CREATE TABLE users (
    id SERIAL PRIMARY KEY,
    username VARCHAR(100) UNIQUE NOT NULL,
    email VARCHAR(255) UNIQUE NOT NULL,
    password_hash VARCHAR(255) NOT NULL,
    display_name VARCHAR(255),
    is_active BOOLEAN DEFAULT true,
    is_verified BOOLEAN DEFAULT false,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

-- Plugins table
CREATE TABLE plugins (
    id VARCHAR(255) PRIMARY KEY,
    name VARCHAR(255) NOT NULL,
    description TEXT,
    author VARCHAR(255) NOT NULL,
    current_version VARCHAR(50) NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    downloads INTEGER DEFAULT 0,
    rating DECIMAL(3,2) DEFAULT 0.00,
    status plugin_status DEFAULT 'active',
    min_geektools_version VARCHAR(50),
    homepage_url VARCHAR(500),
    repository_url VARCHAR(500),
    license VARCHAR(100)
);

-- Plugin versions table
CREATE TABLE plugin_versions (
    id SERIAL PRIMARY KEY,
    plugin_id VARCHAR(255) NOT NULL REFERENCES plugins(id) ON DELETE CASCADE,
    version VARCHAR(50) NOT NULL,
    changelog TEXT,
    file_path VARCHAR(500) NOT NULL,
    file_size BIGINT NOT NULL,
    file_hash VARCHAR(64) NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    downloads INTEGER DEFAULT 0,
    is_stable BOOLEAN DEFAULT true,
    UNIQUE(plugin_id, version)
);

-- Plugin scripts table
CREATE TABLE plugin_scripts (
    id SERIAL PRIMARY KEY,
    plugin_id VARCHAR(255) NOT NULL REFERENCES plugins(id) ON DELETE CASCADE,
    version VARCHAR(50) NOT NULL,
    script_name VARCHAR(255) NOT NULL,
    script_file VARCHAR(255) NOT NULL,
    description TEXT,
    is_executable BOOLEAN DEFAULT false
);

-- Plugin dependencies table
CREATE TABLE plugin_dependencies (
    id SERIAL PRIMARY KEY,
    plugin_id VARCHAR(255) NOT NULL REFERENCES plugins(id) ON DELETE CASCADE,
    dependency_id VARCHAR(255) NOT NULL REFERENCES plugins(id) ON DELETE CASCADE,
    min_version VARCHAR(50)
);

-- Plugin tags table
CREATE TABLE plugin_tags (
    id SERIAL PRIMARY KEY,
    plugin_id VARCHAR(255) NOT NULL REFERENCES plugins(id) ON DELETE CASCADE,
    tag VARCHAR(100) NOT NULL,
    UNIQUE(plugin_id, tag)
);

-- Plugin ratings table
CREATE TABLE plugin_ratings (
    id SERIAL PRIMARY KEY,
    plugin_id VARCHAR(255) NOT NULL REFERENCES plugins(id) ON DELETE CASCADE,
    user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    rating INTEGER CHECK (rating >= 1 AND rating <= 5),
    review TEXT,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(user_id, plugin_id)
);

-- Create indexes for better performance
CREATE INDEX idx_plugins_status_downloads ON plugins(status, downloads DESC);
CREATE INDEX idx_plugins_created_at ON plugins(created_at DESC);
CREATE INDEX idx_plugins_rating ON plugins(rating DESC);
CREATE INDEX idx_plugins_updated_at ON plugins(updated_at DESC);
CREATE INDEX idx_plugin_versions_plugin_id ON plugin_versions(plugin_id);
CREATE INDEX idx_plugin_versions_created_at ON plugin_versions(created_at DESC);
CREATE INDEX idx_plugin_tags_plugin_id ON plugin_tags(plugin_id);
CREATE INDEX idx_plugin_tags_tag ON plugin_tags(tag);
CREATE INDEX idx_plugin_ratings_plugin_id ON plugin_ratings(plugin_id);
CREATE INDEX idx_plugin_ratings_user_id ON plugin_ratings(user_id);

-- Create function to update updated_at timestamp
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = CURRENT_TIMESTAMP;
    RETURN NEW;
END;
$$ language 'plpgsql';

-- Create triggers for updated_at
CREATE TRIGGER update_users_updated_at BEFORE UPDATE ON users 
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
    
CREATE TRIGGER update_plugins_updated_at BEFORE UPDATE ON plugins 
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
    
CREATE TRIGGER update_plugin_ratings_updated_at BEFORE UPDATE ON plugin_ratings 
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

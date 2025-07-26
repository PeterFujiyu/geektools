# GeekTools Plugin Marketplace Server Documentation

## 概述

本文档描述了如何构建和部署 GeekTools 插件市场服务端，用于托管、分发和管理插件包。服务端提供 RESTful API 接口，支持插件上传、搜索、下载和版本管理。

## 架构设计

### 系统架构

```
┌─────────────────┐    HTTP/HTTPS    ┌──────────────────┐
│   GeekTools     │ ◄──────────────► │   Marketplace    │
│   Client        │                  │   API Server     │
└─────────────────┘                  └──────────────────┘
                                              │
                                              ▼
                    ┌──────────────────┬──────────────────┐
                    │                  │                  │
            ┌───────▼────────┐ ┌───────▼────────┐ ┌──────▼──────┐
            │   Database     │ │  File Storage  │ │   Cache     │
            │  (Metadata)    │ │ (Plugin Files) │ │  (Redis)    │
            └────────────────┘ └────────────────┘ └─────────────┘
```

### 技术栈建议

- **Web 框架**: Flask (Python) / Express.js (Node.js) / Axum (Rust)
- **数据库**: PostgreSQL / MySQL / SQLite
- **文件存储**: 本地文件系统 / AWS S3 / MinIO
- **缓存**: Redis
- **认证**: JWT / OAuth2
- **容器化**: Docker + Docker Compose

## 数据库设计

### 插件表 (plugins)

```sql
CREATE TABLE plugins (
    id VARCHAR(255) PRIMARY KEY,
    name VARCHAR(255) NOT NULL,
    description TEXT,
    author VARCHAR(255) NOT NULL,
    current_version VARCHAR(50) NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
    downloads INTEGER DEFAULT 0,
    rating DECIMAL(3,2) DEFAULT 0.00,
    status ENUM('active', 'deprecated', 'banned') DEFAULT 'active',
    min_geektools_version VARCHAR(50),
    homepage_url VARCHAR(500),
    repository_url VARCHAR(500),
    license VARCHAR(100)
);
```

### 插件版本表 (plugin_versions)

```sql
CREATE TABLE plugin_versions (
    id INTEGER PRIMARY KEY AUTO_INCREMENT,
    plugin_id VARCHAR(255) NOT NULL,
    version VARCHAR(50) NOT NULL,
    changelog TEXT,
    file_path VARCHAR(500) NOT NULL,
    file_size INTEGER NOT NULL,
    file_hash VARCHAR(64) NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    downloads INTEGER DEFAULT 0,
    is_stable BOOLEAN DEFAULT true,
    FOREIGN KEY (plugin_id) REFERENCES plugins(id) ON DELETE CASCADE,
    UNIQUE KEY unique_plugin_version (plugin_id, version)
);
```

### 插件脚本表 (plugin_scripts)

```sql
CREATE TABLE plugin_scripts (
    id INTEGER PRIMARY KEY AUTO_INCREMENT,
    plugin_id VARCHAR(255) NOT NULL,
    version VARCHAR(50) NOT NULL,
    script_name VARCHAR(255) NOT NULL,
    script_file VARCHAR(255) NOT NULL,
    description TEXT,
    is_executable BOOLEAN DEFAULT false,
    FOREIGN KEY (plugin_id) REFERENCES plugins(id) ON DELETE CASCADE
);
```

### 插件依赖表 (plugin_dependencies)

```sql
CREATE TABLE plugin_dependencies (
    id INTEGER PRIMARY KEY AUTO_INCREMENT,
    plugin_id VARCHAR(255) NOT NULL,
    dependency_id VARCHAR(255) NOT NULL,
    min_version VARCHAR(50),
    FOREIGN KEY (plugin_id) REFERENCES plugins(id) ON DELETE CASCADE,
    FOREIGN KEY (dependency_id) REFERENCES plugins(id) ON DELETE CASCADE
);
```

### 插件标签表 (plugin_tags)

```sql
CREATE TABLE plugin_tags (
    id INTEGER PRIMARY KEY AUTO_INCREMENT,
    plugin_id VARCHAR(255) NOT NULL,
    tag VARCHAR(100) NOT NULL,
    FOREIGN KEY (plugin_id) REFERENCES plugins(id) ON DELETE CASCADE,
    UNIQUE KEY unique_plugin_tag (plugin_id, tag)
);
```

### 用户表 (users)

```sql
CREATE TABLE users (
    id INTEGER PRIMARY KEY AUTO_INCREMENT,
    username VARCHAR(100) UNIQUE NOT NULL,
    email VARCHAR(255) UNIQUE NOT NULL,
    password_hash VARCHAR(255) NOT NULL,
    display_name VARCHAR(255),
    is_active BOOLEAN DEFAULT true,
    is_verified BOOLEAN DEFAULT false,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP
);
```

### 插件评分表 (plugin_ratings)

```sql
CREATE TABLE plugin_ratings (
    id INTEGER PRIMARY KEY AUTO_INCREMENT,
    plugin_id VARCHAR(255) NOT NULL,
    user_id INTEGER NOT NULL,
    rating INTEGER CHECK (rating >= 1 AND rating <= 5),
    review TEXT,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
    FOREIGN KEY (plugin_id) REFERENCES plugins(id) ON DELETE CASCADE,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
    UNIQUE KEY unique_user_plugin_rating (user_id, plugin_id)
);
```

## API 设计

### 基础信息

- **Base URL**: `https://api.geektools.dev/v1`
- **认证方式**: Bearer Token (JWT)
- **数据格式**: JSON
- **字符编码**: UTF-8

### 认证相关 API

#### 用户注册
```http
POST /auth/register
Content-Type: application/json

{
  "username": "developer123",
  "email": "dev@example.com",
  "password": "secure_password",
  "display_name": "Developer Name"
}
```

**Response (201 Created):**
```json
{
  "success": true,
  "message": "用户注册成功",
  "data": {
    "user_id": 123,
    "username": "developer123",
    "email": "dev@example.com",
    "display_name": "Developer Name"
  }
}
```

#### 用户登录
```http
POST /auth/login
Content-Type: application/json

{
  "username": "developer123",
  "password": "secure_password"
}
```

**Response (200 OK):**
```json
{
  "success": true,
  "data": {
    "access_token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...",
    "refresh_token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...",
    "expires_in": 86400,
    "user": {
      "id": 123,
      "username": "developer123",
      "display_name": "Developer Name"
    }
  }
}
```

### 插件相关 API

#### 获取插件列表
```http
GET /plugins?page=1&limit=20&search=system&tag=tools&sort=downloads
```

**Query Parameters:**
- `page` (int): 页码，默认 1
- `limit` (int): 每页数量，默认 20，最大 100
- `search` (string): 搜索关键词
- `tag` (string): 标签过滤
- `sort` (string): 排序方式 (`downloads`, `rating`, `updated_at`, `name`)
- `order` (string): 排序顺序 (`asc`, `desc`)

**Response (200 OK):**
```json
{
  "success": true,
  "data": {
    "plugins": [
      {
        "id": "system_tools",
        "name": "系统工具集",
        "description": "包含常用系统管理和监控工具的插件包",
        "author": "GeekTools Team",
        "current_version": "1.0.0",
        "downloads": 1250,
        "rating": 4.5,
        "tags": ["system", "monitoring", "tools"],
        "created_at": "2024-01-15T10:30:00Z",
        "updated_at": "2024-01-20T14:45:00Z"
      }
    ],
    "pagination": {
      "page": 1,
      "limit": 20,
      "total": 156,
      "pages": 8
    }
  }
}
```

#### 获取插件详细信息
```http
GET /plugins/{plugin_id}
```

**Response (200 OK):**
```json
{
  "success": true,
  "data": {
    "id": "system_tools",
    "name": "系统工具集",
    "description": "包含常用系统管理和监控工具的插件包",
    "author": "GeekTools Team",
    "current_version": "1.0.0",
    "downloads": 1250,
    "rating": 4.5,
    "tags": ["system", "monitoring", "tools"],
    "min_geektools_version": "0.5.0",
    "homepage_url": "https://github.com/geektools/system-tools-plugin",
    "repository_url": "https://github.com/geektools/system-tools-plugin",
    "license": "MIT",
    "versions": [
      {
        "version": "1.0.0",
        "changelog": "初始版本发布",
        "file_size": 15420,
        "created_at": "2024-01-15T10:30:00Z",
        "downloads": 1250,
        "is_stable": true
      }
    ],
    "scripts": [
      {
        "name": "系统信息",
        "file": "system_info.sh",
        "description": "显示详细的系统信息",
        "executable": true
      }
    ],
    "dependencies": [],
    "created_at": "2024-01-15T10:30:00Z",
    "updated_at": "2024-01-20T14:45:00Z"
  }
}
```

#### 上传插件
```http
POST /plugins
Authorization: Bearer {access_token}
Content-Type: multipart/form-data

{
  plugin_file: <binary_file_data>
}
```

**Response (201 Created):**
```json
{
  "success": true,
  "message": "插件上传成功",
  "data": {
    "plugin_id": "new_plugin",
    "version": "1.0.0",
    "upload_id": "upload_12345"
  }
}
```

#### 下载插件
```http
GET /plugins/{plugin_id}/download?version=1.0.0
```

**Response (302 Redirect):**
```http
Location: https://cdn.geektools.dev/plugins/system_tools/1.0.0/system_tools.tar.gz
```

### 搜索 API

#### 高级搜索
```http
POST /search
Content-Type: application/json

{
  "query": "system monitoring",
  "filters": {
    "tags": ["system", "monitoring"],
    "author": "GeekTools Team",
    "min_rating": 4.0,
    "min_geektools_version": "0.5.0"
  },
  "sort": {
    "field": "downloads",
    "order": "desc"
  },
  "pagination": {
    "page": 1,
    "limit": 20
  }
}
```

#### 搜索建议
```http
GET /search/suggestions?q=sys
```

**Response (200 OK):**
```json
{
  "success": true,
  "data": {
    "suggestions": [
      "system",
      "system tools",
      "system monitoring",
      "syslog"
    ]
  }
}
```

### 统计 API

#### 插件统计
```http
GET /plugins/{plugin_id}/stats
```

**Response (200 OK):**
```json
{
  "success": true,
  "data": {
    "total_downloads": 1250,
    "weekly_downloads": 85,
    "monthly_downloads": 340,
    "version_distribution": {
      "1.0.0": 1100,
      "0.9.0": 150
    },
    "download_trend": [
      {
        "date": "2024-01-15",
        "downloads": 45
      }
    ]
  }
}
```

## 服务端实现示例

### Python (Flask) 实现

#### 项目结构
```
marketplace-server/
├── app/
│   ├── __init__.py
│   ├── models/
│   │   ├── __init__.py
│   │   ├── plugin.py
│   │   ├── user.py
│   │   └── rating.py
│   ├── api/
│   │   ├── __init__.py
│   │   ├── auth.py
│   │   ├── plugins.py
│   │   └── search.py
│   ├── services/
│   │   ├── __init__.py
│   │   ├── plugin_service.py
│   │   ├── auth_service.py
│   │   └── file_service.py
│   └── utils/
│       ├── __init__.py
│       ├── validation.py
│       └── security.py
├── migrations/
├── tests/
├── config.py
├── requirements.txt
├── docker-compose.yml
└── Dockerfile
```

#### 主应用文件 (app/__init__.py)
```python
from flask import Flask
from flask_sqlalchemy import SQLAlchemy
from flask_migrate import Migrate
from flask_jwt_extended import JWTManager
from flask_cors import CORS
from config import Config

db = SQLAlchemy()
migrate = Migrate()
jwt = JWTManager()

def create_app(config_class=Config):
    app = Flask(__name__)
    app.config.from_object(config_class)
    
    # 初始化扩展
    db.init_app(app)
    migrate.init_app(app, db)
    jwt.init_app(app)
    CORS(app)
    
    # 注册蓝图
    from app.api.auth import bp as auth_bp
    from app.api.plugins import bp as plugins_bp
    from app.api.search import bp as search_bp
    
    app.register_blueprint(auth_bp, url_prefix='/api/v1/auth')
    app.register_blueprint(plugins_bp, url_prefix='/api/v1/plugins')
    app.register_blueprint(search_bp, url_prefix='/api/v1/search')
    
    return app
```

#### 插件模型 (app/models/plugin.py)
```python
from app import db
from datetime import datetime
from sqlalchemy.ext.hybrid import hybrid_property

class Plugin(db.Model):
    __tablename__ = 'plugins'
    
    id = db.Column(db.String(255), primary_key=True)
    name = db.Column(db.String(255), nullable=False)
    description = db.Column(db.Text)
    author = db.Column(db.String(255), nullable=False)
    current_version = db.Column(db.String(50), nullable=False)
    created_at = db.Column(db.DateTime, default=datetime.utcnow)
    updated_at = db.Column(db.DateTime, default=datetime.utcnow, onupdate=datetime.utcnow)
    downloads = db.Column(db.Integer, default=0)
    rating = db.Column(db.Numeric(3, 2), default=0.00)
    status = db.Column(db.Enum('active', 'deprecated', 'banned'), default='active')
    min_geektools_version = db.Column(db.String(50))
    homepage_url = db.Column(db.String(500))
    repository_url = db.Column(db.String(500))
    license = db.Column(db.String(100))
    
    # 关系
    versions = db.relationship('PluginVersion', backref='plugin', lazy='dynamic')
    scripts = db.relationship('PluginScript', backref='plugin', lazy='dynamic')
    tags = db.relationship('PluginTag', backref='plugin', lazy='dynamic')
    ratings = db.relationship('PluginRating', backref='plugin', lazy='dynamic')
    
    def to_dict(self):
        return {
            'id': self.id,
            'name': self.name,
            'description': self.description,
            'author': self.author,
            'current_version': self.current_version,
            'downloads': self.downloads,
            'rating': float(self.rating) if self.rating else 0.0,
            'tags': [tag.tag for tag in self.tags],
            'created_at': self.created_at.isoformat() + 'Z',
            'updated_at': self.updated_at.isoformat() + 'Z'
        }

class PluginVersion(db.Model):
    __tablename__ = 'plugin_versions'
    
    id = db.Column(db.Integer, primary_key=True)
    plugin_id = db.Column(db.String(255), db.ForeignKey('plugins.id'), nullable=False)
    version = db.Column(db.String(50), nullable=False)
    changelog = db.Column(db.Text)
    file_path = db.Column(db.String(500), nullable=False)
    file_size = db.Column(db.Integer, nullable=False)
    file_hash = db.Column(db.String(64), nullable=False)
    created_at = db.Column(db.DateTime, default=datetime.utcnow)
    downloads = db.Column(db.Integer, default=0)
    is_stable = db.Column(db.Boolean, default=True)
    
    __table_args__ = (db.UniqueConstraint('plugin_id', 'version'),)

class PluginScript(db.Model):
    __tablename__ = 'plugin_scripts'
    
    id = db.Column(db.Integer, primary_key=True)
    plugin_id = db.Column(db.String(255), db.ForeignKey('plugins.id'), nullable=False)
    version = db.Column(db.String(50), nullable=False)
    script_name = db.Column(db.String(255), nullable=False)
    script_file = db.Column(db.String(255), nullable=False)
    description = db.Column(db.Text)
    is_executable = db.Column(db.Boolean, default=False)

class PluginTag(db.Model):
    __tablename__ = 'plugin_tags'
    
    id = db.Column(db.Integer, primary_key=True)
    plugin_id = db.Column(db.String(255), db.ForeignKey('plugins.id'), nullable=False)
    tag = db.Column(db.String(100), nullable=False)
    
    __table_args__ = (db.UniqueConstraint('plugin_id', 'tag'),)
```

#### 插件服务 (app/services/plugin_service.py)
```python
import json
import tarfile
import tempfile
import hashlib
import os
from app import db
from app.models.plugin import Plugin, PluginVersion, PluginScript, PluginTag
from app.utils.validation import validate_plugin_package
from app.services.file_service import FileService

class PluginService:
    def __init__(self):
        self.file_service = FileService()
    
    def upload_plugin(self, file_data, user_id):
        """上传并处理插件包"""
        with tempfile.NamedTemporaryFile(delete=False) as tmp_file:
            file_data.save(tmp_file.name)
            
            try:
                # 解压并验证插件包
                plugin_info = self._extract_and_validate(tmp_file.name)
                
                # 检查插件是否已存在
                existing_plugin = Plugin.query.get(plugin_info['id'])
                if existing_plugin:
                    # 检查版本是否已存在
                    existing_version = PluginVersion.query.filter_by(
                        plugin_id=plugin_info['id'],
                        version=plugin_info['version']
                    ).first()
                    
                    if existing_version:
                        raise ValueError(f"版本 {plugin_info['version']} 已存在")
                
                # 计算文件哈希
                file_hash = self._calculate_file_hash(tmp_file.name)
                
                # 存储文件
                file_path = self.file_service.store_plugin_file(
                    tmp_file.name, plugin_info['id'], plugin_info['version']
                )
                
                # 保存到数据库
                plugin = self._save_plugin_to_db(plugin_info, file_path, file_hash, user_id)
                
                return plugin.to_dict()
                
            finally:
                os.unlink(tmp_file.name)
    
    def _extract_and_validate(self, file_path):
        """解压并验证插件包"""
        with tempfile.TemporaryDirectory() as temp_dir:
            # 解压文件
            with tarfile.open(file_path, 'r:gz') as tar:
                tar.extractall(temp_dir)
            
            # 读取 info.json
            info_path = os.path.join(temp_dir, 'info.json')
            if not os.path.exists(info_path):
                raise ValueError("插件包缺少 info.json 文件")
            
            with open(info_path, 'r', encoding='utf-8') as f:
                plugin_info = json.load(f)
            
            # 验证插件信息
            validate_plugin_package(plugin_info, temp_dir)
            
            return plugin_info
    
    def _calculate_file_hash(self, file_path):
        """计算文件 SHA256 哈希"""
        hash_sha256 = hashlib.sha256()
        with open(file_path, "rb") as f:
            for chunk in iter(lambda: f.read(4096), b""):
                hash_sha256.update(chunk)
        return hash_sha256.hexdigest()
    
    def _save_plugin_to_db(self, plugin_info, file_path, file_hash, user_id):
        """保存插件信息到数据库"""
        # 创建或更新插件记录
        plugin = Plugin.query.get(plugin_info['id'])
        if not plugin:
            plugin = Plugin(
                id=plugin_info['id'],
                name=plugin_info['name'],
                description=plugin_info['description'],
                author=plugin_info['author'],
                current_version=plugin_info['version'],
                min_geektools_version=plugin_info.get('min_geektools_version'),
                homepage_url=plugin_info.get('homepage_url'),
                repository_url=plugin_info.get('repository_url'),
                license=plugin_info.get('license', 'Unknown')
            )
            db.session.add(plugin)
        else:
            plugin.current_version = plugin_info['version']
            plugin.updated_at = datetime.utcnow()
        
        # 创建版本记录
        version = PluginVersion(
            plugin_id=plugin_info['id'],
            version=plugin_info['version'],
            changelog=plugin_info.get('changelog', ''),
            file_path=file_path,
            file_size=os.path.getsize(file_path),
            file_hash=file_hash,
            is_stable=True
        )
        db.session.add(version)
        
        # 保存脚本信息
        for script_info in plugin_info.get('scripts', []):
            script = PluginScript(
                plugin_id=plugin_info['id'],
                version=plugin_info['version'],
                script_name=script_info['name'],
                script_file=script_info['file'],
                description=script_info['description'],
                is_executable=script_info.get('executable', False)
            )
            db.session.add(script)
        
        # 保存标签
        # 首先删除旧标签
        PluginTag.query.filter_by(plugin_id=plugin_info['id']).delete()
        
        for tag_name in plugin_info.get('tags', []):
            tag = PluginTag(
                plugin_id=plugin_info['id'],
                tag=tag_name
            )
            db.session.add(tag)
        
        db.session.commit()
        return plugin
    
    def search_plugins(self, query=None, tags=None, page=1, limit=20, sort='downloads'):
        """搜索插件"""
        query_obj = Plugin.query.filter_by(status='active')
        
        if query:
            query_obj = query_obj.filter(
                db.or_(
                    Plugin.name.contains(query),
                    Plugin.description.contains(query)
                )
            )
        
        if tags:
            # 通过标签过滤
            tag_list = tags if isinstance(tags, list) else [tags]
            query_obj = query_obj.join(PluginTag).filter(
                PluginTag.tag.in_(tag_list)
            )
        
        # 排序
        if sort == 'downloads':
            query_obj = query_obj.order_by(Plugin.downloads.desc())
        elif sort == 'rating':
            query_obj = query_obj.order_by(Plugin.rating.desc())
        elif sort == 'updated_at':
            query_obj = query_obj.order_by(Plugin.updated_at.desc())
        elif sort == 'name':
            query_obj = query_obj.order_by(Plugin.name.asc())
        
        # 分页
        plugins = query_obj.paginate(
            page=page, per_page=limit, error_out=False
        )
        
        return {
            'plugins': [plugin.to_dict() for plugin in plugins.items],
            'pagination': {
                'page': page,
                'limit': limit,
                'total': plugins.total,
                'pages': plugins.pages
            }
        }
```

#### API 路由 (app/api/plugins.py)
```python
from flask import Blueprint, request, jsonify, send_file
from flask_jwt_extended import jwt_required, get_jwt_identity
from app.services.plugin_service import PluginService
from app.models.plugin import Plugin, PluginVersion
from app.utils.validation import validate_json

bp = Blueprint('plugins', __name__)
plugin_service = PluginService()

@bp.route('/', methods=['GET'])
def get_plugins():
    """获取插件列表"""
    page = request.args.get('page', 1, type=int)
    limit = min(request.args.get('limit', 20, type=int), 100)
    search = request.args.get('search')
    tag = request.args.get('tag')
    sort = request.args.get('sort', 'downloads')
    
    try:
        result = plugin_service.search_plugins(
            query=search,
            tags=tag,
            page=page,
            limit=limit,
            sort=sort
        )
        
        return jsonify({
            'success': True,
            'data': result
        })
        
    except Exception as e:
        return jsonify({
            'success': False,
            'error': str(e)
        }), 500

@bp.route('/<plugin_id>', methods=['GET'])
def get_plugin(plugin_id):
    """获取插件详细信息"""
    plugin = Plugin.query.get(plugin_id)
    if not plugin:
        return jsonify({
            'success': False,
            'error': '插件不存在'
        }), 404
    
    # 构建详细信息
    plugin_data = plugin.to_dict()
    
    # 添加版本信息
    plugin_data['versions'] = [
        {
            'version': v.version,
            'changelog': v.changelog,
            'file_size': v.file_size,
            'created_at': v.created_at.isoformat() + 'Z',
            'downloads': v.downloads,
            'is_stable': v.is_stable
        }
        for v in plugin.versions.order_by(PluginVersion.created_at.desc())
    ]
    
    # 添加脚本信息
    latest_scripts = plugin.scripts.filter_by(version=plugin.current_version)
    plugin_data['scripts'] = [
        {
            'name': s.script_name,
            'file': s.script_file,
            'description': s.description,
            'executable': s.is_executable
        }
        for s in latest_scripts
    ]
    
    return jsonify({
        'success': True,
        'data': plugin_data
    })

@bp.route('/', methods=['POST'])
@jwt_required()
def upload_plugin():
    """上传插件"""
    if 'plugin_file' not in request.files:
        return jsonify({
            'success': False,
            'error': '未提供插件文件'
        }), 400
    
    file = request.files['plugin_file']
    if file.filename == '':
        return jsonify({
            'success': False,
            'error': '未选择文件'
        }), 400
    
    if not file.filename.endswith('.tar.gz'):
        return jsonify({
            'success': False,
            'error': '文件格式错误，请上传 .tar.gz 文件'
        }), 400
    
    try:
        user_id = get_jwt_identity()
        result = plugin_service.upload_plugin(file, user_id)
        
        return jsonify({
            'success': True,
            'message': '插件上传成功',
            'data': result
        }), 201
        
    except ValueError as e:
        return jsonify({
            'success': False,
            'error': str(e)
        }), 400
    except Exception as e:
        return jsonify({
            'success': False,
            'error': '上传失败: ' + str(e)
        }), 500

@bp.route('/<plugin_id>/download', methods=['GET'])
def download_plugin(plugin_id):
    """下载插件"""
    version = request.args.get('version')
    
    # 查找插件版本
    query = PluginVersion.query.filter_by(plugin_id=plugin_id)
    if version:
        query = query.filter_by(version=version)
    else:
        # 如果未指定版本，获取最新版本
        plugin = Plugin.query.get(plugin_id)
        if not plugin:
            return jsonify({
                'success': False,
                'error': '插件不存在'
            }), 404
        query = query.filter_by(version=plugin.current_version)
    
    plugin_version = query.first()
    if not plugin_version:
        return jsonify({
            'success': False,
            'error': '插件版本不存在'
        }), 404
    
    # 增加下载计数
    plugin_version.downloads += 1
    plugin_version.plugin.downloads += 1
    db.session.commit()
    
    # 返回文件
    try:
        return send_file(
            plugin_version.file_path,
            as_attachment=True,
            download_name=f"{plugin_id}-{plugin_version.version}.tar.gz"
        )
    except FileNotFoundError:
        return jsonify({
            'success': False,
            'error': '文件不存在'
        }), 404
```

### Docker 部署配置

#### Dockerfile
```dockerfile
FROM python:3.11-slim

WORKDIR /app

# 安装系统依赖
RUN apt-get update && apt-get install -y \
    gcc \
    && rm -rf /var/lib/apt/lists/*

# 复制并安装 Python 依赖
COPY requirements.txt .
RUN pip install --no-cache-dir -r requirements.txt

# 复制应用代码
COPY . .

# 创建非 root 用户
RUN adduser --disabled-password --gecos '' appuser
RUN chown -R appuser:appuser /app
USER appuser

# 暴露端口
EXPOSE 5000

# 启动命令
CMD ["gunicorn", "--bind", "0.0.0.0:5000", "--workers", "4", "app:create_app()"]
```

#### docker-compose.yml
```yaml
version: '3.8'

services:
  web:
    build: .
    ports:
      - "5000:5000"
    environment:
      - FLASK_ENV=production
      - DATABASE_URL=postgresql://user:password@db:5432/marketplace
      - REDIS_URL=redis://redis:6379/0
      - JWT_SECRET_KEY=your-secret-key
      - UPLOAD_FOLDER=/app/uploads
    volumes:
      - ./uploads:/app/uploads
    depends_on:
      - db
      - redis
    restart: unless-stopped

  db:
    image: postgres:15
    environment:
      - POSTGRES_DB=marketplace
      - POSTGRES_USER=user
      - POSTGRES_PASSWORD=password
    volumes:
      - postgres_data:/var/lib/postgresql/data
    restart: unless-stopped

  redis:
    image: redis:7-alpine
    restart: unless-stopped

  nginx:
    image: nginx:alpine
    ports:
      - "80:80"
      - "443:443"
    volumes:
      - ./nginx.conf:/etc/nginx/nginx.conf
      - ./ssl:/etc/nginx/ssl
    depends_on:
      - web
    restart: unless-stopped

volumes:
  postgres_data:
```

#### nginx.conf
```nginx
events {
    worker_connections 1024;
}

http {
    upstream app {
        server web:5000;
    }

    # HTTP to HTTPS redirect
    server {
        listen 80;
        server_name api.geektools.dev;
        return 301 https://$server_name$request_uri;
    }

    # HTTPS server
    server {
        listen 443 ssl http2;
        server_name api.geektools.dev;

        ssl_certificate /etc/nginx/ssl/cert.pem;
        ssl_certificate_key /etc/nginx/ssl/key.pem;

        # Security headers
        add_header X-Frame-Options DENY;
        add_header X-Content-Type-Options nosniff;
        add_header X-XSS-Protection "1; mode=block";

        # Gzip compression
        gzip on;
        gzip_types text/plain application/json application/javascript text/css;

        # Rate limiting
        limit_req_zone $binary_remote_addr zone=api:10m rate=10r/s;

        location / {
            limit_req zone=api burst=20 nodelay;
            
            proxy_pass http://app;
            proxy_set_header Host $host;
            proxy_set_header X-Real-IP $remote_addr;
            proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
            proxy_set_header X-Forwarded-Proto $scheme;
            
            # Increase timeout for large file uploads
            proxy_read_timeout 300s;
            proxy_send_timeout 300s;
        }

        # Large file upload support
        client_max_body_size 100M;
    }
}
```

## 安全考虑

### 1. 文件上传安全

```python
import magic

def validate_uploaded_file(file_path):
    """验证上传的文件"""
    # 检查文件类型
    file_type = magic.from_file(file_path, mime=True)
    if file_type != 'application/gzip':
        raise ValueError("无效的文件类型")
    
    # 检查文件大小
    file_size = os.path.getsize(file_path)
    if file_size > 100 * 1024 * 1024:  # 100MB
        raise ValueError("文件过大")
    
    # 病毒扫描 (可选)
    if scan_for_malware(file_path):
        raise ValueError("检测到恶意内容")
```

### 2. 输入验证

```python
from cerberus import Validator

plugin_schema = {
    'id': {
        'type': 'string',
        'required': True,
        'regex': '^[a-z0-9_-]+$',
        'minlength': 3,
        'maxlength': 50
    },
    'name': {
        'type': 'string',
        'required': True,
        'minlength': 1,
        'maxlength': 255
    },
    'version': {
        'type': 'string',
        'required': True,
        'regex': r'^\d+\.\d+\.\d+(-[a-zA-Z0-9]+)?$'
    },
    'scripts': {
        'type': 'list',
        'required': True,
        'minlength': 1,
        'schema': {
            'type': 'dict',
            'schema': {
                'name': {'type': 'string', 'required': True},
                'file': {'type': 'string', 'required': True},
                'description': {'type': 'string', 'required': True}
            }
        }
    }
}

def validate_plugin_info(plugin_info):
    """验证插件信息"""
    validator = Validator(plugin_schema)
    if not validator.validate(plugin_info):
        raise ValueError(f"插件信息验证失败: {validator.errors}")
```

### 3. 访问控制

```python
from functools import wraps
from flask_jwt_extended import get_jwt_identity

def require_role(role):
    """角色权限装饰器"""
    def decorator(f):
        @wraps(f)
        @jwt_required()
        def decorated_function(*args, **kwargs):
            user_id = get_jwt_identity()
            user = User.query.get(user_id)
            
            if not user or user.role != role:
                return jsonify({
                    'success': False,
                    'error': '权限不足'
                }), 403
            
            return f(*args, **kwargs)
        return decorated_function
    return decorator

@bp.route('/admin/plugins', methods=['DELETE'])
@require_role('admin')
def delete_plugin():
    """删除插件 (仅管理员)"""
    pass
```

## 监控和日志

### 1. 日志配置

```python
import logging
from logging.handlers import RotatingFileHandler

def setup_logging(app):
    """配置日志"""
    if not app.debug:
        # 文件日志
        file_handler = RotatingFileHandler(
            'marketplace.log',
            maxBytes=10240000,  # 10MB
            backupCount=10
        )
        file_handler.setFormatter(logging.Formatter(
            '%(asctime)s %(levelname)s: %(message)s [in %(pathname)s:%(lineno)d]'
        ))
        file_handler.setLevel(logging.INFO)
        app.logger.addHandler(file_handler)
        
        app.logger.setLevel(logging.INFO)
        app.logger.info('Marketplace startup')
```

### 2. 指标收集

```python
from prometheus_client import Counter, Histogram, generate_latest

# 定义指标
upload_counter = Counter('plugin_uploads_total', 'Total plugin uploads')
download_counter = Counter('plugin_downloads_total', 'Total plugin downloads', ['plugin_id'])
request_duration = Histogram('request_duration_seconds', 'Request duration')

@bp.route('/metrics')
def metrics():
    """Prometheus 指标端点"""
    return generate_latest()

@bp.before_request
def before_request():
    g.start_time = time.time()

@bp.after_request  
def after_request(response):
    request_duration.observe(time.time() - g.start_time)
    return response
```

## 性能优化

### 1. 缓存策略

```python
from flask_caching import Cache

cache = Cache()

@bp.route('/plugins')
@cache.cached(timeout=300, query_string=True)
def get_plugins():
    """缓存插件列表"""
    pass

def invalidate_plugin_cache(plugin_id):
    """清除插件相关缓存"""
    cache.delete(f'plugin:{plugin_id}')
    cache.delete_memoized(get_plugins)
```

### 2. 数据库优化

```python
# 添加索引
class Plugin(db.Model):
    # ...
    __table_args__ = (
        db.Index('idx_plugin_status_downloads', 'status', 'downloads'),
        db.Index('idx_plugin_created_at', 'created_at'),
        db.Index('idx_plugin_rating', 'rating'),
    )

# 查询优化
def get_popular_plugins(limit=10):
    """获取热门插件"""
    return Plugin.query\
        .filter_by(status='active')\
        .order_by(Plugin.downloads.desc())\
        .limit(limit)\
        .all()
```

### 3. CDN 集成

```python
class FileService:
    def __init__(self):
        self.use_cdn = current_app.config.get('USE_CDN', False)
        self.cdn_base_url = current_app.config.get('CDN_BASE_URL')
    
    def get_download_url(self, plugin_id, version):
        """获取下载URL"""
        if self.use_cdn:
            return f"{self.cdn_base_url}/plugins/{plugin_id}/{version}/{plugin_id}.tar.gz"
        else:
            return url_for('plugins.download_plugin', plugin_id=plugin_id, version=version)
```

## 测试

### 单元测试示例

```python
import unittest
from app import create_app, db
from app.models.plugin import Plugin
from config import TestConfig

class PluginTestCase(unittest.TestCase):
    def setUp(self):
        self.app = create_app(TestConfig)
        self.app_context = self.app.app_context()
        self.app_context.push()
        db.create_all()
        self.client = self.app.test_client()
    
    def tearDown(self):
        db.session.remove()
        db.drop_all()
        self.app_context.pop()
    
    def test_upload_plugin(self):
        """测试插件上传"""
        # 创建测试用户并获取 token
        token = self.get_auth_token()
        
        # 准备测试文件
        with open('test_plugin.tar.gz', 'rb') as f:
            response = self.client.post(
                '/api/v1/plugins',
                data={'plugin_file': (f, 'test_plugin.tar.gz')},
                headers={'Authorization': f'Bearer {token}'},
                content_type='multipart/form-data'
            )
        
        self.assertEqual(response.status_code, 201)
        data = response.get_json()
        self.assertTrue(data['success'])
    
    def test_search_plugins(self):
        """测试插件搜索"""
        # 创建测试数据
        plugin = Plugin(
            id='test_plugin',
            name='Test Plugin',
            description='A test plugin',
            author='Test Author',
            current_version='1.0.0'
        )
        db.session.add(plugin)
        db.session.commit()
        
        # 测试搜索
        response = self.client.get('/api/v1/plugins?search=test')
        self.assertEqual(response.status_code, 200)
        
        data = response.get_json()
        self.assertTrue(data['success'])
        self.assertEqual(len(data['data']['plugins']), 1)
```

## 部署和运维

### 1. 部署脚本

```bash
#!/bin/bash
# deploy.sh

set -e

echo "开始部署插件市场服务..."

# 拉取最新代码
git pull origin main

# 构建镜像
docker-compose build

# 备份数据库
docker-compose exec db pg_dump -U user marketplace > backup_$(date +%Y%m%d_%H%M%S).sql

# 更新服务
docker-compose up -d

# 运行数据库迁移
docker-compose exec web flask db upgrade

# 健康检查
sleep 10
curl -f http://localhost:5000/health || exit 1

echo "部署完成！"
```

### 2. 监控脚本

```bash
#!/bin/bash
# monitor.sh

# 检查服务状态
check_service() {
    local service=$1
    if ! docker-compose ps $service | grep -q "Up"; then
        echo "❌ $service 服务异常"
        # 发送告警...
        return 1
    fi
    echo "✅ $service 服务正常"
    return 0
}

# 检查磁盘空间
check_disk_space() {
    local usage=$(df / | awk 'NR==2 {print $5}' | sed 's/%//')
    if [ $usage -gt 80 ]; then
        echo "⚠️  磁盘使用率过高: ${usage}%"
        # 清理旧文件...
    fi
}

echo "=== 服务监控 ==="
check_service web
check_service db
check_service redis

echo "=== 资源监控 ==="
check_disk_space

echo "=== API 健康检查 ==="
if curl -f http://localhost:5000/health >/dev/null 2>&1; then
    echo "✅ API 服务正常"
else
    echo "❌ API 服务异常"
fi
```

## 总结

本文档提供了构建 GeekTools 插件市场服务端的完整指南，包括：

1. **系统架构设计** - 可扩展的微服务架构
2. **数据库设计** - 完整的数据模型和关系
3. **API 设计** - RESTful API 接口规范
4. **安全考虑** - 文件上传、输入验证、访问控制
5. **性能优化** - 缓存、数据库优化、CDN 集成
6. **部署运维** - Docker 容器化部署和监控

通过遵循这些最佳实践，您可以构建一个安全、高效、可扩展的插件市场服务，为 GeekTools 用户提供优质的插件生态系统。